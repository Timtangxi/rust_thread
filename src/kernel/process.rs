#![allow(dead_code)]

use core::mem;

use crate::fs::vfs::InodeRef;
use crate::kernel::address::{PhysAddr, USER_TOP, VirtAddr};
use crate::kernel::memory::PAGE_SIZE;

pub const MAX_PROCESSES: usize = 16;
pub const MAX_FILES: usize = 8;
pub const MAX_ADDRESS_SPACE_L2: usize = 16;
pub const MAX_ADDRESS_SPACE_REGIONS: usize = 16;
pub const USER_STACK_PAGES: usize = 4;
pub const USER_STACK_SIZE: usize = USER_STACK_PAGES * PAGE_SIZE;
pub const USER_STACK_SLOT_SIZE: usize = 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ProcessId(usize);

impl ProcessId {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }

    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ThreadId(usize);

impl ThreadId {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThreadMode {
    Kernel,
    User,
}

impl ThreadMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::User => "user",
        }
    }

    pub const fn is_user(self) -> bool {
        matches!(self, Self::User)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Empty,
    Alive,
    Zombie,
}

impl ProcessState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Alive => "alive",
            Self::Zombie => "zombie",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AddressSpaceKind {
    Kernel,
    User,
}

impl AddressSpaceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::User => "user",
        }
    }

    pub const fn is_user(self) -> bool {
        matches!(self, Self::User)
    }
}

#[derive(Clone, Copy)]
pub struct AddressSpace {
    pub kind: AddressSpaceKind,
    pub page_table_root: PhysAddr,
    pub asid: u16,
    pub l1_pages: usize,
    pub owned_l2: [PhysAddr; MAX_ADDRESS_SPACE_L2],
    pub owned_l2_count: usize,
    pub owned_regions: [UserRegion; MAX_ADDRESS_SPACE_REGIONS],
    pub owned_region_count: usize,
    pub entry: VirtAddr,
    pub user_start: VirtAddr,
    pub user_end: VirtAddr,
    pub stack_bottom: VirtAddr,
    pub stack_top: VirtAddr,
}

impl AddressSpace {
    pub const fn kernel(page_table_root: PhysAddr) -> Self {
        Self {
            kind: AddressSpaceKind::Kernel,
            page_table_root,
            asid: 0,
            l1_pages: 0,
            owned_l2: [PhysAddr::new(0); MAX_ADDRESS_SPACE_L2],
            owned_l2_count: 0,
            owned_regions: [const { UserRegion::empty() }; MAX_ADDRESS_SPACE_REGIONS],
            owned_region_count: 0,
            entry: VirtAddr::new(0),
            user_start: VirtAddr::new(0),
            user_end: VirtAddr::new(0),
            stack_bottom: VirtAddr::new(0),
            stack_top: VirtAddr::new(0),
        }
    }

    pub const fn user(
        page_table_root: PhysAddr,
        asid: u16,
        l1_pages: usize,
        entry: VirtAddr,
        user_start: VirtAddr,
        user_end: VirtAddr,
        stack_bottom: VirtAddr,
        stack_top: VirtAddr,
    ) -> Self {
        Self {
            kind: AddressSpaceKind::User,
            page_table_root,
            asid,
            l1_pages,
            owned_l2: [PhysAddr::new(0); MAX_ADDRESS_SPACE_L2],
            owned_l2_count: 0,
            owned_regions: [const { UserRegion::empty() }; MAX_ADDRESS_SPACE_REGIONS],
            owned_region_count: 0,
            entry,
            user_start,
            user_end,
            stack_bottom,
            stack_top,
        }
    }

    pub fn add_owned_l2(&mut self, table: PhysAddr) {
        self.try_add_owned_l2(table)
            .unwrap_or_else(|_| panic!("address space l2 ownership table full"));
    }

    pub fn try_add_owned_l2(&mut self, table: PhysAddr) -> Result<(), u32> {
        if table.as_usize() == 0 {
            return Ok(());
        }

        if self.owned_l2_count >= MAX_ADDRESS_SPACE_L2 {
            return Err(crate::kernel::syscall::ENOMEM);
        }

        self.owned_l2[self.owned_l2_count] = table;
        self.owned_l2_count += 1;
        Ok(())
    }

    pub fn add_owned_region(&mut self, virt: VirtAddr, phys: PhysAddr, pages: usize) {
        self.try_add_owned_region(virt, phys, pages)
            .unwrap_or_else(|_| panic!("address space region ownership table full"));
    }

    pub fn try_add_owned_region(
        &mut self,
        virt: VirtAddr,
        phys: PhysAddr,
        pages: usize,
    ) -> Result<(), u32> {
        if pages == 0 {
            return Ok(());
        }

        if self.owned_region_count >= MAX_ADDRESS_SPACE_REGIONS {
            return Err(crate::kernel::syscall::ENOMEM);
        }

        self.owned_regions[self.owned_region_count] = UserRegion { virt, phys, pages };
        self.owned_region_count += 1;
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileObject {
    Closed,
    ConsoleIn,
    ConsoleOut,
    ConsoleErr,
    Regular { inode: InodeRef },
}

impl FileObject {
    pub const fn can_read(self) -> bool {
        matches!(self, Self::ConsoleIn | Self::Regular { .. })
    }

    pub const fn can_write(self) -> bool {
        matches!(
            self,
            Self::ConsoleOut | Self::ConsoleErr | Self::Regular { .. }
        )
    }
}

#[derive(Clone, Copy)]
pub struct UserRegion {
    pub virt: VirtAddr,
    pub phys: PhysAddr,
    pub pages: usize,
}

impl UserRegion {
    pub const fn empty() -> Self {
        Self {
            virt: VirtAddr::new(0),
            phys: PhysAddr::new(0),
            pages: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct FileDescriptor {
    pub object: FileObject,
    pub offset: usize,
    pub ref_count: u16,
    pub flags: u32,
}

impl FileDescriptor {
    pub const fn closed() -> Self {
        Self {
            object: FileObject::Closed,
            offset: 0,
            ref_count: 0,
            flags: 0,
        }
    }

    pub const fn new(object: FileObject, flags: u32) -> Self {
        Self {
            object,
            offset: 0,
            ref_count: 1,
            flags,
        }
    }

    pub const fn is_open(self) -> bool {
        !matches!(self.object, FileObject::Closed)
    }
}

#[derive(Clone, Copy)]
pub struct FileTable {
    entries: [FileDescriptor; MAX_FILES],
}

impl FileTable {
    pub const fn empty() -> Self {
        Self {
            entries: [FileDescriptor::closed(); MAX_FILES],
        }
    }

    pub const fn with_stdio() -> Self {
        let mut entries = [FileDescriptor::closed(); MAX_FILES];
        entries[0] = FileDescriptor::new(FileObject::ConsoleIn, 0);
        entries[1] = FileDescriptor::new(FileObject::ConsoleOut, 0);
        entries[2] = FileDescriptor::new(FileObject::ConsoleErr, 0);
        Self { entries }
    }

    pub fn get(&self, fd: usize) -> Option<FileDescriptor> {
        self.entries.get(fd).copied()
    }

    pub fn get_mut(&mut self, fd: usize) -> Option<&mut FileDescriptor> {
        self.entries.get_mut(fd).filter(|file| file.is_open())
    }

    pub fn open(&mut self, object: FileObject, flags: u32) -> Result<usize, u32> {
        let Some(fd) = self.entries.iter().position(|file| !file.is_open()) else {
            return Err(crate::kernel::syscall::EMFILE);
        };

        self.entries[fd] = FileDescriptor::new(object, flags);
        Ok(fd)
    }

    pub fn close(&mut self, fd: usize) -> Result<(), u32> {
        let Some(file) = self.entries.get_mut(fd) else {
            return Err(crate::kernel::syscall::EBADF);
        };

        if !file.is_open() {
            return Err(crate::kernel::syscall::EBADF);
        }

        if file.ref_count > 1 {
            file.ref_count -= 1;
        } else {
            *file = FileDescriptor::closed();
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct ProcessControlBlock {
    pub pid: ProcessId,
    pub parent: ProcessId,
    pub name: &'static str,
    pub state: ProcessState,
    pub address_space: AddressSpace,
    pub files: FileTable,
    pub main_thread: ThreadId,
    pub thread_count: usize,
    pub exit_code: i32,
    pub resources_reaped: bool,
}

impl ProcessControlBlock {
    pub const fn empty() -> Self {
        Self {
            pid: ProcessId::new(0),
            parent: ProcessId::new(0),
            name: "",
            state: ProcessState::Empty,
            address_space: AddressSpace::kernel(PhysAddr::new(0)),
            files: FileTable::empty(),
            main_thread: ThreadId::new(0),
            thread_count: 0,
            exit_code: 0,
            resources_reaped: true,
        }
    }

    pub fn new(
        pid: ProcessId,
        parent: ProcessId,
        name: &'static str,
        address_space: AddressSpace,
    ) -> Self {
        Self {
            pid,
            parent,
            name,
            state: ProcessState::Alive,
            address_space,
            files: FileTable::with_stdio(),
            main_thread: ThreadId::new(0),
            thread_count: 1,
            exit_code: 0,
            resources_reaped: false,
        }
    }
}

pub struct ProcessTable {
    entries: [ProcessControlBlock; MAX_PROCESSES],
    next_pid: usize,
}

impl ProcessTable {
    pub const fn new() -> Self {
        Self {
            entries: [const { ProcessControlBlock::empty() }; MAX_PROCESSES],
            next_pid: 1,
        }
    }

    pub fn create(
        &mut self,
        name: &'static str,
        parent: ProcessId,
        address_space: AddressSpace,
    ) -> ProcessId {
        self.try_create(name, parent, address_space)
            .unwrap_or_else(|_| panic!("process table full"))
    }

    pub fn try_create(
        &mut self,
        name: &'static str,
        parent: ProcessId,
        address_space: AddressSpace,
    ) -> Result<ProcessId, u32> {
        let Some(slot) = self
            .entries
            .iter()
            .position(|process| process.state == ProcessState::Empty)
        else {
            return Err(crate::kernel::syscall::ENOMEM);
        };

        let pid = ProcessId::new(self.next_pid);
        self.next_pid = self.next_pid.wrapping_add(1).max(1);
        self.entries[slot] = ProcessControlBlock::new(pid, parent, name, address_space);
        Ok(pid)
    }

    pub fn has_free_slot(&self) -> bool {
        self.entries
            .iter()
            .any(|process| process.state == ProcessState::Empty)
    }

    pub fn rename(&mut self, pid: ProcessId, name: &'static str) {
        if let Some(process) = self.get_mut(pid) {
            process.name = name;
        }
    }

    pub fn set_main_thread(&mut self, pid: ProcessId, tid: ThreadId) {
        if let Some(process) = self.get_mut(pid) {
            process.main_thread = tid;
        }
    }

    pub fn mark_zombie(&mut self, pid: ProcessId, exit_code: i32) {
        if let Some(process) = self.get_mut(pid) {
            process.state = ProcessState::Zombie;
            process.exit_code = exit_code;
        }
    }

    pub fn mark_resources_reaped(
        &mut self,
        pid: ProcessId,
        replacement_address_space: AddressSpace,
    ) -> Option<AddressSpace> {
        let process = self.get_mut(pid)?;
        if process.resources_reaped {
            return None;
        }

        process.resources_reaped = true;
        process.thread_count = 0;
        process.files = FileTable::empty();
        Some(mem::replace(
            &mut process.address_space,
            replacement_address_space,
        ))
    }

    pub fn release(&mut self, pid: ProcessId) {
        if let Some(slot) = self.index_of(pid) {
            self.entries[slot] = ProcessControlBlock::empty();
            self.orphan_children(pid);
        }
    }

    pub fn address_space(&self, pid: ProcessId) -> Option<AddressSpace> {
        self.get(pid).map(|process| process.address_space)
    }

    pub fn address_space_mut(&mut self, pid: ProcessId) -> Option<&mut AddressSpace> {
        self.get_mut(pid).map(|process| &mut process.address_space)
    }

    pub fn replace_address_space(
        &mut self,
        pid: ProcessId,
        address_space: AddressSpace,
    ) -> Option<AddressSpace> {
        let process = self.get_mut(pid)?;
        let old = process.address_space;
        process.address_space = address_space;
        Some(old)
    }

    pub fn take_zombie_child(
        &mut self,
        parent: ProcessId,
        requested: ProcessId,
    ) -> Result<(ProcessId, i32), u32> {
        let mut has_child = false;

        for slot in 0..self.entries.len() {
            let process = self.entries[slot];
            if process.state == ProcessState::Empty || process.parent != parent {
                continue;
            }

            if requested.is_valid() && process.pid != requested {
                continue;
            }

            has_child = true;
            if process.state == ProcessState::Zombie && process.resources_reaped {
                let pid = process.pid;
                let exit_code = process.exit_code;
                self.release(pid);
                return Ok((pid, exit_code));
            }
        }

        if has_child {
            Err(crate::kernel::syscall::EAGAIN)
        } else {
            Err(crate::kernel::syscall::ECHILD)
        }
    }

    pub fn parent_of(&self, pid: ProcessId) -> ProcessId {
        self.get(pid)
            .map(|process| process.parent)
            .unwrap_or(ProcessId::new(0))
    }

    pub fn has_live_parent(&self, pid: ProcessId) -> bool {
        let parent = self.parent_of(pid);
        parent.is_valid()
            && self
                .get(parent)
                .map(|process| process.state == ProcessState::Alive)
                .unwrap_or(false)
    }

    pub fn can_write_fd(&self, pid: ProcessId, fd: usize) -> bool {
        self.get(pid)
            .and_then(|process| process.files.get(fd))
            .map(|file| file.object.can_write())
            .unwrap_or(false)
    }

    pub fn can_read_fd(&self, pid: ProcessId, fd: usize) -> bool {
        self.get(pid)
            .and_then(|process| process.files.get(fd))
            .map(|file| file.object.can_read())
            .unwrap_or(false)
    }

    pub fn file(&self, pid: ProcessId, fd: usize) -> Option<FileDescriptor> {
        self.get(pid).and_then(|process| process.files.get(fd))
    }

    pub fn file_mut(&mut self, pid: ProcessId, fd: usize) -> Option<&mut FileDescriptor> {
        self.get_mut(pid)
            .and_then(|process| process.files.get_mut(fd))
    }

    pub fn open_file(
        &mut self,
        pid: ProcessId,
        object: FileObject,
        flags: u32,
    ) -> Result<usize, u32> {
        self.get_mut(pid)
            .ok_or(crate::kernel::syscall::EINVAL)?
            .files
            .open(object, flags)
    }

    pub fn close_file(&mut self, pid: ProcessId, fd: usize) -> Result<(), u32> {
        self.get_mut(pid)
            .ok_or(crate::kernel::syscall::EINVAL)?
            .files
            .close(fd)
    }

    pub fn state_str(&self, pid: ProcessId) -> &'static str {
        self.get(pid)
            .map(|process| process.state.as_str())
            .unwrap_or("missing")
    }

    pub fn address_space_kind(&self, pid: ProcessId) -> &'static str {
        self.get(pid)
            .map(|process| process.address_space.kind.as_str())
            .unwrap_or("missing")
    }

    fn get(&self, pid: ProcessId) -> Option<&ProcessControlBlock> {
        self.index_of(pid).map(|slot| &self.entries[slot])
    }

    fn get_mut(&mut self, pid: ProcessId) -> Option<&mut ProcessControlBlock> {
        self.index_of(pid).map(|slot| &mut self.entries[slot])
    }

    fn index_of(&self, pid: ProcessId) -> Option<usize> {
        if !pid.is_valid() {
            return None;
        }

        self.entries
            .iter()
            .position(|process| process.pid == pid && process.state != ProcessState::Empty)
    }

    fn orphan_children(&mut self, parent: ProcessId) {
        for slot in 0..self.entries.len() {
            if self.entries[slot].state == ProcessState::Empty
                || self.entries[slot].parent != parent
            {
                continue;
            }

            if self.entries[slot].state == ProcessState::Zombie
                && self.entries[slot].resources_reaped
            {
                let child = self.entries[slot].pid;
                self.entries[slot] = ProcessControlBlock::empty();
                self.orphan_children(child);
            } else {
                self.entries[slot].parent = ProcessId::new(0);
            }
        }
    }
}

pub const fn user_stack_top(slot: usize) -> usize {
    USER_TOP - slot * USER_STACK_SLOT_SIZE
}

pub const fn user_stack_bottom(slot: usize) -> usize {
    user_stack_top(slot) - USER_STACK_SIZE
}
