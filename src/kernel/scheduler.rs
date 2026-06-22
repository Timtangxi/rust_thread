#[cfg(feature = "mmu")]
use crate::arch::aarch32::context::USER_INITIAL_CPSR;
use crate::arch::aarch32::context::{TaskContext, TaskEntry, TrapFrame};
use crate::drivers::device::IrqStatus;
use crate::drivers::{uart, virtio};
use crate::fs::vfs;
use crate::kernel::address::PhysAddr;
#[cfg(feature = "mmu")]
use crate::kernel::address::VirtAddr;
use crate::kernel::console;
use crate::kernel::ipc::{self, PipeEnd, PipeIo};
#[cfg(feature = "mmu")]
use crate::kernel::loader;
#[cfg(feature = "mmu")]
use crate::kernel::loader::UserImage;
use crate::kernel::log::{KernelLog, LogEvent};
use crate::kernel::memory;
#[cfg(feature = "mmu")]
use crate::kernel::process::USER_STACK_PAGES;
use crate::kernel::process::{AddressSpace, FileObject, ProcessId, ProcessTable, ThreadId};
#[cfg(feature = "mmu")]
use crate::kernel::process::{
    USER_MMAP_BASE, USER_MMAP_TOP, VM_ANON, VM_EXEC, VM_FILE, VM_FIXED, VM_HEAP, VM_MMAP, VM_READ,
    VM_USER, VM_WRITE, user_stack_bottom, user_stack_top,
};
use crate::kernel::queue::ReadyQueue;
use crate::kernel::sleep::SleepQueue;
use crate::kernel::syscall::{
    ARM_SYS_SET_TLS, AT_EMPTY_PATH, AT_FDCWD, AT_REMOVEDIR, AT_SYMLINK_FOLLOW, AT_SYMLINK_NOFOLLOW,
    CLOCK_MONOTONIC, CLOCK_REALTIME, DT_CHR, DT_DIR, DT_LNK, DT_REG, EACCES, EAFNOSUPPORT, EAGAIN,
    EBADF, EFAULT, EINVAL, EISDIR, ENOENT, ENOMEM, ENOSYS, ENOTTY, ERANGE, ESPIPE, ESRCH, F_DUPFD,
    F_DUPFD_CLOEXEC, F_GETFD, F_GETFL, F_OK, F_SETFD, F_SETFL, FD_CLOEXEC_FLAG, FUTEX_WAIT,
    FUTEX_WAKE, KERNEL_ABI_MAGIC, LINUX_SYS_ACCEPT, LINUX_SYS_ACCEPT4, LINUX_SYS_ACCESS,
    LINUX_SYS_BIND, LINUX_SYS_BRK, LINUX_SYS_CHDIR, LINUX_SYS_CLOCK_GETRES,
    LINUX_SYS_CLOCK_GETRES_TIME64, LINUX_SYS_CLOCK_GETTIME, LINUX_SYS_CLOCK_GETTIME64,
    LINUX_SYS_CLOCK_NANOSLEEP, LINUX_SYS_CLOCK_NANOSLEEP_TIME64, LINUX_SYS_CLONE, LINUX_SYS_CLOSE,
    LINUX_SYS_CONNECT, LINUX_SYS_CREAT, LINUX_SYS_DUP, LINUX_SYS_DUP2, LINUX_SYS_DUP3,
    LINUX_SYS_EXECVE, LINUX_SYS_EXIT, LINUX_SYS_EXIT_GROUP, LINUX_SYS_FACCESSAT,
    LINUX_SYS_FCHMODAT, LINUX_SYS_FCHOWNAT, LINUX_SYS_FCNTL, LINUX_SYS_FCNTL64,
    LINUX_SYS_FDATASYNC, LINUX_SYS_FLOCK, LINUX_SYS_FORK, LINUX_SYS_FSTAT, LINUX_SYS_FSTAT64,
    LINUX_SYS_FSTATAT64, LINUX_SYS_FSTATFS, LINUX_SYS_FSTATFS64, LINUX_SYS_FSYNC,
    LINUX_SYS_FTRUNCATE, LINUX_SYS_FUTEX, LINUX_SYS_GET_ROBUST_LIST, LINUX_SYS_GETCWD,
    LINUX_SYS_GETDENTS, LINUX_SYS_GETDENTS64, LINUX_SYS_GETEGID, LINUX_SYS_GETEGID32,
    LINUX_SYS_GETEUID, LINUX_SYS_GETEUID32, LINUX_SYS_GETGID, LINUX_SYS_GETGID32,
    LINUX_SYS_GETGROUPS, LINUX_SYS_GETGROUPS32, LINUX_SYS_GETPEERNAME, LINUX_SYS_GETPID,
    LINUX_SYS_GETPPID, LINUX_SYS_GETRANDOM, LINUX_SYS_GETRLIMIT, LINUX_SYS_GETRUSAGE,
    LINUX_SYS_GETSOCKNAME, LINUX_SYS_GETSOCKOPT, LINUX_SYS_GETTID, LINUX_SYS_GETTIMEOFDAY,
    LINUX_SYS_GETUID, LINUX_SYS_GETUID32, LINUX_SYS_IOCTL, LINUX_SYS_KILL, LINUX_SYS_LINK,
    LINUX_SYS_LINKAT, LINUX_SYS_LISTEN, LINUX_SYS_LLSEEK, LINUX_SYS_LSEEK, LINUX_SYS_LSTAT,
    LINUX_SYS_LSTAT64, LINUX_SYS_MADVISE, LINUX_SYS_MKDIR, LINUX_SYS_MKDIRAT, LINUX_SYS_MKNODAT,
    LINUX_SYS_MMAP, LINUX_SYS_MMAP2, LINUX_SYS_MPROTECT, LINUX_SYS_MSYNC, LINUX_SYS_MUNMAP,
    LINUX_SYS_NANOSLEEP, LINUX_SYS_NEWSELECT, LINUX_SYS_OPEN, LINUX_SYS_OPENAT, LINUX_SYS_PIPE,
    LINUX_SYS_PIPE2, LINUX_SYS_POLL, LINUX_SYS_PPOLL, LINUX_SYS_PRCTL, LINUX_SYS_PSELECT6,
    LINUX_SYS_READ, LINUX_SYS_READLINK, LINUX_SYS_READLINKAT, LINUX_SYS_READV, LINUX_SYS_RECV,
    LINUX_SYS_RECVFROM, LINUX_SYS_RECVMSG, LINUX_SYS_RENAME, LINUX_SYS_RENAMEAT,
    LINUX_SYS_RENAMEAT2, LINUX_SYS_RESTART_SYSCALL, LINUX_SYS_RMDIR, LINUX_SYS_RT_SIGACTION,
    LINUX_SYS_RT_SIGPROCMASK, LINUX_SYS_RT_SIGRETURN, LINUX_SYS_SCHED_YIELD, LINUX_SYS_SELECT,
    LINUX_SYS_SEND, LINUX_SYS_SENDMSG, LINUX_SYS_SENDTO, LINUX_SYS_SET_ROBUST_LIST,
    LINUX_SYS_SET_TID_ADDRESS, LINUX_SYS_SETGROUPS, LINUX_SYS_SETGROUPS32, LINUX_SYS_SETSOCKOPT,
    LINUX_SYS_SHUTDOWN, LINUX_SYS_SOCKET, LINUX_SYS_SOCKETPAIR, LINUX_SYS_STAT, LINUX_SYS_STAT64,
    LINUX_SYS_STATFS, LINUX_SYS_STATFS64, LINUX_SYS_STATX, LINUX_SYS_SYSINFO, LINUX_SYS_TGKILL,
    LINUX_SYS_TIMES, LINUX_SYS_TRUNCATE, LINUX_SYS_UGETRLIMIT, LINUX_SYS_UMASK, LINUX_SYS_UNAME,
    LINUX_SYS_UNLINK, LINUX_SYS_UNLINKAT, LINUX_SYS_VFORK, LINUX_SYS_WAIT4, LINUX_SYS_WAITPID,
    LINUX_SYS_WRITE, LINUX_SYS_WRITEV, O_CLOEXEC, O_DIRECTORY, O_NOFOLLOW, POLLERR, POLLHUP,
    POLLIN, POLLNVAL, POLLOUT, PollFd, R_OK, RLIMIT_NOFILE, RLIMIT_STACK, RLimit, SEEK_CUR,
    SEEK_END, SEEK_SET, SYS_ACCESS, SYS_BLOCK, SYS_BRK, SYS_CHDIR, SYS_CLOCK_GETTIME, SYS_CLOSE,
    SYS_DUP, SYS_DUP2, SYS_DUP3, SYS_EXEC, SYS_EXECVE, SYS_EXIT, SYS_FCNTL, SYS_FSTAT, SYS_GETCWD,
    SYS_GETDENTS, SYS_GETPID, SYS_GETPPID, SYS_GETTIMEOFDAY, SYS_IOCTL, SYS_LSEEK, SYS_MKDIR,
    SYS_MMAP, SYS_MPROTECT, SYS_MUNMAP, SYS_NEWFSTATAT, SYS_OPEN, SYS_PIPE, SYS_PIPE2, SYS_POLL,
    SYS_READ, SYS_READLINK, SYS_READV, SYS_RENAME, SYS_RT_SIGACTION, SYS_RT_SIGPROCMASK,
    SYS_SELECT, SYS_SLEEP, SYS_SPAWN, SYS_STAT, SYS_UNAME, SYS_UNLINK, SYS_WAIT, SYS_WAIT4,
    SYS_WAKE, SYS_WRITE, SYS_WRITEV, SYS_YIELD, StatFs64, SysInfo, TCGETS, TCSETS, TCSETSF,
    TCSETSW, TIOCGWINSZ, TimeSpec, TimeSpec64, TimeVal, Tms, UtsName, W_OK, WinSize, X_OK,
};
#[cfg(feature = "mmu")]
use crate::kernel::syscall::{MAP_ANONYMOUS, MAP_FIXED, PROT_EXEC, PROT_READ, PROT_WRITE};
use crate::kernel::syscall::{O_APPEND, O_CREAT, O_TRUNC, UserIovec};
use crate::kernel::task::{MAX_TASKS, TASK_STATES, Task, TaskControlBlock, TaskId, TaskState};
use crate::kernel::user::{UserPtr, copy_from_user, copy_to_user};
use crate::kernel::wait::WaitQueueTable;
use crate::{print, println};

unsafe extern "C" {
    fn context_restore(sp: *mut u32) -> !;
}

pub const DEFAULT_TIME_SLICE_TICKS: u32 = 1;

#[derive(Clone, Copy)]
enum ScheduleReason {
    Tick,
    Yield,
    Sleep,
    Block,
    Wake,
    Exit,
}

enum SyscallAction {
    Return(u32),
    Block { channel: u32 },
}

impl ScheduleReason {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Tick => "tick",
            Self::Yield => "yield",
            Self::Sleep => "sleep",
            Self::Block => "block",
            Self::Wake => "wake",
            Self::Exit => "exit",
        }
    }
}

const WAIT_CHANNEL_BASE: u32 = 0x8000_0000;
const CLONE_VM: u32 = 0x0000_0100;
const CLONE_THREAD: u32 = 0x0001_0000;
const CLONE_PARENT_SETTID: u32 = 0x0010_0000;
const CLONE_CHILD_SETTID: u32 = 0x0100_0000;
const SIGINT: i32 = 2;
const SIGTERM: i32 = 15;
const TICKS_PER_SECOND: u64 = 100;

fn child_wait_channel(parent: ProcessId) -> u32 {
    WAIT_CHANNEL_BASE | ((parent.as_usize() as u32) & 0x7fff_ffff)
}

fn kernel_wait_status(pid: ProcessId, exit_code: i32) -> u32 {
    ((pid.as_usize() as u32) << 8) | ((exit_code as u32) & 0xff)
}

fn linux_wait_status(exit_code: i32) -> u32 {
    if exit_code < 0 {
        (-exit_code as u32) & 0x7f
    } else {
        ((exit_code as u32) & 0xff) << 8
    }
}

fn timespec_to_ticks(sec: u64, nsec: u64) -> u64 {
    let sec_ticks = sec.saturating_mul(TICKS_PER_SECOND);
    let nsec_ticks = nsec
        .saturating_mul(TICKS_PER_SECOND)
        .saturating_add(999_999_999)
        / 1_000_000_000;
    sec_ticks.saturating_add(nsec_ticks)
}

fn fill_random(dst: &mut [u8]) {
    let mut state = (crate::drivers::timer::counter() as u32)
        ^ ((crate::drivers::timer::counter() >> 32) as u32)
        ^ memory::free_page_count() as u32
        ^ 0xa5a5_1f2d;
    for byte in dst {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        *byte = (state >> 24) as u8;
    }
}

fn init_name(path: &[u8]) -> &'static str {
    match path {
        b"/sbin/init" => "sbin-init",
        b"/bin/init" => "bin-init",
        b"/bin/sh" => "sh",
        b"/bin/busybox" => "busybox",
        _ => "init",
    }
}

fn exec_name(path: &[u8]) -> &'static str {
    match path {
        b"/bin/busybox" => "busybox",
        b"/bin/sh" => "sh",
        b"/bin/ash" => "ash",
        b"/bin/ls" => "ls",
        b"/bin/cat" => "cat",
        b"/bin/echo" => "echo",
        b"/sbin/init" => "sbin-init",
        b"/bin/init" => "bin-init",
        _ => "exec",
    }
}

fn is_error(value: u32) -> bool {
    value >= 0xffff_f000
}

fn write_nul_field(dst: &mut [u8; 65], src: &[u8]) {
    let len = src.len().min(dst.len() - 1);
    dst[..len].copy_from_slice(&src[..len]);
    dst[len] = 0;
}

fn align_dirent64_len(len: usize) -> usize {
    (len + 7) & !7
}

fn write_u16(dst: &mut [u8], value: u16) {
    let bytes = value.to_le_bytes();
    dst[..bytes.len()].copy_from_slice(&bytes);
}

fn write_u32(dst: &mut [u8], value: u32) {
    let bytes = value.to_le_bytes();
    dst[..bytes.len()].copy_from_slice(&bytes);
}

fn write_u64(dst: &mut [u8], value: u64) {
    let bytes = value.to_le_bytes();
    dst[..bytes.len()].copy_from_slice(&bytes);
}

fn write_i64(dst: &mut [u8], value: i64) {
    let bytes = value.to_le_bytes();
    dst[..bytes.len()].copy_from_slice(&bytes);
}

fn linux_dtype(file_type: vfs::FileType) -> u8 {
    match file_type {
        vfs::FileType::Regular => DT_REG,
        vfs::FileType::Directory => DT_DIR,
        vfs::FileType::Device => DT_CHR,
        vfs::FileType::Symlink => DT_LNK,
    }
}

fn linux_rdev(metadata: vfs::Metadata) -> u32 {
    if metadata.file_type == vfs::FileType::Device {
        metadata.inode.ino as u32
    } else {
        0
    }
}

fn normalize_user_path(path: &[u8], out: &mut [u8]) -> usize {
    if out.is_empty() {
        return 0;
    }
    let path = match path.iter().position(|byte| *byte == 0) {
        Some(end) => &path[..end],
        None => path,
    };
    let mut len = 0usize;
    if !path.starts_with(b"/") {
        out[0] = b'/';
        len = 1;
    }
    for byte in path {
        if len >= out.len() {
            break;
        }
        out[len] = *byte;
        len += 1;
    }
    if len == 0 {
        out[0] = b'/';
        len = 1;
    }
    len
}

fn trim_nul(path: &[u8]) -> &[u8] {
    match path.iter().position(|byte| *byte == 0) {
        Some(end) => &path[..end],
        None => path,
    }
}

#[cfg(feature = "mmu")]
fn flags_prot_to_vm(prot: u32) -> u32 {
    let mut flags = 0;
    if prot & PROT_READ != 0 {
        flags |= VM_READ;
    }
    if prot & PROT_WRITE != 0 {
        flags |= VM_WRITE;
    }
    if prot & PROT_EXEC != 0 {
        flags |= VM_EXEC;
    }
    flags
}

#[cfg(feature = "mmu")]
fn user_mapping_from_vm(flags: u32) -> crate::arch::aarch32::mmu::UserMapping {
    if flags & (VM_READ | VM_WRITE | VM_EXEC) == 0 {
        crate::arch::aarch32::mmu::UserMapping::NoAccess
    } else if flags & VM_EXEC != 0 {
        crate::arch::aarch32::mmu::UserMapping::Rx
    } else if flags & VM_WRITE != 0 {
        crate::arch::aarch32::mmu::UserMapping::RwData
    } else {
        crate::arch::aarch32::mmu::UserMapping::RoData
    }
}

fn kernel_address_space() -> AddressSpace {
    let root = {
        #[cfg(feature = "mmu")]
        {
            PhysAddr::new(crate::arch::aarch32::mmu::table_base())
        }
        #[cfg(not(feature = "mmu"))]
        {
            PhysAddr::new(0)
        }
    };
    AddressSpace::kernel(root)
}

pub struct Scheduler {
    tasks: [Task; MAX_TASKS],
    ready: ReadyQueue,
    sleep: SleepQueue,
    wait: WaitQueueTable,
    processes: ProcessTable,
    log: KernelLog,
    count: usize,
    current: Option<TaskId>,
    idle: Option<TaskId>,
    ticks: u64,
    switches: u64,
    next_tid: usize,
    #[cfg(feature = "mmu")]
    next_asid: u16,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            tasks: [const { TaskControlBlock::empty() }; MAX_TASKS],
            ready: ReadyQueue::new(),
            sleep: SleepQueue::new(),
            wait: WaitQueueTable::new(),
            processes: ProcessTable::new(),
            log: KernelLog::new(),
            count: 0,
            current: None,
            idle: None,
            ticks: 0,
            switches: 0,
            next_tid: 1,
            #[cfg(feature = "mmu")]
            next_asid: 1,
        }
    }

    pub fn spawn(&mut self, name: &'static str, entry: TaskEntry) -> TaskId {
        self.spawn_with_options(name, entry, 0, DEFAULT_TIME_SLICE_TICKS)
    }

    pub fn spawn_with_options(
        &mut self,
        name: &'static str,
        entry: TaskEntry,
        priority: u8,
        time_slice_ticks: u32,
    ) -> TaskId {
        let id = self.alloc_task_slot();
        let root = {
            #[cfg(feature = "mmu")]
            {
                PhysAddr::new(crate::arch::aarch32::mmu::table_base())
            }
            #[cfg(not(feature = "mmu"))]
            {
                PhysAddr::new(0)
            }
        };
        let process_id = self
            .processes
            .create(name, ProcessId::new(0), AddressSpace::kernel(root));
        let thread_id = self.alloc_thread_id();

        self.tasks[id.index()] = TaskControlBlock::new_kernel(
            id,
            process_id,
            thread_id,
            name,
            entry,
            priority,
            time_slice_ticks,
        );
        self.processes.set_main_thread(process_id, thread_id);
        if name == "idle" {
            self.idle = Some(id);
        } else {
            self.enqueue_ready(id);
        }
        self.count += 1;
        id
    }

    #[cfg(feature = "mmu")]
    pub fn spawn_user_init(
        &mut self,
        image: UserImage,
        priority: u8,
        time_slice_ticks: u32,
    ) -> TaskId {
        if crate::config::CONFIG_BUSYBOX_SMOKE {
            if let Ok(inode) = vfs::lookup(b"/bin/busybox") {
                let argv = [
                    loader::UserArg::from_bytes(b"busybox"),
                    loader::UserArg::from_bytes(b"--help"),
                ];
                if let Ok(task) = self.spawn_loaded_user_with_args(
                    ProcessId::new(0),
                    "busybox",
                    priority,
                    time_slice_ticks,
                    |address_space| unsafe {
                        loader::load_elf_from_inode_into(address_space, "busybox", inode)
                    },
                    &argv,
                    &[],
                ) {
                    self.processes.set_init_process(self.task(task).process_id);
                    return task;
                }
            }
        }

        let ext4_boot = crate::config::CONFIG_EXT4_ROOTFS;
        let candidates: [(&[u8], &[u8]); 4] = if ext4_boot {
            [
                (b"bin/busybox".as_slice(), b"/bin/busybox".as_slice()),
                (b"bin/sh".as_slice(), b"/bin/sh".as_slice()),
                (b"sbin/init".as_slice(), b"/sbin/init".as_slice()),
                (b"bin/init".as_slice(), b"/bin/init".as_slice()),
            ]
        } else {
            [
                (b"/sbin/init".as_slice(), b"/sbin/init".as_slice()),
                (b"/bin/init".as_slice(), b"/bin/init".as_slice()),
                (b"/bin/sh".as_slice(), b"/bin/sh".as_slice()),
                (b"/bin/busybox".as_slice(), b"/bin/busybox".as_slice()),
            ]
        };
        for (lookup_path, argv0_path) in candidates {
            match vfs::lookup(lookup_path) {
                Ok(inode) => {
                    let name = init_name(argv0_path);
                    let busybox_shell_argv = [
                        loader::UserArg::from_bytes(b"sh"),
                        loader::UserArg::from_bytes(b"-i"),
                    ];
                    let default_argv = [loader::UserArg::from_bytes(argv0_path)];
                    let argv = if ext4_boot && argv0_path == b"/bin/busybox" {
                        &busybox_shell_argv[..]
                    } else {
                        &default_argv[..]
                    };
                    match self.spawn_loaded_user_with_args(
                        ProcessId::new(0),
                        name,
                        priority,
                        time_slice_ticks,
                        |address_space| unsafe {
                            loader::load_elf_from_inode_into(address_space, name, inode)
                        },
                        argv,
                        &[],
                    ) {
                        Ok(task) => {
                            self.processes.set_init_process(self.task(task).process_id);
                            return task;
                        }
                        Err(err) => {
                            if crate::config::CONFIG_BOOT_VERBOSE {
                                println!(
                                    "init: candidate {} failed err={}",
                                    core::str::from_utf8(argv0_path).unwrap_or("<?>"),
                                    err as i32
                                );
                            }
                        }
                    }
                }
                Err(err) => {
                    if crate::config::CONFIG_BOOT_VERBOSE {
                        println!(
                            "init: candidate {} lookup failed err={} ext4={} ext4_lookup={}",
                            core::str::from_utf8(argv0_path).unwrap_or("<?>"),
                            err as i32,
                            vfs::ext4_mounted(),
                            vfs::ext4_lookup_error() as i32
                        );
                    }
                }
            }
        }

        let builtin = self
            .spawn_loaded_user(
                ProcessId::new(0),
                "builtin-init",
                priority.saturating_add(1),
                time_slice_ticks,
                |address_space| unsafe { loader::load_builtin_into(address_space, image) },
            )
            .unwrap_or_else(|err| panic!("failed to spawn builtin user shell: {}", err as i32));
        self.processes
            .set_init_process(self.task(builtin).process_id);
        builtin
    }

    #[cfg(feature = "mmu")]
    fn spawn_loaded_user<F>(
        &mut self,
        parent: ProcessId,
        name: &'static str,
        priority: u8,
        time_slice_ticks: u32,
        load: F,
    ) -> Result<TaskId, u32>
    where
        F: FnOnce(&mut AddressSpace) -> Result<loader::LoadedImage, u32>,
    {
        self.spawn_loaded_user_with_args(parent, name, priority, time_slice_ticks, load, &[], &[])
    }

    #[cfg(feature = "mmu")]
    fn spawn_loaded_user_with_args<F>(
        &mut self,
        parent: ProcessId,
        name: &'static str,
        priority: u8,
        time_slice_ticks: u32,
        load: F,
        argv: &[loader::UserArg],
        envp: &[loader::UserArg],
    ) -> Result<TaskId, u32>
    where
        F: FnOnce(&mut AddressSpace) -> Result<loader::LoadedImage, u32>,
    {
        let id = self.try_alloc_task_slot()?;
        let stack_slot = id.index() + 1;
        let argv = if argv.is_empty() { None } else { Some(argv) };
        let envp = if envp.is_empty() { None } else { Some(envp) };
        let (address_space, loaded, user_stack, initial_sp) =
            self.build_user_address_space_with_args(stack_slot, load, argv, envp)?;

        let process_id = match self.processes.try_create(name, parent, address_space) {
            Ok(pid) => pid,
            Err(err) => {
                loader::release_address_space_regions(address_space);
                return Err(err);
            }
        };
        let thread_id = self.alloc_thread_id();

        self.tasks[id.index()] = TaskControlBlock::new_user(
            id,
            process_id,
            thread_id,
            name,
            loaded.entry.as_usize(),
            user_stack,
            USER_STACK_PAGES,
            address_space.stack_bottom.as_usize(),
            address_space.stack_top.as_usize(),
            initial_sp,
            priority,
            time_slice_ticks,
        );
        self.processes.set_main_thread(process_id, thread_id);
        self.enqueue_ready(id);
        self.count += 1;
        Ok(id)
    }

    pub unsafe fn start(&mut self) -> ! {
        let next = self
            .pick_next()
            .unwrap_or_else(|| panic!("no runnable tasks"));
        self.current = Some(next);

        let tick = self.ticks;
        {
            let task = self.task_mut(next);
            task.mark_scheduled(tick);
        }
        self.switch_address_space(next);

        if crate::config::CONFIG_BOOT_VERBOSE {
            let task = self.task(next);
            println!(
                "scheduler: start task {} (pid={}, state={}, prio={}, slice={} tick)",
                task.name,
                task.pid,
                task.state.as_str(),
                task.priority,
                task.time_slice_ticks
            );
        }

        unsafe { context_restore(self.task(next).context.sp) }
    }

    pub unsafe fn tick(&mut self, saved_sp: *mut u32) -> *mut u32 {
        self.ticks = self.ticks.wrapping_add(1);
        self.wake_sleepers();
        self.reap_zombies();
        self.poll_console_input();

        let Some(current) = self.current else {
            return saved_sp;
        };
        if self.task(current).state != TaskState::Running {
            return self.schedule_from_current(current, ScheduleReason::Exit);
        }

        {
            let current_task = self.task_mut(current);
            current_task.context = TaskContext { sp: saved_sp };
            current_task.account_tick();
        }
        if self.task(current).remaining_ticks > 0 {
            return saved_sp;
        }

        self.schedule_from_current(current, ScheduleReason::Tick)
    }

    fn poll_console_input(&mut self) {
        let bytes = uart::poll_rx(|byte| {
            console::push_input(byte);
        });
        if bytes != 0 {
            self.wake_console_readers();
            self.log.push(LogEvent::ConsoleInput {
                tick: self.ticks,
                bytes,
                available: console::available(),
                dropped: console::dropped(),
            });
        }
        self.handle_console_interrupts();
    }

    fn wait_poll_console_input(&mut self) {
        for _ in 0..4096 {
            self.poll_console_input();
            if console::available() != 0 || !uart::rx_ready() {
                break;
            }
        }
    }

    pub unsafe fn device_irq(&mut self, irq: u32, saved_sp: *mut u32) -> *mut u32 {
        let Some(current) = self.current else {
            return saved_sp;
        };

        self.task_mut(current).context = TaskContext { sp: saved_sp };

        if irq == uart::irq() {
            let bytes = uart::handle_irq(|byte| {
                console::push_input(byte);
            });
            if bytes != 0 {
                self.wake_console_readers();
                self.log.push(LogEvent::ConsoleInput {
                    tick: self.ticks,
                    bytes,
                    available: console::available(),
                    dropped: console::dropped(),
                });
            }
            self.handle_console_interrupts();
        } else {
            match virtio::handle_irq(irq) {
                IrqStatus::Wake { channel } => {
                    self.wake_channel(channel);
                }
                IrqStatus::Handled | IrqStatus::Unhandled => {}
            }
        }

        if self.task(current).state != TaskState::Running {
            return self.schedule_from_current(current, ScheduleReason::Wake);
        }

        if Some(current) == self.idle && !self.ready.is_empty() {
            return self.schedule_from_current(current, ScheduleReason::Wake);
        }

        if self.task(current).remaining_ticks > 0 {
            return saved_sp;
        }

        self.schedule_from_current(current, ScheduleReason::Tick)
    }

    pub unsafe fn syscall(&mut self, saved_sp: *mut u32) -> *mut u32 {
        let Some(current) = self.current else {
            return saved_sp;
        };

        let frame = unsafe { TrapFrame::from_saved_sp(saved_sp) };
        if frame.kernel_abi_magic() != KERNEL_ABI_MAGIC
            && !(frame.kernel_abi_magic() == 0 && frame.syscall_number() <= SYS_RT_SIGPROCMASK)
        {
            return self.dispatch_linux_syscall(current, frame, saved_sp);
        }

        let syscall = frame.syscall_number();
        let arg0 = frame.syscall_arg0();
        let arg1 = frame.syscall_arg1();
        let arg2 = frame.syscall_arg2();
        let arg3 = frame.syscall_arg3();
        let arg4 = frame.syscall_arg4();
        let arg5 = frame.syscall_arg5();

        {
            let task = self.task_mut(current);
            task.context = TaskContext { sp: saved_sp };
            task.stats.last_syscall = syscall;
        }

        match syscall {
            SYS_YIELD => {
                frame.set_return_value(0);
                self.schedule_from_current(current, ScheduleReason::Yield)
            }
            SYS_SLEEP => {
                let wake_at = self.ticks.wrapping_add(u64::from(arg0.max(1)));
                self.task_mut(current).mark_sleeping(wake_at);
                self.sleep.insert(current, wake_at);
                frame.set_return_value(0);
                self.schedule_from_current(current, ScheduleReason::Sleep)
            }
            SYS_BLOCK => {
                self.task_mut(current).mark_blocked(arg0);
                self.wait.sleep_on(arg0, current);
                frame.set_return_value(0);
                self.schedule_from_current(current, ScheduleReason::Block)
            }
            SYS_WAKE => {
                let count = self.wake_channel(arg0);
                frame.set_return_value(count as u32);
                saved_sp
            }
            SYS_WRITE => {
                let result = self.sys_write(current, arg0, arg1, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_READ => match self.sys_read(current, arg0, arg1, arg2) {
                SyscallAction::Return(result) => {
                    frame.set_return_value(result);
                    saved_sp
                }
                SyscallAction::Block { channel } => {
                    frame.set_return_value(EAGAIN);
                    self.wait.sleep_on(channel, current);
                    self.schedule_from_current(current, ScheduleReason::Block)
                }
            },
            SYS_READV => match self.sys_readv(current, arg0, arg1, arg2) {
                SyscallAction::Return(result) => {
                    frame.set_return_value(result);
                    saved_sp
                }
                SyscallAction::Block { channel } => {
                    frame.set_return_value(EAGAIN);
                    self.wait.sleep_on(channel, current);
                    self.schedule_from_current(current, ScheduleReason::Block)
                }
            },
            SYS_WRITEV => {
                let result = self.sys_writev(current, arg0, arg1, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_OPEN => {
                let result = self.sys_open(current, arg0, arg1, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_CLOSE => {
                let result = self.sys_close(current, arg0);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_LSEEK => {
                let result = self.sys_lseek(current, arg0, arg1 as i32, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_STAT => {
                let result = self.sys_stat(current, arg0, arg1, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_FSTAT => {
                let result = self.sys_fstat_kernel(current, arg0, arg1);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_NEWFSTATAT => {
                let result = self.sys_newfstatat_kernel(current, arg0 as i32, arg1, arg2, arg3);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_ACCESS => {
                let result = self.sys_access(current, arg0, arg1, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_FCNTL => {
                let result = self.sys_fcntl(current, arg0, arg1, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_IOCTL => {
                let result = self.sys_ioctl(current, arg0, arg1, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_GETDENTS => {
                let result = self.sys_getdents(current, arg0, arg1, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_MKDIR => {
                let result = self.sys_mkdir(current, arg0, arg1, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_UNLINK => {
                let result = self.sys_unlink(current, arg0, arg1);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_RENAME => {
                let result = self.sys_rename(current, arg0, arg1);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_PIPE => {
                let result = self.sys_pipe(current, arg0);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_BRK => {
                let result = self.sys_brk(current, arg0 as usize);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_MMAP => {
                let result = self.sys_mmap(
                    current,
                    arg0 as usize,
                    arg1 as usize,
                    arg2,
                    arg3,
                    arg4 as i32,
                    arg5 as usize,
                );
                frame.set_return_value(result);
                saved_sp
            }
            SYS_MUNMAP => {
                let result = self.sys_munmap(current, arg0 as usize, arg1 as usize);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_MPROTECT => {
                let result = self.sys_mprotect(current, arg0 as usize, arg1 as usize, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_GETPID => {
                frame.set_return_value(self.task(current).process_id.as_usize() as u32);
                saved_sp
            }
            SYS_GETPPID => {
                let ppid = self.processes.parent_of(self.task(current).process_id);
                frame.set_return_value(ppid.as_usize() as u32);
                saved_sp
            }
            SYS_UNAME => {
                let result = self.sys_uname(arg0);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_GETTIMEOFDAY => {
                let result = self.sys_gettimeofday(arg0, arg1);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_CLOCK_GETTIME => {
                let result = self.sys_clock_gettime(arg0, arg1);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_EXECVE => {
                let result = self.sys_execve(current, arg0, arg1, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_WAIT4 => match self.sys_wait4(current, arg0 as i32, arg1, arg2, arg3) {
                Ok(result) => {
                    frame.set_return_value(result);
                    saved_sp
                }
                Err(EAGAIN) => {
                    frame.set_return_value(EAGAIN);
                    let channel = child_wait_channel(self.task(current).process_id);
                    let wait_pid = if (arg0 as i32) > 0 { arg0 as usize } else { 0 };
                    let task = self.task_mut(current);
                    task.mark_waiting_for_child(channel, wait_pid, arg1 as usize, arg3 as usize);
                    self.wait.sleep_on(channel, current);
                    self.schedule_from_current(current, ScheduleReason::Block)
                }
                Err(err) => {
                    frame.set_return_value(err);
                    saved_sp
                }
            },
            SYS_DUP => {
                let result = self.sys_dup(current, arg0, 0, false);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_DUP2 => {
                let result = self.sys_dup2(current, arg0, arg1, false);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_DUP3 => {
                let result = self.sys_dup3(current, arg0, arg1, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_PIPE2 => {
                let result = self.sys_pipe2(current, arg0, arg1);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_GETCWD => {
                let result = self.sys_getcwd(current, arg0, arg1);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_CHDIR => {
                let result = self.sys_chdir(current, arg0, arg1);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_READLINK => {
                let result = self.sys_readlink(current, arg0, arg1, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_POLL => {
                let result = self.sys_poll(current, arg0, arg1, arg2 as i32);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_SELECT => {
                let result = self.sys_select(current, arg0, arg1, arg2, arg3, arg4);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_RT_SIGACTION | SYS_RT_SIGPROCMASK => {
                frame.set_return_value(0);
                saved_sp
            }
            SYS_WAIT => match self.try_wait(current, ProcessId::new(arg0 as usize)) {
                Ok(result) => {
                    frame.set_return_value(result);
                    saved_sp
                }
                Err(EAGAIN) => {
                    frame.set_return_value(EAGAIN);
                    let channel = child_wait_channel(self.task(current).process_id);
                    let task = self.task_mut(current);
                    task.mark_waiting_for_child(channel, arg0 as usize, 0, 0);
                    self.wait.sleep_on(channel, current);
                    self.schedule_from_current(current, ScheduleReason::Block)
                }
                Err(err) => {
                    frame.set_return_value(err);
                    saved_sp
                }
            },
            SYS_SPAWN => {
                let result = self.sys_spawn(current, arg0, arg1, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_EXEC => {
                let result = self.sys_exec(current, arg0, arg1);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_EXIT => {
                self.exit_task(current, arg0 as i32);
                frame.set_return_value(0);
                self.schedule_from_current(current, ScheduleReason::Exit)
            }
            _ => {
                frame.set_return_value(EINVAL);
                saved_sp
            }
        }
    }

    pub fn wake_channel(&mut self, channel: u32) -> usize {
        let mut woken_tasks = [None; MAX_TASKS];
        let mut woken_len = 0;
        let count = self.wait.wake_all(channel, |task| {
            woken_tasks[woken_len] = Some(task);
            woken_len += 1;
        });

        for item in woken_tasks.iter().take(woken_len) {
            if let Some(task) = *item {
                if self.task(task).state == TaskState::Blocked {
                    let t = self.task_mut(task);
                    t.state = TaskState::Ready;
                    t.wait_channel = 0;
                    t.wait_pid = 0;
                    t.wait_user_buf = 0;
                    t.wait_len = 0;
                    self.enqueue_ready(task);
                }
            }
        }

        if count != 0 {
            self.log.push(LogEvent::Wake {
                tick: self.ticks,
                channel,
                count,
            });
        }
        count
    }

    fn handle_console_interrupts(&mut self) {
        let interrupts = console::take_interrupts();
        if interrupts == 0 {
            return;
        }

        let target = self
            .current
            .and_then(|current| self.foreground_interrupt_target(current));
        if let Some(task) = target {
            self.interrupt_task(task, SIGINT);
        }
    }

    fn foreground_interrupt_target(&self, current: TaskId) -> Option<TaskId> {
        for idx in 0..MAX_TASKS {
            let waiter = &self.tasks[idx];
            if waiter.state != TaskState::Blocked {
                continue;
            }
            let channel = child_wait_channel(waiter.process_id);
            if waiter.wait_channel != channel {
                continue;
            }
            if let Some(child) = self.find_child_task(waiter.process_id, waiter.wait_pid) {
                return Some(child);
            }
        }

        let _ = current;
        None
    }

    fn find_child_task(&self, parent: ProcessId, requested_pid: usize) -> Option<TaskId> {
        for task in &self.tasks {
            if task.state == TaskState::Empty || task.state == TaskState::Zombie {
                continue;
            }
            if !task.mode.is_user() {
                continue;
            }
            if requested_pid != 0 && task.process_id.as_usize() != requested_pid {
                continue;
            }
            if self.processes.parent_of(task.process_id) == parent {
                return Some(task.id);
            }
        }
        None
    }

    fn interrupt_task(&mut self, task_id: TaskId, signal: i32) {
        if !self.task(task_id).mode.is_user() || self.task(task_id).state == TaskState::Zombie {
            return;
        }
        self.wait.remove_task(task_id);
        self.sleep.remove_task(task_id);
        self.exit_task(task_id, -signal);
    }

    fn exit_task(&mut self, task_id: TaskId, exit_code: i32) {
        if self.task(task_id).state == TaskState::Zombie {
            return;
        }
        let tick = self.ticks;
        let (name, pid, process_id) = {
            let task = self.task_mut(task_id);
            task.mark_zombie(exit_code);
            (task.name, task.pid, task.process_id)
        };
        self.processes.mark_zombie(process_id, exit_code);
        self.processes.orphan_children_of(process_id);
        self.log.push(LogEvent::TaskExit { tick, name, pid });
    }

    pub fn flush_logs(&mut self) {
        self.log.flush();
    }

    fn dispatch_linux_syscall(
        &mut self,
        current: TaskId,
        frame: &mut TrapFrame,
        saved_sp: *mut u32,
    ) -> *mut u32 {
        let nr = frame.linux_syscall_number();
        let a0 = frame.linux_arg0();
        let a1 = frame.linux_arg1();
        let a2 = frame.linux_arg2();
        let a3 = frame.linux_arg3();
        let a4 = frame.linux_arg4();
        let a5 = frame.linux_arg5();
        let a6 = frame.linux_arg6();

        {
            let task = self.task_mut(current);
            task.context = TaskContext { sp: saved_sp };
            task.stats.last_syscall = nr;
        }

        match nr {
            LINUX_SYS_EXIT | LINUX_SYS_EXIT_GROUP => {
                self.exit_task(current, a0 as i32);
                frame.set_return_value(0);
                self.schedule_from_current(current, ScheduleReason::Exit)
            }
            LINUX_SYS_READ => {
                let action = self.sys_read(current, a0, a1, a2);
                self.finish_read_syscall(current, frame, action, saved_sp)
            }
            LINUX_SYS_WRITE => {
                frame.set_return_value(self.sys_write(current, a0, a1, a2));
                saved_sp
            }
            LINUX_SYS_OPEN => {
                frame.set_return_value(self.sys_open_cstr(current, a0, a1));
                saved_sp
            }
            LINUX_SYS_OPENAT => {
                frame.set_return_value(self.sys_openat_cstr(current, a0 as i32, a1, a2));
                saved_sp
            }
            LINUX_SYS_CLOSE => {
                frame.set_return_value(self.sys_close(current, a0));
                saved_sp
            }
            LINUX_SYS_LSEEK => {
                frame.set_return_value(self.sys_lseek(current, a0, a1 as i32, a2));
                saved_sp
            }
            LINUX_SYS_READV => {
                let action = self.sys_readv(current, a0, a1, a2);
                self.finish_read_syscall(current, frame, action, saved_sp)
            }
            LINUX_SYS_WRITEV => {
                frame.set_return_value(self.sys_writev(current, a0, a1, a2));
                saved_sp
            }
            LINUX_SYS_RESTART_SYSCALL | LINUX_SYS_SCHED_YIELD => {
                frame.set_return_value(0);
                self.schedule_from_current(current, ScheduleReason::Yield)
            }
            LINUX_SYS_BRK => {
                frame.set_return_value(self.sys_brk(current, a0 as usize));
                saved_sp
            }
            LINUX_SYS_MMAP | LINUX_SYS_MMAP2 => {
                let offset = if nr == LINUX_SYS_MMAP2 {
                    (a5 as usize).saturating_mul(memory::PAGE_SIZE)
                } else {
                    a5 as usize
                };
                frame.set_return_value(self.sys_mmap(
                    current,
                    a0 as usize,
                    a1 as usize,
                    a2,
                    a3,
                    a4 as i32,
                    offset,
                ));
                saved_sp
            }
            LINUX_SYS_MUNMAP => {
                frame.set_return_value(self.sys_munmap(current, a0 as usize, a1 as usize));
                saved_sp
            }
            LINUX_SYS_MPROTECT => {
                frame.set_return_value(self.sys_mprotect(current, a0 as usize, a1 as usize, a2));
                saved_sp
            }
            LINUX_SYS_IOCTL => {
                frame.set_return_value(self.sys_ioctl(current, a0, a1, a2));
                saved_sp
            }
            LINUX_SYS_FCNTL | LINUX_SYS_FCNTL64 => {
                frame.set_return_value(self.sys_fcntl(current, a0, a1, a2));
                saved_sp
            }
            LINUX_SYS_FSYNC | LINUX_SYS_FDATASYNC | LINUX_SYS_FLOCK | LINUX_SYS_MSYNC => {
                frame.set_return_value(0);
                saved_sp
            }
            LINUX_SYS_ACCESS => {
                frame.set_return_value(self.sys_access_cstr(current, a0, a1));
                saved_sp
            }
            LINUX_SYS_FACCESSAT => {
                frame.set_return_value(self.sys_faccessat_cstr(current, a0 as i32, a1, a2));
                saved_sp
            }
            LINUX_SYS_STAT => {
                frame.set_return_value(self.sys_stat_cstr(current, a0, a1, false, false));
                saved_sp
            }
            LINUX_SYS_LSTAT => {
                frame.set_return_value(self.sys_stat_cstr(current, a0, a1, false, true));
                saved_sp
            }
            LINUX_SYS_STAT64 => {
                frame.set_return_value(self.sys_stat_cstr(current, a0, a1, true, false));
                saved_sp
            }
            LINUX_SYS_LSTAT64 => {
                frame.set_return_value(self.sys_stat_cstr(current, a0, a1, true, true));
                saved_sp
            }
            LINUX_SYS_FSTAT => {
                frame.set_return_value(self.sys_fstat(current, a0, a1, false));
                saved_sp
            }
            LINUX_SYS_FSTAT64 => {
                frame.set_return_value(self.sys_fstat(current, a0, a1, true));
                saved_sp
            }
            LINUX_SYS_FSTATAT64 => {
                frame.set_return_value(self.sys_newfstatat(current, a0 as i32, a1, a2, a3, true));
                saved_sp
            }
            LINUX_SYS_STATX => {
                frame.set_return_value(self.sys_statx(current, a0 as i32, a1, a2, a3, a4));
                saved_sp
            }
            LINUX_SYS_GETDENTS => {
                frame.set_return_value(self.sys_getdents(current, a0, a1, a2));
                saved_sp
            }
            LINUX_SYS_GETDENTS64 => {
                frame.set_return_value(self.sys_getdents64(current, a0, a1, a2));
                saved_sp
            }
            LINUX_SYS_CREAT => {
                frame.set_return_value(self.sys_creat_cstr(current, a0, a1));
                saved_sp
            }
            LINUX_SYS_MKDIR => {
                frame.set_return_value(self.sys_mkdir_cstr(current, a0, a1));
                saved_sp
            }
            LINUX_SYS_MKDIRAT => {
                frame.set_return_value(self.sys_mkdirat_cstr(current, a0 as i32, a1, a2));
                saved_sp
            }
            LINUX_SYS_MKNODAT => {
                frame.set_return_value(self.sys_mknodat_cstr(current, a0 as i32, a1, a2));
                saved_sp
            }
            LINUX_SYS_FCHMODAT | LINUX_SYS_FCHOWNAT => {
                frame.set_return_value(0);
                saved_sp
            }
            LINUX_SYS_RMDIR | LINUX_SYS_UNLINK => {
                frame.set_return_value(self.sys_unlink_cstr(current, a0));
                saved_sp
            }
            LINUX_SYS_LINK => {
                frame.set_return_value(self.sys_link_cstr(current, a0, a1));
                saved_sp
            }
            LINUX_SYS_UNLINKAT => {
                frame.set_return_value(self.sys_unlinkat_cstr(current, a0 as i32, a1, a2));
                saved_sp
            }
            LINUX_SYS_LINKAT => {
                frame.set_return_value(
                    self.sys_linkat_cstr(current, a0 as i32, a1, a2 as i32, a3, a4),
                );
                saved_sp
            }
            LINUX_SYS_RENAME => {
                frame.set_return_value(self.sys_rename(current, a0, a1));
                saved_sp
            }
            LINUX_SYS_RENAMEAT => {
                frame.set_return_value(
                    self.sys_renameat_cstr(current, a0 as i32, a1, a2 as i32, a3),
                );
                saved_sp
            }
            LINUX_SYS_RENAMEAT2 => {
                frame.set_return_value(
                    self.sys_renameat2_cstr(current, a0 as i32, a1, a2 as i32, a3, a4),
                );
                saved_sp
            }
            LINUX_SYS_TRUNCATE => {
                frame.set_return_value(self.sys_truncate_cstr(current, a0, a1 as usize));
                saved_sp
            }
            LINUX_SYS_FTRUNCATE => {
                frame.set_return_value(self.sys_ftruncate(current, a0, a1 as usize));
                saved_sp
            }
            LINUX_SYS_LLSEEK => {
                frame.set_return_value(self.sys_llseek(current, a0, a1, a2, a3, a4));
                saved_sp
            }
            LINUX_SYS_GETPID => {
                frame.set_return_value(self.task(current).process_id.as_usize() as u32);
                saved_sp
            }
            LINUX_SYS_GETPPID => {
                let ppid = self.processes.parent_of(self.task(current).process_id);
                frame.set_return_value(ppid.as_usize() as u32);
                saved_sp
            }
            LINUX_SYS_GETUID | LINUX_SYS_GETUID32 | LINUX_SYS_GETEUID | LINUX_SYS_GETEUID32 => {
                let uid = self
                    .processes
                    .credentials(self.task(current).process_id)
                    .map(|creds| {
                        if nr == LINUX_SYS_GETEUID || nr == LINUX_SYS_GETEUID32 {
                            creds.euid
                        } else {
                            creds.uid
                        }
                    })
                    .unwrap_or(0);
                frame.set_return_value(uid);
                saved_sp
            }
            LINUX_SYS_GETGID | LINUX_SYS_GETGID32 | LINUX_SYS_GETEGID | LINUX_SYS_GETEGID32 => {
                let gid = self
                    .processes
                    .credentials(self.task(current).process_id)
                    .map(|creds| {
                        if nr == LINUX_SYS_GETEGID || nr == LINUX_SYS_GETEGID32 {
                            creds.egid
                        } else {
                            creds.gid
                        }
                    })
                    .unwrap_or(0);
                frame.set_return_value(gid);
                saved_sp
            }
            LINUX_SYS_UNAME => {
                frame.set_return_value(self.sys_uname(a0));
                saved_sp
            }
            LINUX_SYS_GETTIMEOFDAY => {
                frame.set_return_value(self.sys_gettimeofday(a0, a1));
                saved_sp
            }
            LINUX_SYS_CLOCK_GETTIME => {
                frame.set_return_value(self.sys_clock_gettime(a0, a1));
                saved_sp
            }
            LINUX_SYS_CLOCK_GETTIME64 => {
                frame.set_return_value(self.sys_clock_gettime64(a0, a1));
                saved_sp
            }
            LINUX_SYS_CLOCK_GETRES => {
                frame.set_return_value(self.sys_clock_getres(a0, a1));
                saved_sp
            }
            LINUX_SYS_CLOCK_GETRES_TIME64 => {
                frame.set_return_value(self.sys_clock_getres64(a0, a1));
                saved_sp
            }
            LINUX_SYS_NANOSLEEP => match self.sleep_timespec(current, a0, a1, false) {
                Ok(()) => {
                    frame.set_return_value(0);
                    self.schedule_from_current(current, ScheduleReason::Sleep)
                }
                Err(err) => {
                    frame.set_return_value(err);
                    saved_sp
                }
            },
            LINUX_SYS_CLOCK_NANOSLEEP | LINUX_SYS_CLOCK_NANOSLEEP_TIME64 => {
                let time64 = nr == LINUX_SYS_CLOCK_NANOSLEEP_TIME64;
                match self.sleep_timespec(current, a2, a3, time64) {
                    Ok(()) => {
                        frame.set_return_value(0);
                        self.schedule_from_current(current, ScheduleReason::Sleep)
                    }
                    Err(err) => {
                        frame.set_return_value(err);
                        saved_sp
                    }
                }
            }
            LINUX_SYS_EXECVE => {
                frame.set_return_value(self.sys_execve(current, a0, a1, a2));
                saved_sp
            }
            LINUX_SYS_WAITPID | LINUX_SYS_WAIT4 => {
                let rusage = if nr == LINUX_SYS_WAIT4 { a3 } else { 0 };
                match self.sys_wait4(current, a0 as i32, a1, a2, rusage) {
                    Ok(result) => {
                        frame.set_return_value(result);
                        saved_sp
                    }
                    Err(EAGAIN) => {
                        frame.set_return_value(EAGAIN);
                        let channel = child_wait_channel(self.task(current).process_id);
                        let wait_pid = if (a0 as i32) > 0 { a0 as usize } else { 0 };
                        let task = self.task_mut(current);
                        task.mark_waiting_for_child(
                            channel,
                            wait_pid,
                            a1 as usize,
                            rusage as usize,
                        );
                        self.wait.sleep_on(channel, current);
                        self.schedule_from_current(current, ScheduleReason::Block)
                    }
                    Err(err) => {
                        frame.set_return_value(err);
                        saved_sp
                    }
                }
            }
            LINUX_SYS_DUP => {
                frame.set_return_value(self.sys_dup(current, a0, 0, false));
                saved_sp
            }
            LINUX_SYS_DUP2 => {
                frame.set_return_value(self.sys_dup2(current, a0, a1, false));
                saved_sp
            }
            LINUX_SYS_DUP3 => {
                frame.set_return_value(self.sys_dup3(current, a0, a1, a2));
                saved_sp
            }
            LINUX_SYS_PIPE => {
                frame.set_return_value(self.sys_pipe2(current, a0, 0));
                saved_sp
            }
            LINUX_SYS_PIPE2 => {
                frame.set_return_value(self.sys_pipe2(current, a0, a1));
                saved_sp
            }
            LINUX_SYS_GETCWD => {
                frame.set_return_value(self.sys_getcwd(current, a0, a1));
                saved_sp
            }
            LINUX_SYS_CHDIR => {
                frame.set_return_value(self.sys_chdir_cstr(current, a0));
                saved_sp
            }
            LINUX_SYS_READLINK => {
                frame.set_return_value(self.sys_readlink_cstr(current, a0, a1, a2));
                saved_sp
            }
            LINUX_SYS_READLINKAT => {
                frame.set_return_value(self.sys_readlinkat_cstr(current, a0 as i32, a1, a2, a3));
                saved_sp
            }
            LINUX_SYS_POLL | LINUX_SYS_PPOLL => {
                let result = self.sys_poll(current, a0, a1, a2 as i32);
                if result == 0 && a2 as i32 != 0 && self.poll_waits_for_console(current, a0, a1) {
                    frame.set_return_value(EAGAIN);
                    let task = self.task_mut(current);
                    task.mark_waiting_for_poll(
                        console::INPUT_WAIT_CHANNEL,
                        a0 as usize,
                        a1 as usize,
                    );
                    self.wait.sleep_on(console::INPUT_WAIT_CHANNEL, current);
                    self.schedule_from_current(current, ScheduleReason::Block)
                } else {
                    frame.set_return_value(result);
                    saved_sp
                }
            }
            LINUX_SYS_SELECT | LINUX_SYS_NEWSELECT | LINUX_SYS_PSELECT6 => {
                frame.set_return_value(self.sys_select(current, a0, a1, a2, a3, a4));
                saved_sp
            }
            LINUX_SYS_RT_SIGACTION | LINUX_SYS_RT_SIGPROCMASK => {
                frame.set_return_value(0);
                saved_sp
            }
            LINUX_SYS_RT_SIGRETURN => {
                frame.set_return_value(EINVAL);
                saved_sp
            }
            ARM_SYS_SET_TLS => {
                self.task_mut(current).user_tls = a0;
                crate::arch::aarch32::cpu::set_user_tls(a0);
                frame.set_return_value(0);
                saved_sp
            }
            LINUX_SYS_SET_TID_ADDRESS => {
                frame.set_return_value(self.task(current).process_id.as_usize() as u32);
                saved_sp
            }
            LINUX_SYS_SET_ROBUST_LIST => {
                frame.set_return_value(0);
                saved_sp
            }
            LINUX_SYS_GET_ROBUST_LIST => {
                frame.set_return_value(EINVAL);
                saved_sp
            }
            LINUX_SYS_GETRLIMIT | LINUX_SYS_UGETRLIMIT => {
                frame.set_return_value(self.sys_getrlimit(a0, a1));
                saved_sp
            }
            LINUX_SYS_GETGROUPS | LINUX_SYS_GETGROUPS32 => {
                frame.set_return_value(self.sys_getgroups(a0, a1));
                saved_sp
            }
            LINUX_SYS_SETGROUPS | LINUX_SYS_SETGROUPS32 => {
                frame.set_return_value(0);
                saved_sp
            }
            LINUX_SYS_TIMES => {
                frame.set_return_value(self.sys_times(a0));
                saved_sp
            }
            LINUX_SYS_GETRUSAGE => {
                frame.set_return_value(self.sys_getrusage(a0, a1));
                saved_sp
            }
            LINUX_SYS_SYSINFO => {
                frame.set_return_value(self.sys_sysinfo(a0));
                saved_sp
            }
            LINUX_SYS_STATFS | LINUX_SYS_STATFS64 => {
                let (size, buf) = if nr == LINUX_SYS_STATFS64 {
                    (a1, a2)
                } else {
                    (core::mem::size_of::<StatFs64>() as u32, a1)
                };
                frame.set_return_value(self.sys_statfs_cstr(current, a0, size, buf));
                saved_sp
            }
            LINUX_SYS_FSTATFS | LINUX_SYS_FSTATFS64 => {
                let (size, buf) = if nr == LINUX_SYS_FSTATFS64 {
                    (a1, a2)
                } else {
                    (core::mem::size_of::<StatFs64>() as u32, a1)
                };
                frame.set_return_value(self.sys_fstatfs(current, a0, size, buf));
                saved_sp
            }
            LINUX_SYS_PRCTL => {
                frame.set_return_value(0);
                saved_sp
            }
            LINUX_SYS_MADVISE => {
                frame.set_return_value(0);
                saved_sp
            }
            LINUX_SYS_FUTEX => {
                frame.set_return_value(self.sys_futex(a0, a1, a2));
                saved_sp
            }
            LINUX_SYS_GETTID => {
                frame.set_return_value(self.task(current).thread_id.as_usize() as u32);
                saved_sp
            }
            LINUX_SYS_KILL => {
                frame.set_return_value(self.sys_kill(current, a0 as i32, a1 as i32));
                if self.task(current).state == TaskState::Zombie {
                    self.schedule_from_current(current, ScheduleReason::Exit)
                } else {
                    saved_sp
                }
            }
            LINUX_SYS_TGKILL => {
                frame.set_return_value(self.sys_kill(current, a1 as i32, a2 as i32));
                if self.task(current).state == TaskState::Zombie {
                    self.schedule_from_current(current, ScheduleReason::Exit)
                } else {
                    saved_sp
                }
            }
            LINUX_SYS_GETRANDOM => {
                frame.set_return_value(self.sys_getrandom(a0, a1, a2));
                saved_sp
            }
            LINUX_SYS_SOCKET
            | LINUX_SYS_BIND
            | LINUX_SYS_CONNECT
            | LINUX_SYS_LISTEN
            | LINUX_SYS_ACCEPT
            | LINUX_SYS_GETSOCKNAME
            | LINUX_SYS_GETPEERNAME
            | LINUX_SYS_SOCKETPAIR
            | LINUX_SYS_SEND
            | LINUX_SYS_SENDTO
            | LINUX_SYS_RECV
            | LINUX_SYS_RECVFROM
            | LINUX_SYS_SHUTDOWN
            | LINUX_SYS_SETSOCKOPT
            | LINUX_SYS_GETSOCKOPT
            | LINUX_SYS_SENDMSG
            | LINUX_SYS_RECVMSG
            | LINUX_SYS_ACCEPT4 => {
                frame.set_return_value(EAFNOSUPPORT);
                saved_sp
            }
            LINUX_SYS_UMASK => {
                frame.set_return_value(0o022);
                saved_sp
            }
            LINUX_SYS_FORK | LINUX_SYS_VFORK | LINUX_SYS_CLONE => {
                let result = self.sys_fork(current, frame, nr, a0, a2, a4);
                frame.set_return_value(result);
                if is_error(result) {
                    saved_sp
                } else {
                    self.schedule_from_current(current, ScheduleReason::Yield)
                }
            }
            _ => {
                if crate::config::CONFIG_BOOT_VERBOSE {
                    println!(
                        "syscall: linux nr={} a0={:#x} a1={:#x} a2={:#x} a3={:#x} a4={:#x} a5={:#x} a6={:#x} -> ENOSYS",
                        nr, a0, a1, a2, a3, a4, a5, a6
                    );
                }
                frame.set_return_value(ENOSYS);
                saved_sp
            }
        }
    }

    fn finish_read_syscall(
        &mut self,
        current: TaskId,
        frame: &mut TrapFrame,
        action: SyscallAction,
        saved_sp: *mut u32,
    ) -> *mut u32 {
        match action {
            SyscallAction::Return(result) => {
                frame.set_return_value(result);
                saved_sp
            }
            SyscallAction::Block { channel } => {
                frame.set_return_value(EAGAIN);
                self.wait.sleep_on(channel, current);
                self.schedule_from_current(current, ScheduleReason::Block)
            }
        }
    }

    pub fn dump_recent_logs(&self) {
        self.log.dump_recent(16);
    }

    pub fn dump_tasks(&self) {
        print!("scheduler: states");
        for state in TASK_STATES {
            print!(" {}", state.as_str());
        }
        println!();
        println!(
            "scheduler: task table (tasks={}, ready={}, sleep={}, ready_empty={})",
            self.count,
            self.ready.len(),
            self.sleep.len(),
            self.ready.is_empty()
        );
        for idx in 0..MAX_TASKS {
            let task = &self.tasks[idx];
            if task.state == TaskState::Empty {
                continue;
            }
            println!(
                "  id={} pid={} tid={} mode={} as={} proc={} name={} state={} prio={} slice={} remain={} wake={} wait={} kstack={:#010x}+{}p ustack={:#010x}..{:#010x}/phys={:#010x}+{}p entry={:#010x} runtime={} scheduled={} sleep={} block={} write={} last_sys={}",
                task.id.index(),
                task.pid,
                task.thread_id.as_usize(),
                task.mode.as_str(),
                self.processes.address_space_kind(task.process_id),
                self.processes.state_str(task.process_id),
                task.name,
                task.state.as_str(),
                task.priority,
                task.time_slice_ticks,
                task.remaining_ticks,
                task.wake_at_tick,
                task.wait_channel,
                task.kernel_stack_start as usize,
                task.kernel_stack_pages,
                task.user_stack_bottom,
                task.user_stack_top,
                task.user_stack_phys as usize,
                task.user_stack_pages,
                task.user_entry,
                task.stats.runtime_ticks,
                task.stats.scheduled_count,
                task.stats.sleep_count,
                task.stats.block_count,
                task.stats.bytes_written,
                task.stats.last_syscall
            );
        }
        self.dump_kernel_core_summary();
    }

    pub fn dump_tasks_summary(&self) {
        println!(
            "  ticks={} switches={} tasks={} ready={} sleep={}",
            self.ticks,
            self.switches,
            self.count,
            self.ready.len(),
            self.sleep.len()
        );
        for idx in 0..MAX_TASKS {
            let task = &self.tasks[idx];
            if task.state == TaskState::Empty {
                continue;
            }
            println!(
                "  id={} pid={} tid={} mode={} name={} state={} prio={} runtime={} sched={} last_sys={} kstack={:#010x}+{}p ustack={:#010x}..{:#010x} vmas={} regions={} files={}",
                task.id.index(),
                task.pid,
                task.thread_id.as_usize(),
                task.mode.as_str(),
                task.name,
                task.state.as_str(),
                task.priority,
                task.stats.runtime_ticks,
                task.stats.scheduled_count,
                task.stats.last_syscall,
                task.kernel_stack_start as usize,
                task.kernel_stack_pages,
                task.user_stack_bottom,
                task.user_stack_top,
                self.processes.vma_count(task.process_id),
                self.processes.owned_region_count(task.process_id),
                self.processes.open_file_count(task.process_id)
            );
        }
        self.dump_kernel_core_summary();
    }

    fn dump_kernel_core_summary(&self) {
        println!(
            "  memory: allocated_pages={} free_pages={} free_blocks={} largest_order={}",
            memory::allocated_pages(),
            memory::free_page_count(),
            memory::free_ranges(),
            memory::largest_free_order().unwrap_or(0)
        );
        if let Some(stats) = crate::kernel::slab::stats(0) {
            println!(
                "  slab: class={} pages={} objects={} allocated={} free={}",
                stats.class_size, stats.pages, stats.objects, stats.allocated, stats.free
            );
        }
        println!("  ipc: pipes={}", ipc::active_count());
    }

    pub fn current_task_name(&self) -> &'static str {
        self.current
            .map(|id| self.task(id).name)
            .unwrap_or("<none>")
    }

    pub fn current_task_pid(&self) -> usize {
        self.current.map(|id| self.task(id).pid).unwrap_or(0)
    }

    pub fn current_task_last_syscall(&self) -> u32 {
        self.current
            .map(|id| self.task(id).stats.last_syscall)
            .unwrap_or(0)
    }

    fn schedule_from_current(&mut self, current: TaskId, reason: ScheduleReason) -> *mut u32 {
        self.enqueue_if_runnable(current);

        let Some(next) = self.pick_next() else {
            if self.task(current).state == TaskState::Running {
                self.current = Some(current);
                let tick = self.ticks;
                let current_task = self.task_mut(current);
                current_task.mark_scheduled(tick);
                return current_task.context.sp;
            }

            panic!("no runnable task after {}", reason.as_str());
        };

        let previous = current;
        self.current = Some(next);
        if previous != next {
            self.switches = self.switches.wrapping_add(1);
        }

        {
            let tick = self.ticks;
            let next_task = self.task_mut(next);
            next_task.mark_scheduled(tick);
        }
        self.switch_address_space(next);

        if previous != next {
            self.log.push(LogEvent::Schedule {
                tick: self.ticks,
                from: self.task(previous).name,
                to: self.task(next).name,
                reason: reason.as_str(),
                ready: self.ready.len(),
                switches: self.switches,
            });
        }

        self.task(next).context.sp
    }

    fn enqueue_if_runnable(&mut self, task_id: TaskId) {
        if Some(task_id) == self.idle && !self.ready.is_empty() {
            self.task_mut(task_id).state = TaskState::Ready;
            return;
        }
        if self.task(task_id).is_runnable() {
            self.task_mut(task_id).state = TaskState::Ready;
            self.enqueue_ready(task_id);
        }
    }

    fn enqueue_ready(&mut self, task_id: TaskId) {
        let priority = self.task(task_id).priority;
        self.ready.push_back(task_id, priority);
    }

    fn wake_sleepers(&mut self) {
        while let Some(task) = self.sleep.pop_expired(self.ticks) {
            if self.task(task).state == TaskState::Sleeping {
                let t = self.task_mut(task);
                t.state = TaskState::Ready;
                t.wake_at_tick = 0;
                self.enqueue_ready(task);
            }
        }
    }

    fn reap_zombies(&mut self) {
        for idx in 0..MAX_TASKS {
            if self.tasks[idx].state != TaskState::Zombie {
                continue;
            }

            if Some(self.tasks[idx].id) == self.current {
                continue;
            }

            let pid = self.tasks[idx].pid;
            let name = self.tasks[idx].name;
            let process_id = self.tasks[idx].process_id;
            let parent = self.processes.parent_of(process_id);
            if let Ok(closed) = self.processes.close_all_files(process_id) {
                for object in closed.iter() {
                    self.close_file_object(object);
                }
            }
            let old_address_space = self
                .processes
                .mark_resources_reaped(process_id, kernel_address_space());
            #[cfg(feature = "mmu")]
            if let Some(address_space) = old_address_space {
                loader::release_address_space_regions(address_space);
            }
            #[cfg(not(feature = "mmu"))]
            let _ = old_address_space;
            unsafe {
                self.tasks[idx].release_resources();
            }
            self.count = self.count.saturating_sub(1);
            self.log.push(LogEvent::TaskReap {
                tick: self.ticks,
                name,
                pid,
            });

            if self.processes.has_live_parent(process_id) {
                self.wake_waiters(parent);
            } else {
                self.processes.release(process_id);
            }
        }
    }

    fn pick_next(&mut self) -> Option<TaskId> {
        while let Some(candidate) = self.ready.pop_front() {
            if self.task(candidate).state == TaskState::Ready {
                return Some(candidate);
            }
        }

        self.idle
            .filter(|idle| self.task(*idle).state == TaskState::Ready)
    }

    fn alloc_task_slot(&self) -> TaskId {
        self.try_alloc_task_slot()
            .unwrap_or_else(|_| panic!("task table full"))
    }

    fn try_alloc_task_slot(&self) -> Result<TaskId, u32> {
        for idx in 0..MAX_TASKS {
            if self.tasks[idx].state == TaskState::Empty {
                return Ok(TaskId::new(idx));
            }
        }
        Err(ENOMEM)
    }

    fn alloc_thread_id(&mut self) -> ThreadId {
        let id = ThreadId::new(self.next_tid);
        self.next_tid = self.next_tid.wrapping_add(1).max(1);
        id
    }

    #[cfg(feature = "mmu")]
    fn alloc_asid(&mut self) -> u16 {
        let asid = self.next_asid;
        self.next_asid = self.next_asid.wrapping_add(1);
        if self.next_asid == 0 {
            self.next_asid = 1;
        }
        asid
    }

    fn switch_address_space(&self, task_id: TaskId) {
        #[cfg(feature = "mmu")]
        if let Some(address_space) = self.processes.address_space(self.task(task_id).process_id) {
            let root = if address_space.page_table_root.as_usize() == 0 {
                crate::arch::aarch32::mmu::table_base()
            } else {
                address_space.page_table_root.as_usize()
            };
            crate::arch::aarch32::mmu::switch_table(root, address_space.asid);
            crate::arch::aarch32::cpu::set_user_tls(self.task(task_id).user_tls);
        }
        #[cfg(not(feature = "mmu"))]
        let _ = task_id;
    }

    #[cfg(feature = "mmu")]
    fn build_user_address_space_with_args<F>(
        &mut self,
        stack_slot: usize,
        load: F,
        argv: Option<&[loader::UserArg]>,
        envp: Option<&[loader::UserArg]>,
    ) -> Result<(AddressSpace, loader::LoadedImage, *mut u8, usize), u32>
    where
        F: FnOnce(&mut AddressSpace) -> Result<loader::LoadedImage, u32>,
    {
        let stack_bottom = user_stack_bottom(stack_slot);
        let stack_top = user_stack_top(stack_slot);
        let root = unsafe { crate::arch::aarch32::mmu::create_user_table() }.ok_or(ENOMEM)?;
        let asid = self.alloc_asid();

        let mut address_space = AddressSpace::user(
            PhysAddr::new(root),
            asid,
            crate::arch::aarch32::mmu::L1_TABLE_PAGES,
            VirtAddr::new(0),
            VirtAddr::new(0),
            VirtAddr::new(0),
            VirtAddr::new(stack_bottom),
            VirtAddr::new(stack_top),
        );

        let loaded = match load(&mut address_space) {
            Ok(loaded) => loaded,
            Err(err) => {
                loader::release_address_space_regions(address_space);
                return Err(err);
            }
        };
        address_space.entry = loaded.entry;
        address_space.user_start = loaded.user_start;
        address_space.user_end = loaded.user_end;
        address_space.brk_start = loaded.brk_start;
        address_space.brk = loaded.brk_start;
        address_space.mmap_base = VirtAddr::new(USER_MMAP_BASE);
        address_space.mmap_next = VirtAddr::new(USER_MMAP_BASE);
        address_space.phdr = loaded.phdr;
        address_space.phnum = loaded.phnum;
        address_space.phent = loaded.phent;
        address_space.interp_base = loaded.interpreter_base;
        address_space.interp_entry = loaded.interpreter_entry;

        let Some(user_stack) = memory::alloc_pages(USER_STACK_PAGES) else {
            loader::release_address_space_regions(address_space);
            return Err(ENOMEM);
        };

        if let Err(err) = self.map_user_stack(&mut address_space, stack_bottom, user_stack as usize)
        {
            unsafe {
                memory::free_pages(user_stack, USER_STACK_PAGES);
            }
            loader::release_address_space_regions(address_space);
            return Err(err);
        }

        let default_argv = [loader::UserArg::from_bytes(loaded.name.as_bytes())];
        let argv = argv.unwrap_or(&default_argv);
        let initial_stack = match loader::build_initial_stack_with_args(
            user_stack,
            stack_bottom,
            stack_top,
            &loaded,
            argv,
            envp.unwrap_or(&[]),
        ) {
            Ok(stack) => stack,
            Err(err) => {
                unsafe {
                    memory::free_pages(user_stack, USER_STACK_PAGES);
                }
                loader::release_address_space_regions(address_space);
                return Err(err);
            }
        };

        Ok((address_space, loaded, user_stack, initial_stack.sp))
    }

    #[cfg(feature = "mmu")]
    fn map_user_stack(
        &mut self,
        address_space: &mut AddressSpace,
        stack_bottom: usize,
        phys: usize,
    ) -> Result<(), u32> {
        let mut addr = stack_bottom;
        let end = stack_bottom + USER_STACK_PAGES * memory::PAGE_SIZE;
        while addr < end {
            let l2 = unsafe {
                crate::arch::aarch32::mmu::ensure_user_l2(
                    address_space.page_table_root.as_usize(),
                    addr,
                )
            }
            .ok_or(ENOMEM)?;
            if let Err(err) = address_space.try_add_owned_l2(PhysAddr::new(l2)) {
                unsafe {
                    crate::arch::aarch32::mmu::free_user_l2(l2);
                }
                return Err(err);
            }
            addr = crate::kernel::address::align_up(addr + 1, 1024 * 1024);
        }
        unsafe {
            crate::arch::aarch32::mmu::map_user_pages_in(
                address_space.page_table_root.as_usize(),
                stack_bottom,
                phys,
                USER_STACK_PAGES,
                crate::arch::aarch32::mmu::UserMapping::Stack,
            );
        }
        address_space.try_add_vma(
            VirtAddr::new(stack_bottom),
            VirtAddr::new(stack_bottom + USER_STACK_PAGES * memory::PAGE_SIZE),
            crate::kernel::process::VM_READ
                | crate::kernel::process::VM_WRITE
                | crate::kernel::process::VM_USER
                | crate::kernel::process::VM_STACK
                | crate::kernel::process::VM_ANON,
            None,
        )?;
        address_space.try_add_owned_region(
            VirtAddr::new(stack_bottom),
            PhysAddr::new(phys),
            USER_STACK_PAGES,
        )
    }

    #[cfg(feature = "mmu")]
    fn map_user_alloc(
        &mut self,
        process_id: ProcessId,
        virt: usize,
        phys: usize,
        pages: usize,
        flags: u32,
        file: Option<vfs::InodeRef>,
    ) -> Result<(), u32> {
        let Some(space) = self.processes.address_space_mut(process_id) else {
            return Err(EINVAL);
        };
        let mut addr = virt;
        let end = virt + pages * memory::PAGE_SIZE;
        while addr < end {
            let l2 = unsafe {
                crate::arch::aarch32::mmu::ensure_user_l2(space.page_table_root.as_usize(), addr)
            }
            .ok_or(ENOMEM)?;
            if let Err(err) = space.try_add_owned_l2(PhysAddr::new(l2)) {
                unsafe {
                    crate::arch::aarch32::mmu::free_user_l2(l2);
                }
                return Err(err);
            }
            addr = crate::kernel::address::align_up(addr + 1, 1024 * 1024);
        }
        let mapping = user_mapping_from_vm(flags);
        unsafe {
            crate::arch::aarch32::mmu::map_user_pages_in(
                space.page_table_root.as_usize(),
                virt,
                phys,
                pages,
                mapping,
            );
        }
        space.try_add_vma(
            VirtAddr::new(virt),
            VirtAddr::new(virt + pages * memory::PAGE_SIZE),
            flags,
            file,
        )?;
        space.try_add_owned_region(VirtAddr::new(virt), PhysAddr::new(phys), pages)
    }

    #[cfg(feature = "mmu")]
    fn clone_address_space(&mut self, parent: AddressSpace) -> Result<AddressSpace, u32> {
        if !parent.kind.is_user() {
            return Err(EINVAL);
        }

        let root = unsafe { crate::arch::aarch32::mmu::create_user_table() }.ok_or(ENOMEM)?;
        let mut child = parent;
        child.page_table_root = PhysAddr::new(root);
        child.asid = self.alloc_asid();
        child.l1_pages = crate::arch::aarch32::mmu::L1_TABLE_PAGES;
        child.owned_l2 = [PhysAddr::new(0); crate::kernel::process::MAX_ADDRESS_SPACE_L2];
        child.owned_l2_count = 0;
        child.owned_regions = [crate::kernel::process::UserRegion::empty();
            crate::kernel::process::MAX_ADDRESS_SPACE_REGIONS];
        child.owned_region_count = 0;
        child.vmas = [crate::kernel::process::VmArea::empty(); crate::kernel::process::MAX_VMAS];
        child.vma_count = 0;

        let mut cleanup = child;
        for region in parent.owned_regions.iter().take(parent.owned_region_count) {
            let mut offset_pages = 0usize;
            while offset_pages < region.pages {
                let virt = region.virt.as_usize() + offset_pages * memory::PAGE_SIZE;
                let remaining_pages = region.pages - offset_pages;
                let region_end = region.virt.as_usize() + region.pages * memory::PAGE_SIZE;
                let vma = parent.find_vma(VirtAddr::new(virt));
                let mapping = vma
                    .map(|vma| user_mapping_from_vm(vma.flags))
                    .unwrap_or(crate::arch::aarch32::mmu::UserMapping::RwData);
                let span_end = vma
                    .map(|vma| vma.end.as_usize().min(region_end))
                    .unwrap_or(virt + memory::PAGE_SIZE);
                let span_pages = ((span_end.saturating_sub(virt)) / memory::PAGE_SIZE)
                    .max(1)
                    .min(remaining_pages);
                let Some(dst) = memory::alloc_exact_pages(span_pages) else {
                    loader::release_address_space_regions(cleanup);
                    return Err(ENOMEM);
                };
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        (region.phys.as_usize() + offset_pages * memory::PAGE_SIZE) as *const u8,
                        dst,
                        span_pages * memory::PAGE_SIZE,
                    );
                }
                if let Err(err) = self.map_owned_range_in_space(
                    &mut cleanup,
                    virt,
                    dst as usize,
                    span_pages,
                    mapping,
                ) {
                    unsafe {
                        memory::free_exact_pages(dst, span_pages);
                    }
                    loader::release_address_space_regions(cleanup);
                    return Err(err);
                }
                offset_pages += span_pages;
            }
        }

        Ok(cleanup)
    }

    #[cfg(feature = "mmu")]
    fn map_owned_range_in_space(
        &mut self,
        address_space: &mut AddressSpace,
        virt: usize,
        phys: usize,
        pages: usize,
        mapping: crate::arch::aarch32::mmu::UserMapping,
    ) -> Result<(), u32> {
        let mut addr = virt;
        let end = virt + pages * memory::PAGE_SIZE;
        while addr < end {
            let l2 = unsafe {
                crate::arch::aarch32::mmu::ensure_user_l2(
                    address_space.page_table_root.as_usize(),
                    addr,
                )
            }
            .ok_or(ENOMEM)?;
            if let Err(err) = address_space.try_add_owned_l2(PhysAddr::new(l2)) {
                unsafe {
                    crate::arch::aarch32::mmu::free_user_l2(l2);
                }
                return Err(err);
            }
            addr = crate::kernel::address::align_up(addr + 1, 1024 * 1024);
        }
        unsafe {
            crate::arch::aarch32::mmu::map_user_pages_in(
                address_space.page_table_root.as_usize(),
                virt,
                phys,
                pages,
                mapping,
            );
        }
        address_space.try_add_owned_region(VirtAddr::new(virt), PhysAddr::new(phys), pages)
    }

    #[cfg(feature = "mmu")]
    fn sys_fork(
        &mut self,
        current: TaskId,
        parent_frame: &TrapFrame,
        nr: u32,
        flags: u32,
        parent_tidptr: u32,
        child_tidptr: u32,
    ) -> u32 {
        if !self.task(current).mode.is_user() {
            return EINVAL;
        }
        let parent_pid = self.task(current).process_id;
        let Some(parent_space) = self.processes.address_space(parent_pid) else {
            return EINVAL;
        };
        let shares_vm =
            nr == LINUX_SYS_CLONE && flags & (CLONE_VM | CLONE_THREAD) == (CLONE_VM | CLONE_THREAD);
        let child_space = if shares_vm {
            self.shared_clone_address_space(parent_space)
        } else {
            match self.clone_address_space(parent_space) {
                Ok(space) => space,
                Err(err) => return err,
            }
        };

        let child_pid = match self.processes.try_fork_from(parent_pid, child_space) {
            Ok(pid) => pid,
            Err(err) => {
                if !shares_vm {
                    loader::release_address_space_regions(child_space);
                }
                return err;
            }
        };

        let id = match self.try_alloc_task_slot() {
            Ok(id) => id,
            Err(err) => {
                if !shares_vm {
                    loader::release_address_space_regions(child_space);
                }
                self.processes.release(child_pid);
                return err;
            }
        };

        let thread_id = self.alloc_thread_id();
        let parent = self.task(current);
        self.tasks[id.index()] = TaskControlBlock::new_user(
            id,
            child_pid,
            thread_id,
            parent.name,
            parent.user_entry,
            core::ptr::null_mut(),
            0,
            child_space.stack_bottom.as_usize(),
            child_space.stack_top.as_usize(),
            parent_frame.user_sp as usize,
            parent.priority,
            parent.time_slice_ticks,
        );
        self.tasks[id.index()].context = TaskContext {
            sp: self.tasks[id.index()].context.sp,
        };
        unsafe {
            *TrapFrame::from_saved_sp(self.tasks[id.index()].context.sp) = *parent_frame;
        }
        let child_frame = unsafe { TrapFrame::from_saved_sp(self.tasks[id.index()].context.sp) };
        child_frame.set_return_value(0);
        if nr == LINUX_SYS_CLONE && parent_frame.linux_arg1() != 0 {
            child_frame.user_sp = parent_frame.linux_arg1();
        }
        self.tasks[id.index()].user_tls = self.task(current).user_tls;
        self.tasks[id.index()].user_entry = self.task(current).user_entry;
        self.tasks[id.index()].user_stack_bottom = child_space.stack_bottom.as_usize();
        self.tasks[id.index()].user_stack_top = child_space.stack_top.as_usize();
        self.processes.set_main_thread(child_pid, thread_id);
        if flags & CLONE_PARENT_SETTID != 0 && parent_tidptr != 0 {
            let tid = child_pid.as_usize() as u32;
            let bytes = tid.to_le_bytes();
            if let Err(err) = unsafe { copy_to_user(UserPtr::new(parent_tidptr as usize), &bytes) }
            {
                if !shares_vm {
                    loader::release_address_space_regions(child_space);
                }
                self.processes.release(child_pid);
                unsafe {
                    self.tasks[id.index()].release_resources();
                }
                return err;
            }
        }
        if flags & CLONE_CHILD_SETTID != 0 && child_tidptr != 0 {
            let tid = child_pid.as_usize() as u32;
            let bytes = tid.to_le_bytes();
            if let Err(err) = self.copy_to_task_user(id, child_tidptr as usize, &bytes) {
                if !shares_vm {
                    loader::release_address_space_regions(child_space);
                }
                self.processes.release(child_pid);
                unsafe {
                    self.tasks[id.index()].release_resources();
                }
                return err;
            }
        }
        self.bump_forked_file_refs(child_pid);
        self.enqueue_ready(id);
        self.count += 1;
        child_pid.as_usize() as u32
    }

    #[cfg(not(feature = "mmu"))]
    fn sys_fork(
        &mut self,
        _current: TaskId,
        _parent_frame: &TrapFrame,
        _nr: u32,
        _flags: u32,
        _parent_tidptr: u32,
        _child_tidptr: u32,
    ) -> u32 {
        ENOSYS
    }

    #[cfg(feature = "mmu")]
    fn shared_clone_address_space(&self, parent: AddressSpace) -> AddressSpace {
        let mut child = parent;
        child.l1_pages = 0;
        child.owned_l2 = [PhysAddr::new(0); crate::kernel::process::MAX_ADDRESS_SPACE_L2];
        child.owned_l2_count = 0;
        child.owned_regions = [crate::kernel::process::UserRegion::empty();
            crate::kernel::process::MAX_ADDRESS_SPACE_REGIONS];
        child.owned_region_count = 0;
        child
    }

    fn sys_write(&mut self, current: TaskId, fd: u32, buf: u32, len: u32) -> u32 {
        let fd = fd as usize;
        let process_id = self.task(current).process_id;
        let Some(file) = self.processes.file(process_id, fd) else {
            return EBADF;
        };
        if !file.object.can_write() {
            return EBADF;
        }

        let len = len as usize;
        if len == 0 {
            return 0;
        }

        match file.object {
            FileObject::Regular { inode } => {
                let mut done = 0usize;
                let mut chunk = [0u8; 128];
                let mut offset = if file.flags & O_APPEND != 0 {
                    vfs::metadata(inode)
                        .map(|meta| meta.size)
                        .unwrap_or(file.offset)
                } else {
                    file.offset
                };
                while done < len {
                    let count = (len - done).min(chunk.len());
                    let ptr = UserPtr::new((buf as usize).wrapping_add(done));
                    let result = unsafe { copy_from_user(&mut chunk[..count], ptr) };
                    if let Err(err) = result {
                        return if done == 0 { err } else { done as u32 };
                    }
                    match vfs::write(inode, offset, &chunk[..count]) {
                        Ok(written) => {
                            done += written;
                            offset = offset.saturating_add(written);
                            if written < count {
                                break;
                            }
                        }
                        Err(err) => return if done == 0 { err } else { done as u32 },
                    }
                }
                if let Some(open_file) = self.processes.file_mut(process_id, fd) {
                    open_file.offset = offset;
                }
                self.task_mut(current).stats.bytes_written = self
                    .task(current)
                    .stats
                    .bytes_written
                    .wrapping_add(done as u64);
                done as u32
            }
            FileObject::Device { inode } => {
                let mut done = 0usize;
                let mut chunk = [0u8; 128];
                while done < len {
                    let count = (len - done).min(chunk.len());
                    let ptr = UserPtr::new((buf as usize).wrapping_add(done));
                    let result = unsafe { copy_from_user(&mut chunk[..count], ptr) };
                    if let Err(err) = result {
                        return if done == 0 { err } else { done as u32 };
                    }
                    match vfs::write(inode, 0, &chunk[..count]) {
                        Ok(written) => {
                            done += written;
                            if written < count {
                                break;
                            }
                        }
                        Err(err) => return if done == 0 { err } else { done as u32 },
                    }
                }
                done as u32
            }
            FileObject::Pipe {
                id,
                end: PipeEnd::Write,
            } => {
                let mut done = 0usize;
                let mut chunk = [0u8; 128];
                while done < len {
                    let count = (len - done).min(chunk.len());
                    let ptr = UserPtr::new((buf as usize).wrapping_add(done));
                    if let Err(err) = unsafe { copy_from_user(&mut chunk[..count], ptr) } {
                        return if done == 0 { err } else { done as u32 };
                    }
                    match ipc::write(id, &chunk[..count]) {
                        PipeIo::Write { bytes, wake_reader } => {
                            done += bytes;
                            if wake_reader {
                                self.wake_pipe_readers(id);
                            }
                            if bytes < count {
                                break;
                            }
                        }
                        PipeIo::WouldBlock { .. } => break,
                        PipeIo::Closed => return if done == 0 { EBADF } else { done as u32 },
                        PipeIo::Error(err) => return if done == 0 { err } else { done as u32 },
                        PipeIo::Read { .. } => return EINVAL,
                    }
                }
                done as u32
            }
            FileObject::Directory { .. } => EISDIR,
            FileObject::Console | FileObject::ConsoleOut | FileObject::ConsoleErr => {
                let mut done = 0usize;
                let mut chunk = [0u8; 128];
                while done < len {
                    let count = (len - done).min(chunk.len());
                    let ptr = UserPtr::new((buf as usize).wrapping_add(done));
                    let result = unsafe { copy_from_user(&mut chunk[..count], ptr) };
                    if let Err(err) = result {
                        return if done == 0 { err } else { done as u32 };
                    }

                    for byte in &chunk[..count] {
                        if *byte == b'\n' {
                            uart::put_byte(b'\r');
                        }
                        uart::put_byte(*byte);
                    }
                    done += count;
                }

                self.task_mut(current).stats.bytes_written = self
                    .task(current)
                    .stats
                    .bytes_written
                    .wrapping_add(done as u64);
                done as u32
            }
            _ => EBADF,
        }
    }

    fn sys_read(&mut self, current: TaskId, fd: u32, buf: u32, len: u32) -> SyscallAction {
        self.wait_poll_console_input();
        let fd = fd as usize;
        if !self
            .processes
            .can_read_fd(self.task(current).process_id, fd)
        {
            return SyscallAction::Return(EBADF);
        }

        let len = len as usize;
        if len == 0 {
            return SyscallAction::Return(0);
        }

        let process_id = self.task(current).process_id;
        let Some(file) = self.processes.file(process_id, fd) else {
            return SyscallAction::Return(EBADF);
        };

        match file.object {
            FileObject::Regular { inode } => {
                let mut chunk = [0u8; 128];
                let mut done = 0usize;
                while done < len {
                    let want = (len - done).min(chunk.len());
                    let count = match vfs::read(inode, file.offset + done, &mut chunk[..want]) {
                        Ok(count) => count,
                        Err(err) => {
                            return SyscallAction::Return(if done == 0 {
                                err
                            } else {
                                done as u32
                            });
                        }
                    };
                    if count == 0 {
                        break;
                    }
                    let dst = UserPtr::new((buf as usize).wrapping_add(done));
                    let result = unsafe { copy_to_user(dst, &chunk[..count]) };
                    if let Err(err) = result {
                        return SyscallAction::Return(if done == 0 { err } else { done as u32 });
                    }
                    done += count;
                }
                if let Some(open_file) = self.processes.file_mut(process_id, fd) {
                    open_file.offset = open_file.offset.saturating_add(done);
                }
                SyscallAction::Return(done as u32)
            }
            FileObject::Device { inode } => {
                let mut chunk = [0u8; 128];
                let want = len.min(chunk.len());
                let count = match vfs::read(inode, 0, &mut chunk[..want]) {
                    Ok(count) => count,
                    Err(err) => return SyscallAction::Return(err),
                };
                let result = unsafe { copy_to_user(UserPtr::new(buf as usize), &chunk[..count]) };
                match result {
                    Ok(()) => SyscallAction::Return(count as u32),
                    Err(err) => SyscallAction::Return(err),
                }
            }
            FileObject::Pipe {
                id,
                end: PipeEnd::Read,
            } => {
                let mut chunk = [0u8; 128];
                let want = len.min(chunk.len());
                match ipc::read(id, &mut chunk[..want]) {
                    PipeIo::Read { bytes, wake_writer } => {
                        if wake_writer {
                            self.wake_channel(ipc::write_wait_channel(id));
                        }
                        match unsafe { copy_to_user(UserPtr::new(buf as usize), &chunk[..bytes]) } {
                            Ok(()) => SyscallAction::Return(bytes as u32),
                            Err(err) => SyscallAction::Return(err),
                        }
                    }
                    PipeIo::WouldBlock { channel } => {
                        let task = self.task_mut(current);
                        task.mark_waiting_for_read(channel, buf as usize, len);
                        SyscallAction::Block { channel }
                    }
                    PipeIo::Closed => SyscallAction::Return(0),
                    PipeIo::Error(err) => SyscallAction::Return(err),
                    PipeIo::Write { .. } => SyscallAction::Return(EINVAL),
                }
            }
            FileObject::Directory { .. } => SyscallAction::Return(EISDIR),
            FileObject::Console | FileObject::ConsoleIn => {
                let mut chunk = [0u8; 128];
                let count = console::pop_into(&mut chunk[..len.min(128)]);
                if count != 0 {
                    let result =
                        unsafe { copy_to_user(UserPtr::new(buf as usize), &chunk[..count]) };
                    return match result {
                        Ok(()) => SyscallAction::Return(count as u32),
                        Err(err) => SyscallAction::Return(err),
                    };
                }

                let task = self.task_mut(current);
                task.mark_waiting_for_read(console::INPUT_WAIT_CHANNEL, buf as usize, len);
                SyscallAction::Block {
                    channel: console::INPUT_WAIT_CHANNEL,
                }
            }
            _ => SyscallAction::Return(EBADF),
        }
    }

    fn sys_writev(&mut self, current: TaskId, fd: u32, iov_ptr: u32, iovcnt: u32) -> u32 {
        if iovcnt > 16 {
            return EINVAL;
        }
        let mut total = 0usize;
        for index in 0..iovcnt as usize {
            let mut iov = UserIovec { base: 0, len: 0 };
            let bytes = unsafe {
                core::slice::from_raw_parts_mut(
                    (&mut iov as *mut UserIovec).cast::<u8>(),
                    core::mem::size_of::<UserIovec>(),
                )
            };
            let ptr = UserPtr::new(
                (iov_ptr as usize).wrapping_add(index * core::mem::size_of::<UserIovec>()),
            );
            if let Err(err) = unsafe { copy_from_user(bytes, ptr) } {
                return if total == 0 { err } else { total as u32 };
            }
            if iov.len == 0 {
                continue;
            }
            let result = self.sys_write(current, fd, iov.base, iov.len);
            if is_error(result) {
                return if total == 0 { result } else { total as u32 };
            }
            total = total.saturating_add(result as usize);
            if result < iov.len {
                break;
            }
        }
        total as u32
    }

    fn sys_readv(&mut self, current: TaskId, fd: u32, iov_ptr: u32, iovcnt: u32) -> SyscallAction {
        if iovcnt > 16 {
            return SyscallAction::Return(EINVAL);
        }
        let mut total = 0usize;
        for index in 0..iovcnt as usize {
            let mut iov = UserIovec { base: 0, len: 0 };
            let bytes = unsafe {
                core::slice::from_raw_parts_mut(
                    (&mut iov as *mut UserIovec).cast::<u8>(),
                    core::mem::size_of::<UserIovec>(),
                )
            };
            let ptr = UserPtr::new(
                (iov_ptr as usize).wrapping_add(index * core::mem::size_of::<UserIovec>()),
            );
            if let Err(err) = unsafe { copy_from_user(bytes, ptr) } {
                return SyscallAction::Return(if total == 0 { err } else { total as u32 });
            }
            if iov.len == 0 {
                continue;
            }
            match self.sys_read(current, fd, iov.base, iov.len) {
                SyscallAction::Return(result) if is_error(result) => {
                    return SyscallAction::Return(if total == 0 { result } else { total as u32 });
                }
                SyscallAction::Return(result) => {
                    total = total.saturating_add(result as usize);
                    if result < iov.len {
                        break;
                    }
                }
                SyscallAction::Block { channel } => {
                    if total == 0 {
                        return SyscallAction::Block { channel };
                    }
                    return SyscallAction::Return(total as u32);
                }
            }
        }
        SyscallAction::Return(total as u32)
    }

    fn sys_open(&mut self, current: TaskId, path_ptr: u32, path_len: u32, flags: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = path_len.min(path.len() as u32) as usize;
        let copied = if len == 0 {
            0
        } else {
            match unsafe { copy_from_user(&mut path[..len], UserPtr::new(path_ptr as usize)) } {
                Ok(()) => len,
                Err(err) => return err,
            }
        };

        self.sys_open_path(current, &path[..copied], flags)
    }

    fn sys_open_cstr(&mut self, current: TaskId, path_ptr: u32, flags: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = match crate::kernel::user::copy_cstr_from_user(
            &mut path,
            UserPtr::new(path_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        self.sys_open_path(current, &path[..len], flags)
    }

    fn sys_openat_cstr(&mut self, current: TaskId, dirfd: i32, path_ptr: u32, flags: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = match crate::kernel::user::copy_cstr_from_user(
            &mut path,
            UserPtr::new(path_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        if path[..len].starts_with(b"/") || dirfd == AT_FDCWD {
            return self.sys_open_path(current, &path[..len], flags);
        }
        let follow = flags & O_NOFOLLOW == 0;
        match self.lookup_at(current, dirfd as u32, &path[..len], follow) {
            Ok(inode) => self.open_inode(current, inode, flags),
            Err(err) => err,
        }
    }

    fn sys_open_path(&mut self, current: TaskId, path: &[u8], flags: u32) -> u32 {
        let mut resolved = [0u8; 128];
        let path = match self.resolve_path(current, path, &mut resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let inode = match self.lookup_path(path, flags & O_NOFOLLOW == 0) {
            Ok(inode) => inode,
            Err(ENOENT) if flags & O_CREAT != 0 => {
                let mode = 0o666 & !self.processes.umask(self.task(current).process_id);
                match vfs::create_file(path, mode) {
                    Ok(inode) => inode,
                    Err(err) => return err,
                }
            }
            Err(err) => return err,
        };
        self.open_inode(current, inode, flags)
    }

    fn open_inode(&mut self, current: TaskId, inode: vfs::InodeRef, flags: u32) -> u32 {
        let Some(metadata) = vfs::metadata(inode) else {
            return ENOENT;
        };
        if flags & O_DIRECTORY != 0 && metadata.file_type != vfs::FileType::Directory {
            return crate::kernel::syscall::ENOTDIR;
        }
        if flags & O_TRUNC != 0 && metadata.file_type == vfs::FileType::Regular {
            if let Err(err) = vfs::truncate(inode, 0) {
                return err;
            }
        }
        let object = match metadata.file_type {
            vfs::FileType::Regular | vfs::FileType::Symlink => FileObject::Regular { inode },
            vfs::FileType::Directory => FileObject::Directory { inode },
            vfs::FileType::Device if self.is_console_device(inode) => FileObject::Console,
            vfs::FileType::Device => FileObject::Device { inode },
        };
        match self
            .processes
            .open_file(self.task(current).process_id, object, flags)
        {
            Ok(fd) => fd as u32,
            Err(err) => err,
        }
    }

    fn sys_close(&mut self, current: TaskId, fd: u32) -> u32 {
        match self
            .processes
            .close_file(self.task(current).process_id, fd as usize)
        {
            Ok(Some(FileObject::Pipe { id, end })) => {
                self.close_file_object(FileObject::Pipe { id, end });
                0
            }
            Ok(_) => 0,
            Err(err) => err,
        }
    }

    fn sys_pipe(&mut self, current: TaskId, user_fds: u32) -> u32 {
        self.sys_pipe2(current, user_fds, 0)
    }

    fn sys_pipe2(&mut self, current: TaskId, user_fds: u32, flags: u32) -> u32 {
        if flags & !(O_CLOEXEC | crate::kernel::syscall::O_NONBLOCK) != 0 {
            return EINVAL;
        }
        let pipe = match ipc::create() {
            Ok(pipe) => pipe,
            Err(err) => return err,
        };
        let process_id = self.task(current).process_id;
        let read_fd = match self.processes.open_file(
            process_id,
            FileObject::Pipe {
                id: pipe,
                end: PipeEnd::Read,
            },
            flags,
        ) {
            Ok(fd) => fd,
            Err(err) => {
                ipc::close(pipe, PipeEnd::Read);
                ipc::close(pipe, PipeEnd::Write);
                return err;
            }
        };
        let write_fd = match self.processes.open_file(
            process_id,
            FileObject::Pipe {
                id: pipe,
                end: PipeEnd::Write,
            },
            flags,
        ) {
            Ok(fd) => fd,
            Err(err) => {
                let _ = self.processes.close_file(process_id, read_fd);
                ipc::close(pipe, PipeEnd::Read);
                ipc::close(pipe, PipeEnd::Write);
                return err;
            }
        };
        let fds = [read_fd as u32, write_fd as u32];
        let bytes = unsafe {
            core::slice::from_raw_parts(fds.as_ptr().cast::<u8>(), core::mem::size_of_val(&fds))
        };
        match unsafe { copy_to_user(UserPtr::new(user_fds as usize), bytes) } {
            Ok(()) => 0,
            Err(err) => {
                let _ = self.processes.close_file(process_id, read_fd);
                let _ = self.processes.close_file(process_id, write_fd);
                ipc::close(pipe, PipeEnd::Read);
                ipc::close(pipe, PipeEnd::Write);
                err
            }
        }
    }

    fn sys_brk(&mut self, current: TaskId, addr: usize) -> u32 {
        #[cfg(feature = "mmu")]
        {
            let process_id = self.task(current).process_id;
            let Some(space) = self.processes.address_space(process_id) else {
                return EINVAL;
            };
            if addr == 0 {
                return space.brk.as_usize() as u32;
            }
            let start = space.brk_start.as_usize();
            let old_brk = space.brk.as_usize();
            let new_brk = crate::kernel::address::align_up(addr, memory::PAGE_SIZE);
            if new_brk < start || new_brk >= USER_MMAP_BASE {
                return old_brk as u32;
            }
            if new_brk <= old_brk {
                if let Some(space) = self.processes.address_space_mut(process_id) {
                    space.brk = VirtAddr::new(new_brk);
                }
                return new_brk as u32;
            }
            let map_start = crate::kernel::address::align_up(old_brk, memory::PAGE_SIZE);
            let pages = (new_brk - map_start) / memory::PAGE_SIZE;
            if pages != 0 {
                let Some(phys) = memory::alloc_exact_pages(pages) else {
                    return old_brk as u32;
                };
                if let Err(err) = self.map_user_alloc(
                    process_id,
                    map_start,
                    phys as usize,
                    pages,
                    VM_READ | VM_WRITE | VM_USER | VM_ANON | VM_HEAP,
                    None,
                ) {
                    unsafe {
                        memory::free_exact_pages(phys, pages);
                    }
                    return err;
                }
            }
            if let Some(space) = self.processes.address_space_mut(process_id) {
                space.brk = VirtAddr::new(new_brk);
            }
            new_brk as u32
        }

        #[cfg(not(feature = "mmu"))]
        {
            let _ = (current, addr);
            EINVAL
        }
    }

    fn sys_mmap(
        &mut self,
        current: TaskId,
        addr: usize,
        len: usize,
        prot: u32,
        map_flags: u32,
        fd: i32,
        offset: usize,
    ) -> u32 {
        #[cfg(feature = "mmu")]
        {
            if len == 0 {
                return EINVAL;
            }
            if offset & (memory::PAGE_SIZE - 1) != 0 {
                return EINVAL;
            }
            let process_id = self.task(current).process_id;
            let pages =
                crate::kernel::address::align_up(len, memory::PAGE_SIZE) / memory::PAGE_SIZE;
            let Some(space) = self.processes.address_space(process_id) else {
                return EINVAL;
            };
            let fixed = map_flags & MAP_FIXED != 0;
            let base = if fixed {
                if addr == 0 || addr & (memory::PAGE_SIZE - 1) != 0 {
                    return EINVAL;
                }
                addr
            } else if addr != 0 {
                crate::kernel::address::align_up(addr, memory::PAGE_SIZE)
            } else {
                crate::kernel::address::align_up(space.mmap_next.as_usize(), memory::PAGE_SIZE)
            };
            let Some(end) = base.checked_add(pages * memory::PAGE_SIZE) else {
                return EINVAL;
            };
            if base < USER_MMAP_BASE || end > USER_MMAP_TOP {
                return EINVAL;
            }

            if fixed {
                if let Err(err) = self.unmap_fixed_range(process_id, base, pages) {
                    return err;
                }
            } else if space.find_vma(VirtAddr::new(base)).is_some()
                || (end > base && space.find_vma(VirtAddr::new(end - 1)).is_some())
            {
                return crate::kernel::syscall::EEXIST;
            }

            let (file_inode, file_offset) = if map_flags & MAP_ANONYMOUS != 0 {
                (None, 0usize)
            } else {
                if fd < 0 {
                    return EBADF;
                }
                let Some(file) = self.processes.file(process_id, fd as usize) else {
                    return EBADF;
                };
                let Some(inode) = file.object.inode() else {
                    return EBADF;
                };
                let Some(metadata) = vfs::metadata(inode) else {
                    return EBADF;
                };
                if metadata.file_type == vfs::FileType::Directory {
                    return EISDIR;
                }
                (Some(inode), offset)
            };

            let Some(phys) = memory::alloc_exact_pages(pages) else {
                return ENOMEM;
            };

            if let Some(inode) = file_inode {
                let total = pages * memory::PAGE_SIZE;
                let dst = unsafe { core::slice::from_raw_parts_mut(phys, total) };
                let mut done = 0usize;
                while done < total {
                    let count = match vfs::read(inode, file_offset + done, &mut dst[done..total]) {
                        Ok(count) => count,
                        Err(err) => {
                            unsafe {
                                memory::free_exact_pages(phys, pages);
                            }
                            return err;
                        }
                    };
                    if count == 0 {
                        break;
                    }
                    done += count;
                }
            }

            let vm_flags = flags_prot_to_vm(prot)
                | VM_USER
                | VM_MMAP
                | if file_inode.is_some() {
                    VM_FILE
                } else {
                    VM_ANON
                }
                | if fixed { VM_FIXED } else { 0 };
            match self.map_user_alloc(process_id, base, phys as usize, pages, vm_flags, file_inode)
            {
                Ok(()) => {
                    if let Some(space) = self.processes.address_space_mut(process_id) {
                        if end > space.mmap_next.as_usize() {
                            space.mmap_next = VirtAddr::new(end);
                        }
                    }
                    base as u32
                }
                Err(err) => {
                    unsafe {
                        memory::free_exact_pages(phys, pages);
                    }
                    err
                }
            }
        }

        #[cfg(not(feature = "mmu"))]
        {
            let _ = (current, addr, len, prot, map_flags, fd, offset);
            EINVAL
        }
    }

    fn sys_munmap(&mut self, current: TaskId, addr: usize, len: usize) -> u32 {
        #[cfg(feature = "mmu")]
        {
            if addr & (memory::PAGE_SIZE - 1) != 0 || len == 0 {
                return EINVAL;
            }
            let process_id = self.task(current).process_id;
            let pages =
                crate::kernel::address::align_up(len, memory::PAGE_SIZE) / memory::PAGE_SIZE;
            let Some(end) = addr.checked_add(pages * memory::PAGE_SIZE) else {
                return EINVAL;
            };
            let Some(space) = self.processes.address_space_mut(process_id) else {
                return EINVAL;
            };
            if let Err(err) = space.unmap_vma_range(VirtAddr::new(addr), VirtAddr::new(end)) {
                return err;
            }
            let region = match space.remove_owned_range(VirtAddr::new(addr), pages) {
                Ok(region) => Some(region),
                Err(_) => space.remove_owned_region(VirtAddr::new(addr), pages),
            };
            unsafe {
                crate::arch::aarch32::mmu::unmap_pages_in(
                    space.page_table_root.as_usize(),
                    addr,
                    pages,
                );
                if let Some(region) = region {
                    memory::free_exact_pages(region.phys.as_usize() as *mut u8, region.pages);
                }
            }
            0
        }

        #[cfg(not(feature = "mmu"))]
        {
            let _ = (current, addr, len);
            EINVAL
        }
    }

    #[cfg(feature = "mmu")]
    fn unmap_fixed_range(
        &mut self,
        process_id: ProcessId,
        addr: usize,
        pages: usize,
    ) -> Result<(), u32> {
        let Some(end) = addr.checked_add(pages * memory::PAGE_SIZE) else {
            return Err(EINVAL);
        };
        let Some(space) = self.processes.address_space_mut(process_id) else {
            return Err(EINVAL);
        };
        space.unmap_vma_range_lenient(VirtAddr::new(addr), VirtAddr::new(end))?;
        unsafe {
            crate::arch::aarch32::mmu::unmap_pages_in(
                space.page_table_root.as_usize(),
                addr,
                pages,
            );
        }
        let mut cursor = addr;
        while cursor < end {
            if let Ok(region) = space.remove_owned_range(VirtAddr::new(cursor), 1) {
                unsafe {
                    memory::free_exact_pages(region.phys.as_usize() as *mut u8, 1);
                }
            }
            cursor += memory::PAGE_SIZE;
        }
        Ok(())
    }

    fn sys_mprotect(&mut self, current: TaskId, addr: usize, len: usize, prot: u32) -> u32 {
        #[cfg(feature = "mmu")]
        {
            if addr & (memory::PAGE_SIZE - 1) != 0 || len == 0 {
                return EINVAL;
            }
            let process_id = self.task(current).process_id;
            let pages =
                crate::kernel::address::align_up(len, memory::PAGE_SIZE) / memory::PAGE_SIZE;
            let Some(end) = addr.checked_add(pages * memory::PAGE_SIZE) else {
                return EINVAL;
            };
            let Some(space) = self.processes.address_space_mut(process_id) else {
                return EINVAL;
            };
            let prot_flags = flags_prot_to_vm(prot);
            if let Err(err) =
                space.protect_vma_range(VirtAddr::new(addr), VirtAddr::new(end), prot_flags)
            {
                return err;
            }
            match unsafe {
                crate::arch::aarch32::mmu::protect_user_pages_in(
                    space.page_table_root.as_usize(),
                    addr,
                    pages,
                    user_mapping_from_vm(prot_flags),
                )
            } {
                Ok(()) => 0,
                Err(err) => err,
            }
        }

        #[cfg(not(feature = "mmu"))]
        {
            let _ = (current, addr, len, prot);
            EINVAL
        }
    }

    fn sys_lseek(&mut self, current: TaskId, fd: u32, offset: i32, whence: u32) -> u32 {
        let process_id = self.task(current).process_id;
        let Some(file) = self.processes.file(process_id, fd as usize) else {
            return EBADF;
        };
        let Some(inode) = file.object.inode() else {
            return ESPIPE;
        };
        let Some(metadata) = vfs::metadata(inode) else {
            return EBADF;
        };

        let base = match whence {
            SEEK_SET => 0isize,
            SEEK_CUR => file.offset as isize,
            SEEK_END => metadata.size as isize,
            _ => return EINVAL,
        };
        let new_offset = base.saturating_add(offset as isize);
        if new_offset < 0 {
            return EINVAL;
        }
        if let Some(open_file) = self.processes.file_mut(process_id, fd as usize) {
            open_file.offset = new_offset as usize;
        }
        new_offset as u32
    }

    fn sys_dup(&mut self, current: TaskId, old_fd: u32, min_fd: usize, cloexec: bool) -> u32 {
        let process_id = self.task(current).process_id;
        match self
            .processes
            .duplicate_file_from(process_id, old_fd as usize, min_fd, cloexec)
        {
            Ok(fd) => fd as u32,
            Err(err) => err,
        }
    }

    fn sys_dup2(&mut self, current: TaskId, old_fd: u32, new_fd: u32, cloexec: bool) -> u32 {
        if new_fd as usize >= crate::kernel::process::MAX_FILES {
            return EBADF;
        }
        let process_id = self.task(current).process_id;
        match self.processes.duplicate_file_to(
            process_id,
            old_fd as usize,
            new_fd as usize,
            cloexec,
        ) {
            Ok(Some(object)) => {
                self.close_file_object(object);
                new_fd
            }
            Ok(None) => new_fd,
            Err(err) => err,
        }
    }

    fn sys_dup3(&mut self, current: TaskId, old_fd: u32, new_fd: u32, flags: u32) -> u32 {
        if old_fd == new_fd {
            return EINVAL;
        }
        if flags & !O_CLOEXEC != 0 {
            return EINVAL;
        }
        self.sys_dup2(current, old_fd, new_fd, flags & O_CLOEXEC != 0)
    }

    fn sys_fstat_kernel(&mut self, current: TaskId, fd: u32, stat_ptr: u32) -> u32 {
        let process_id = self.task(current).process_id;
        let Some(file) = self.processes.file(process_id, fd as usize) else {
            return EBADF;
        };
        let Some(inode) = file.object.inode() else {
            return EINVAL;
        };
        let Some(metadata) = vfs::metadata(inode) else {
            return ENOENT;
        };
        self.copy_kernel_stat_to_user(metadata, stat_ptr)
    }

    fn sys_stat(&mut self, current: TaskId, path_ptr: u32, path_len: u32, stat_ptr: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = path_len.min(path.len() as u32) as usize;
        if len != 0 {
            if let Err(err) =
                unsafe { copy_from_user(&mut path[..len], UserPtr::new(path_ptr as usize)) }
            {
                return err;
            }
        }
        let mut resolved = [0u8; 128];
        let path = match self.resolve_path(current, &path[..len], &mut resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let inode = match vfs::lookup(path) {
            Ok(inode) => inode,
            Err(err) => return err,
        };
        let Some(metadata) = vfs::metadata(inode) else {
            return ENOENT;
        };
        self.copy_kernel_stat_to_user(metadata, stat_ptr)
    }

    fn sys_fstat(&mut self, current: TaskId, fd: u32, stat_ptr: u32, stat64: bool) -> u32 {
        let process_id = self.task(current).process_id;
        let Some(file) = self.processes.file(process_id, fd as usize) else {
            return EBADF;
        };
        let Some(inode) = file.object.inode() else {
            return EINVAL;
        };
        let Some(metadata) = vfs::metadata(inode) else {
            return ENOENT;
        };
        self.copy_linux_stat_to_user(metadata, stat_ptr, stat64)
    }

    fn sys_stat_cstr(
        &mut self,
        current: TaskId,
        path_ptr: u32,
        stat_ptr: u32,
        stat64: bool,
        nofollow: bool,
    ) -> u32 {
        let mut path = [0u8; 96];
        let len = match crate::kernel::user::copy_cstr_from_user(
            &mut path,
            UserPtr::new(path_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        let mut resolved = [0u8; 128];
        let path = match self.resolve_path(current, &path[..len], &mut resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let inode = match self.lookup_path(path, !nofollow) {
            Ok(inode) => inode,
            Err(err) => return err,
        };
        let Some(metadata) = vfs::metadata(inode) else {
            return ENOENT;
        };
        self.copy_linux_stat_to_user(metadata, stat_ptr, stat64)
    }

    fn sys_newfstatat(
        &mut self,
        current: TaskId,
        dirfd: i32,
        path_ptr: u32,
        stat_ptr: u32,
        flags: u32,
        stat64: bool,
    ) -> u32 {
        let mut path = [0u8; 96];
        let len = match crate::kernel::user::copy_cstr_from_user(
            &mut path,
            UserPtr::new(path_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        if len == 0 && flags & AT_EMPTY_PATH != 0 {
            if dirfd == AT_FDCWD {
                return EINVAL;
            }
            return self.sys_fstat(current, dirfd as u32, stat_ptr, stat64);
        }
        let follow = flags & AT_SYMLINK_NOFOLLOW == 0;
        let inode = match self.lookup_at_or_path(current, dirfd, &path[..len], follow) {
            Ok(inode) => inode,
            Err(err) => return err,
        };
        let Some(metadata) = vfs::metadata(inode) else {
            return ENOENT;
        };
        self.copy_linux_stat_to_user(metadata, stat_ptr, stat64)
    }

    fn sys_newfstatat_kernel(
        &mut self,
        current: TaskId,
        dirfd: i32,
        path_ptr: u32,
        stat_ptr: u32,
        flags: u32,
    ) -> u32 {
        let mut path = [0u8; 96];
        let len = match crate::kernel::user::copy_cstr_from_user(
            &mut path,
            UserPtr::new(path_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        if len == 0 && flags & AT_EMPTY_PATH != 0 {
            if dirfd == AT_FDCWD {
                return EINVAL;
            }
            return self.sys_fstat_kernel(current, dirfd as u32, stat_ptr);
        }
        let follow = flags & AT_SYMLINK_NOFOLLOW == 0;
        let inode = match self.lookup_at_or_path(current, dirfd, &path[..len], follow) {
            Ok(inode) => inode,
            Err(err) => return err,
        };
        let Some(metadata) = vfs::metadata(inode) else {
            return ENOENT;
        };
        self.copy_kernel_stat_to_user(metadata, stat_ptr)
    }

    fn sys_statx(
        &mut self,
        current: TaskId,
        dirfd: i32,
        path_ptr: u32,
        flags: u32,
        _mask: u32,
        statx_ptr: u32,
    ) -> u32 {
        let mut path = [0u8; 96];
        let len = match crate::kernel::user::copy_cstr_from_user(
            &mut path,
            UserPtr::new(path_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        if len == 0 && flags & AT_EMPTY_PATH != 0 {
            if dirfd == AT_FDCWD {
                return EINVAL;
            }
            let process_id = self.task(current).process_id;
            let Some(file) = self.processes.file(process_id, dirfd as usize) else {
                return EBADF;
            };
            let Some(inode) = file.object.inode() else {
                return EINVAL;
            };
            let Some(metadata) = vfs::metadata(inode) else {
                return ENOENT;
            };
            return self.copy_statx_to_user(metadata, statx_ptr);
        }
        let inode = match self.lookup_at_or_path(
            current,
            dirfd,
            &path[..len],
            flags & AT_SYMLINK_NOFOLLOW == 0,
        ) {
            Ok(inode) => inode,
            Err(err) => return err,
        };
        let Some(metadata) = vfs::metadata(inode) else {
            return ENOENT;
        };
        self.copy_statx_to_user(metadata, statx_ptr)
    }

    fn lookup_at_or_path(
        &self,
        current: TaskId,
        dirfd: i32,
        path: &[u8],
        follow: bool,
    ) -> Result<vfs::InodeRef, u32> {
        if path.starts_with(b"/") || dirfd == AT_FDCWD {
            let mut resolved = [0u8; 128];
            let path = self.resolve_path(current, path, &mut resolved)?;
            return self.lookup_path(path, follow);
        }
        self.lookup_at(current, dirfd as u32, path, follow)
    }

    fn lookup_path(&self, path: &[u8], follow: bool) -> Result<vfs::InodeRef, u32> {
        if follow {
            vfs::lookup(path)
        } else {
            vfs::lookup_nofollow(path)
        }
    }

    fn lookup_at(
        &self,
        current: TaskId,
        dirfd: u32,
        path: &[u8],
        follow: bool,
    ) -> Result<vfs::InodeRef, u32> {
        let process_id = self.task(current).process_id;
        let Some(file) = self.processes.file(process_id, dirfd as usize) else {
            return Err(EBADF);
        };
        match file.object {
            FileObject::Directory { inode } => vfs::lookup_child(inode, path, follow),
            _ => Err(EBADF),
        }
    }

    fn path_at_cstr<'a>(
        &self,
        current: TaskId,
        dirfd: i32,
        path_ptr: u32,
        path: &'a mut [u8],
        resolved: &'a mut [u8],
    ) -> Result<&'a [u8], u32> {
        let len = crate::kernel::user::copy_cstr_from_user(
            path,
            UserPtr::new(path_ptr as usize),
            path.len(),
        )?;
        if path[..len].starts_with(b"/") || dirfd == AT_FDCWD {
            return self.resolve_path(current, &path[..len], resolved);
        }
        let process_id = self.task(current).process_id;
        let Some(file) = self.processes.file(process_id, dirfd as usize) else {
            return Err(EBADF);
        };
        if !matches!(file.object, FileObject::Directory { .. }) {
            return Err(EBADF);
        }
        self.resolve_path(current, &path[..len], resolved)
    }

    fn copy_kernel_stat_to_user(&self, metadata: vfs::Metadata, stat_ptr: u32) -> u32 {
        let mut stat = [0u8; 104];
        write_u64(&mut stat[0..8], vfs::fs_dev(metadata.inode.fs));
        write_u32(&mut stat[12..16], metadata.inode.ino as u32);
        write_u32(&mut stat[16..20], metadata.linux_mode());
        write_u32(&mut stat[20..24], metadata.nlink);
        write_u32(&mut stat[24..28], metadata.uid);
        write_u32(&mut stat[28..32], metadata.gid);
        write_u64(
            &mut stat[32..40],
            if metadata.file_type == vfs::FileType::Device {
                metadata.inode.ino as u64
            } else {
                0
            },
        );
        write_i64(&mut stat[48..56], metadata.size as i64);
        write_u32(&mut stat[56..60], 4096);
        write_u64(&mut stat[64..72], metadata.size.div_ceil(512) as u64);
        write_u32(&mut stat[72..76], metadata.atime);
        write_u32(&mut stat[80..84], metadata.mtime);
        write_u32(&mut stat[88..92], metadata.ctime);
        write_u64(&mut stat[96..104], metadata.inode.ino as u64);
        match unsafe { copy_to_user(UserPtr::new(stat_ptr as usize), &stat) } {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn copy_linux_stat_to_user(&self, metadata: vfs::Metadata, stat_ptr: u32, stat64: bool) -> u32 {
        if stat64 {
            self.copy_linux_stat64_to_user(metadata, stat_ptr)
        } else {
            self.copy_linux_old_stat_to_user(metadata, stat_ptr)
        }
    }

    fn copy_linux_old_stat_to_user(&self, metadata: vfs::Metadata, stat_ptr: u32) -> u32 {
        let mut stat = [0u8; 64];
        write_u32(&mut stat[0..4], vfs::fs_dev(metadata.inode.fs) as u32);
        write_u32(&mut stat[4..8], metadata.inode.ino as u32);
        write_u16(&mut stat[8..10], metadata.linux_mode() as u16);
        write_u16(&mut stat[10..12], metadata.nlink as u16);
        write_u16(&mut stat[12..14], metadata.uid as u16);
        write_u16(&mut stat[14..16], metadata.gid as u16);
        write_u32(&mut stat[16..20], linux_rdev(metadata));
        write_u32(&mut stat[20..24], metadata.size as u32);
        write_u32(&mut stat[24..28], 4096);
        write_u32(&mut stat[28..32], metadata.size.div_ceil(512) as u32);
        write_u32(&mut stat[32..36], metadata.atime);
        write_u32(&mut stat[40..44], metadata.mtime);
        write_u32(&mut stat[48..52], metadata.ctime);
        match unsafe { copy_to_user(UserPtr::new(stat_ptr as usize), &stat) } {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn copy_linux_stat64_to_user(&self, metadata: vfs::Metadata, stat_ptr: u32) -> u32 {
        let mut stat = [0u8; 104];
        write_u64(&mut stat[0..8], vfs::fs_dev(metadata.inode.fs));
        write_u32(&mut stat[12..16], metadata.inode.ino as u32);
        write_u32(&mut stat[16..20], metadata.linux_mode());
        write_u32(&mut stat[20..24], metadata.nlink);
        write_u32(&mut stat[24..28], metadata.uid);
        write_u32(&mut stat[28..32], metadata.gid);
        write_u64(&mut stat[32..40], linux_rdev(metadata) as u64);
        write_i64(&mut stat[48..56], metadata.size as i64);
        write_u32(&mut stat[56..60], 4096);
        write_u64(&mut stat[64..72], metadata.size.div_ceil(512) as u64);
        write_u32(&mut stat[72..76], metadata.atime);
        write_u32(&mut stat[80..84], metadata.mtime);
        write_u32(&mut stat[88..92], metadata.ctime);
        write_u64(&mut stat[96..104], metadata.inode.ino as u64);
        match unsafe { copy_to_user(UserPtr::new(stat_ptr as usize), &stat) } {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn copy_statx_to_user(&self, metadata: vfs::Metadata, statx_ptr: u32) -> u32 {
        let statx = vfs::Statx::from_metadata(metadata);
        let bytes = unsafe {
            core::slice::from_raw_parts(
                (&statx as *const vfs::Statx).cast::<u8>(),
                core::mem::size_of::<vfs::Statx>(),
            )
        };
        match unsafe { copy_to_user(UserPtr::new(statx_ptr as usize), bytes) } {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn sys_access(&mut self, current: TaskId, path_ptr: u32, path_len: u32, mode: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = path_len.min(path.len() as u32) as usize;
        if len != 0 {
            if let Err(err) =
                unsafe { copy_from_user(&mut path[..len], UserPtr::new(path_ptr as usize)) }
            {
                return err;
            }
        }
        let mut resolved = [0u8; 128];
        let path = match self.resolve_path(current, &path[..len], &mut resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        self.check_access(path, mode)
    }

    fn sys_access_cstr(&mut self, current: TaskId, path_ptr: u32, mode: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = match crate::kernel::user::copy_cstr_from_user(
            &mut path,
            UserPtr::new(path_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        let mut resolved = [0u8; 128];
        let path = match self.resolve_path(current, &path[..len], &mut resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let result = self.check_access(path, mode);
        if crate::config::CONFIG_BOOT_VERBOSE {
            println!(
                "syscall: access path={} mode={:#x} -> {}",
                core::str::from_utf8(path).unwrap_or("<?>"),
                mode,
                result as i32
            );
        }
        result
    }

    fn sys_faccessat_cstr(&mut self, current: TaskId, dirfd: i32, path_ptr: u32, mode: u32) -> u32 {
        let mut path = [0u8; 96];
        let mut resolved = [0u8; 128];
        let path = match self.path_at_cstr(current, dirfd, path_ptr, &mut path, &mut resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        self.check_access(path, mode)
    }

    fn check_access(&self, path: &[u8], mode: u32) -> u32 {
        let inode = match vfs::lookup(path) {
            Ok(inode) => inode,
            Err(err) => return err,
        };
        let Some(metadata) = vfs::metadata(inode) else {
            return ENOENT;
        };
        if mode & !(R_OK | W_OK | X_OK | F_OK) != 0 {
            return EINVAL;
        }
        if mode & W_OK != 0 && metadata.mode & 0o222 == 0 {
            return EACCES;
        }
        if mode & R_OK != 0 && metadata.mode & 0o444 == 0 {
            return EACCES;
        }
        if mode & X_OK != 0 && metadata.mode & 0o111 == 0 {
            return EACCES;
        }
        0
    }

    fn sys_fcntl(&mut self, current: TaskId, fd: u32, cmd: u32, arg: u32) -> u32 {
        let process_id = self.task(current).process_id;
        match cmd {
            F_GETFD => self
                .processes
                .fd_flags(process_id, fd as usize)
                .unwrap_or(EBADF),
            F_SETFD => {
                let flags = if arg & FD_CLOEXEC_FLAG != 0 {
                    crate::kernel::process::FD_CLOEXEC
                } else {
                    0
                };
                self.processes
                    .set_fd_flags(process_id, fd as usize, flags)
                    .map(|_| 0)
                    .unwrap_or_else(|err| err)
            }
            F_GETFL => self
                .processes
                .file(process_id, fd as usize)
                .map(|file| file.flags)
                .unwrap_or(EBADF),
            F_SETFL => {
                let Some(file) = self.processes.file_mut(process_id, fd as usize) else {
                    return EBADF;
                };
                file.flags = (file.flags & !(O_APPEND | crate::kernel::syscall::O_NONBLOCK))
                    | (arg & (O_APPEND | crate::kernel::syscall::O_NONBLOCK));
                0
            }
            F_DUPFD => {
                let min_fd = arg as usize;
                if min_fd >= crate::kernel::process::MAX_FILES {
                    return EINVAL;
                }
                match self
                    .processes
                    .duplicate_file_from(process_id, fd as usize, min_fd, false)
                {
                    Ok(new_fd) => new_fd as u32,
                    Err(err) => err,
                }
            }
            F_DUPFD_CLOEXEC => {
                let min_fd = arg as usize;
                if min_fd >= crate::kernel::process::MAX_FILES {
                    return EINVAL;
                }
                match self
                    .processes
                    .duplicate_file_from(process_id, fd as usize, min_fd, true)
                {
                    Ok(new_fd) => new_fd as u32,
                    Err(err) => err,
                }
            }
            _ => ENOSYS,
        }
    }

    fn sys_ioctl(&mut self, current: TaskId, fd: u32, request: u32, arg: u32) -> u32 {
        let process_id = self.task(current).process_id;
        let Some(file) = self.processes.file(process_id, fd as usize) else {
            return EBADF;
        };
        match request {
            TIOCGWINSZ => {
                if !matches!(
                    file.object,
                    FileObject::Console
                        | FileObject::ConsoleIn
                        | FileObject::ConsoleOut
                        | FileObject::ConsoleErr
                ) {
                    return ENOTTY;
                }
                let winsize = WinSize {
                    ws_row: 25,
                    ws_col: 80,
                    ws_xpixel: 0,
                    ws_ypixel: 0,
                };
                self.copy_plain_to_user(arg, &winsize)
            }
            TCGETS => {
                if !matches!(
                    file.object,
                    FileObject::Console
                        | FileObject::ConsoleIn
                        | FileObject::ConsoleOut
                        | FileObject::ConsoleErr
                ) {
                    return ENOTTY;
                }
                let (iflag, oflag, cflag, lflag, cc) = console::termios();
                let termios = crate::kernel::syscall::Termios {
                    iflag,
                    oflag,
                    cflag,
                    lflag,
                    line: 0,
                    cc,
                };
                self.copy_plain_to_user(arg, &termios)
            }
            TCSETS | TCSETSW | TCSETSF => {
                if !matches!(
                    file.object,
                    FileObject::Console
                        | FileObject::ConsoleIn
                        | FileObject::ConsoleOut
                        | FileObject::ConsoleErr
                ) {
                    return ENOTTY;
                }
                if arg == 0 {
                    return EFAULT;
                }
                let mut termios = crate::kernel::syscall::Termios {
                    iflag: 0,
                    oflag: 0,
                    cflag: 0,
                    lflag: 0,
                    line: 0,
                    cc: [0; 19],
                };
                let bytes = unsafe {
                    core::slice::from_raw_parts_mut(
                        (&mut termios as *mut crate::kernel::syscall::Termios).cast::<u8>(),
                        core::mem::size_of::<crate::kernel::syscall::Termios>(),
                    )
                };
                if let Err(err) = unsafe { copy_from_user(bytes, UserPtr::new(arg as usize)) } {
                    return err;
                }
                console::set_termios(
                    termios.iflag,
                    termios.oflag,
                    termios.cflag,
                    termios.lflag,
                    termios.cc,
                );
                0
            }
            _ => ENOTTY,
        }
    }

    fn sys_uname(&self, uts_ptr: u32) -> u32 {
        let mut uts = UtsName::empty();
        write_nul_field(&mut uts.sysname, b"RustOS");
        write_nul_field(&mut uts.nodename, b"qemu-virt");
        write_nul_field(&mut uts.release, b"6.8.0");
        write_nul_field(&mut uts.version, b"L8");
        write_nul_field(&mut uts.machine, b"armv7l");
        write_nul_field(&mut uts.domainname, b"localdomain");
        self.copy_plain_to_user(uts_ptr, &uts)
    }

    fn sys_gettimeofday(&self, tv_ptr: u32, tz_ptr: u32) -> u32 {
        if tv_ptr != 0 {
            let ticks = self.ticks;
            let tv = TimeVal {
                tv_sec: (ticks / 100) as i32,
                tv_usec: ((ticks % 100) * 10_000) as i32,
            };
            let result = self.copy_plain_to_user(tv_ptr, &tv);
            if result != 0 {
                return result;
            }
        }
        if tz_ptr != 0 {
            let tz = [0u32; 2];
            let bytes = unsafe {
                core::slice::from_raw_parts(tz.as_ptr().cast::<u8>(), core::mem::size_of_val(&tz))
            };
            if let Err(err) = unsafe { copy_to_user(UserPtr::new(tz_ptr as usize), bytes) } {
                return err;
            }
        }
        0
    }

    fn sys_clock_gettime(&self, clock_id: u32, ts_ptr: u32) -> u32 {
        if clock_id != CLOCK_REALTIME && clock_id != CLOCK_MONOTONIC {
            return EINVAL;
        }
        let ticks = self.ticks;
        let ts = TimeSpec {
            tv_sec: (ticks / 100) as i32,
            tv_nsec: ((ticks % 100) * 10_000_000) as i32,
        };
        self.copy_plain_to_user(ts_ptr, &ts)
    }

    fn sys_clock_gettime64(&self, clock_id: u32, ts_ptr: u32) -> u32 {
        if clock_id != CLOCK_REALTIME && clock_id != CLOCK_MONOTONIC {
            return EINVAL;
        }
        let ticks = self.ticks;
        let ts = TimeSpec64 {
            tv_sec: (ticks / TICKS_PER_SECOND) as i64,
            tv_nsec: ((ticks % TICKS_PER_SECOND) * 1_000_000_000 / TICKS_PER_SECOND) as i64,
        };
        self.copy_plain_to_user(ts_ptr, &ts)
    }

    fn sys_clock_getres(&self, clock_id: u32, ts_ptr: u32) -> u32 {
        if clock_id != CLOCK_REALTIME && clock_id != CLOCK_MONOTONIC {
            return EINVAL;
        }
        if ts_ptr == 0 {
            return 0;
        }
        let ts = TimeSpec {
            tv_sec: 0,
            tv_nsec: (1_000_000_000 / TICKS_PER_SECOND) as i32,
        };
        self.copy_plain_to_user(ts_ptr, &ts)
    }

    fn sys_clock_getres64(&self, clock_id: u32, ts_ptr: u32) -> u32 {
        if clock_id != CLOCK_REALTIME && clock_id != CLOCK_MONOTONIC {
            return EINVAL;
        }
        if ts_ptr == 0 {
            return 0;
        }
        let ts = TimeSpec64 {
            tv_sec: 0,
            tv_nsec: (1_000_000_000 / TICKS_PER_SECOND) as i64,
        };
        self.copy_plain_to_user(ts_ptr, &ts)
    }

    fn sleep_timespec(
        &mut self,
        current: TaskId,
        req_ptr: u32,
        rem_ptr: u32,
        time64: bool,
    ) -> Result<(), u32> {
        let _ = rem_ptr;
        if req_ptr == 0 {
            return Err(EFAULT);
        }
        let ticks = if time64 {
            let mut ts = TimeSpec64 {
                tv_sec: 0,
                tv_nsec: 0,
            };
            let bytes = unsafe {
                core::slice::from_raw_parts_mut(
                    (&mut ts as *mut TimeSpec64).cast::<u8>(),
                    core::mem::size_of::<TimeSpec64>(),
                )
            };
            unsafe { copy_from_user(bytes, UserPtr::new(req_ptr as usize)) }?;
            if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1_000_000_000 {
                return Err(EINVAL);
            }
            timespec_to_ticks(ts.tv_sec as u64, ts.tv_nsec as u64)
        } else {
            let mut ts = TimeSpec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            let bytes = unsafe {
                core::slice::from_raw_parts_mut(
                    (&mut ts as *mut TimeSpec).cast::<u8>(),
                    core::mem::size_of::<TimeSpec>(),
                )
            };
            unsafe { copy_from_user(bytes, UserPtr::new(req_ptr as usize)) }?;
            if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1_000_000_000 {
                return Err(EINVAL);
            }
            timespec_to_ticks(ts.tv_sec as u64, ts.tv_nsec as u64)
        };

        let wake_at = self.ticks.wrapping_add(ticks.max(1));
        self.task_mut(current).mark_sleeping(wake_at);
        self.sleep.insert(current, wake_at);
        Ok(())
    }

    fn sys_getrlimit(&self, resource: u32, rlim_ptr: u32) -> u32 {
        let rlimit = match resource {
            RLIMIT_STACK => RLimit {
                rlim_cur: 8 * 1024 * 1024,
                rlim_max: 8 * 1024 * 1024,
            },
            RLIMIT_NOFILE => RLimit {
                rlim_cur: crate::kernel::process::MAX_FILES as u32,
                rlim_max: crate::kernel::process::MAX_FILES as u32,
            },
            _ => RLimit {
                rlim_cur: crate::kernel::syscall::RLIM_INFINITY,
                rlim_max: crate::kernel::syscall::RLIM_INFINITY,
            },
        };
        self.copy_plain_to_user(rlim_ptr, &rlimit)
    }

    fn sys_getgroups(&self, size: u32, list_ptr: u32) -> u32 {
        if size == 0 {
            return 0;
        }
        if list_ptr == 0 {
            return EFAULT;
        }
        0
    }

    fn sys_times(&self, tms_ptr: u32) -> u32 {
        if tms_ptr != 0 {
            let ticks = self.ticks as i32;
            let tms = Tms {
                tms_utime: ticks,
                tms_stime: 0,
                tms_cutime: 0,
                tms_cstime: 0,
            };
            let result = self.copy_plain_to_user(tms_ptr, &tms);
            if result != 0 {
                return result;
            }
        }
        self.ticks as u32
    }

    fn sys_getrusage(&self, _who: u32, rusage_ptr: u32) -> u32 {
        if rusage_ptr == 0 {
            return EFAULT;
        }
        let zero = [0u8; 72];
        match unsafe { copy_to_user(UserPtr::new(rusage_ptr as usize), &zero) } {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn sys_sysinfo(&self, info_ptr: u32) -> u32 {
        let page_size = memory::PAGE_SIZE as u32;
        let info = SysInfo {
            uptime: (self.ticks / TICKS_PER_SECOND) as i32,
            loads: [0; 3],
            totalram: (crate::config::CONFIG_QEMU_MEMORY_MB as u32) * 1024 * 1024,
            freeram: (memory::free_page_count() as u32).saturating_mul(page_size),
            sharedram: 0,
            bufferram: 0,
            totalswap: 0,
            freeswap: 0,
            procs: self.count as u16,
            pad: 0,
            totalhigh: 0,
            freehigh: 0,
            mem_unit: 1,
            reserved: [0; 8],
        };
        self.copy_plain_to_user(info_ptr, &info)
    }

    fn sys_statfs_cstr(&mut self, current: TaskId, path_ptr: u32, size: u32, buf: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = match crate::kernel::user::copy_cstr_from_user(
            &mut path,
            UserPtr::new(path_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        let mut resolved = [0u8; 128];
        let path = match self.resolve_path(current, &path[..len], &mut resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        if let Err(err) = vfs::lookup(path) {
            return err;
        }
        self.copy_statfs_to_user(size, buf)
    }

    fn sys_fstatfs(&mut self, current: TaskId, fd: u32, size: u32, buf: u32) -> u32 {
        let process_id = self.task(current).process_id;
        if self.processes.file(process_id, fd as usize).is_none() {
            return EBADF;
        }
        self.copy_statfs_to_user(size, buf)
    }

    fn copy_statfs_to_user(&self, size: u32, buf: u32) -> u32 {
        if buf == 0 {
            return EFAULT;
        }
        let statfs = StatFs64 {
            f_type: 0xef53,
            f_bsize: memory::PAGE_SIZE as u32,
            f_blocks: (crate::config::CONFIG_QEMU_MEMORY_MB as u64 * 1024 * 1024)
                / memory::PAGE_SIZE as u64,
            f_bfree: memory::free_page_count() as u64,
            f_bavail: memory::free_page_count() as u64,
            f_files: 4096,
            f_ffree: 2048,
            f_fsid: [0; 2],
            f_namelen: vfs::DIR_NAME_LEN as u32,
            f_frsize: memory::PAGE_SIZE as u32,
            f_flags: 0,
            f_spare: [0; 4],
        };
        let bytes = unsafe {
            core::slice::from_raw_parts(
                (&statfs as *const StatFs64).cast::<u8>(),
                core::mem::size_of::<StatFs64>(),
            )
        };
        let len = (size as usize).min(bytes.len());
        match unsafe { copy_to_user(UserPtr::new(buf as usize), &bytes[..len]) } {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn sys_futex(&mut self, _uaddr: u32, op: u32, _val: u32) -> u32 {
        match op & !crate::kernel::syscall::FUTEX_PRIVATE_FLAG {
            FUTEX_WAKE => 0,
            FUTEX_WAIT => EAGAIN,
            _ => ENOSYS,
        }
    }

    fn sys_getrandom(&self, buf: u32, len: u32, _flags: u32) -> u32 {
        if len == 0 {
            return 0;
        }
        if buf == 0 {
            return EFAULT;
        }
        let mut done = 0usize;
        let mut chunk = [0u8; 128];
        let len = len as usize;
        while done < len {
            let count = (len - done).min(chunk.len());
            fill_random(&mut chunk[..count]);
            let dst = UserPtr::new((buf as usize).wrapping_add(done));
            if let Err(err) = unsafe { copy_to_user(dst, &chunk[..count]) } {
                return if done == 0 { err } else { done as u32 };
            }
            done += count;
        }
        done as u32
    }

    fn sys_kill(&mut self, current: TaskId, pid: i32, sig: i32) -> u32 {
        if sig == 0 {
            return if self.find_task_by_pid(pid).is_some() {
                0
            } else {
                ESRCH
            };
        }
        let target = if pid == 0 {
            Some(current)
        } else {
            self.find_task_by_pid(pid)
        };
        let Some(task_id) = target else {
            return ESRCH;
        };
        match sig {
            SIGINT | SIGTERM => {
                self.interrupt_task(task_id, sig);
                0
            }
            _ => 0,
        }
    }

    fn sys_getcwd(&self, current: TaskId, buf: u32, size: u32) -> u32 {
        let process_id = self.task(current).process_id;
        let Some(cwd) = self.processes.cwd(process_id) else {
            return EINVAL;
        };
        let needed = cwd.len() + 1;
        if buf == 0 || size as usize == 0 {
            return EFAULT;
        }
        if needed > size as usize {
            return ERANGE;
        }
        let mut tmp = [0u8; crate::kernel::process::CWD_MAX];
        tmp[..cwd.len()].copy_from_slice(cwd);
        tmp[cwd.len()] = 0;
        match unsafe { copy_to_user(UserPtr::new(buf as usize), &tmp[..needed]) } {
            Ok(()) => buf,
            Err(err) => err,
        }
    }

    fn sys_chdir(&mut self, current: TaskId, path_ptr: u32, path_len: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = path_len.min(path.len() as u32) as usize;
        if len != 0 {
            if let Err(err) =
                unsafe { copy_from_user(&mut path[..len], UserPtr::new(path_ptr as usize)) }
            {
                return err;
            }
        }
        self.chdir_path(current, &path[..len])
    }

    fn sys_chdir_cstr(&mut self, current: TaskId, path_ptr: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = match crate::kernel::user::copy_cstr_from_user(
            &mut path,
            UserPtr::new(path_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        self.chdir_path(current, &path[..len])
    }

    fn chdir_path(&mut self, current: TaskId, path: &[u8]) -> u32 {
        let mut resolved = [0u8; crate::kernel::process::CWD_MAX];
        let path = match self.resolve_path(current, path, &mut resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let inode = match vfs::lookup(path) {
            Ok(inode) => inode,
            Err(err) => return err,
        };
        let Some(metadata) = vfs::metadata(inode) else {
            return ENOENT;
        };
        if metadata.file_type != vfs::FileType::Directory {
            return crate::kernel::syscall::ENOTDIR;
        }
        match self.processes.set_cwd(self.task(current).process_id, path) {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn sys_readlink(&self, current: TaskId, path_ptr: u32, path_len: u32, buf: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = path_len.min(path.len() as u32) as usize;
        if len != 0 {
            if let Err(err) =
                unsafe { copy_from_user(&mut path[..len], UserPtr::new(path_ptr as usize)) }
            {
                return err;
            }
        }
        self.readlink_path(current, &path[..len], buf, 96)
    }

    fn sys_readlink_cstr(&self, current: TaskId, path_ptr: u32, buf: u32, buf_size: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = match crate::kernel::user::copy_cstr_from_user(
            &mut path,
            UserPtr::new(path_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        self.readlink_path(current, &path[..len], buf, buf_size)
    }

    fn sys_readlinkat_cstr(
        &self,
        current: TaskId,
        dirfd: i32,
        path_ptr: u32,
        buf: u32,
        buf_size: u32,
    ) -> u32 {
        let mut path = [0u8; 96];
        let len = match crate::kernel::user::copy_cstr_from_user(
            &mut path,
            UserPtr::new(path_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        if path[..len].starts_with(b"/") || dirfd == AT_FDCWD {
            return self.readlink_path(current, &path[..len], buf, buf_size);
        }
        match self.lookup_at(current, dirfd as u32, &path[..len], false) {
            Ok(inode) => self.readlink_inode(inode, buf, buf_size),
            Err(err) => err,
        }
    }

    fn readlink_path(&self, current: TaskId, path: &[u8], buf: u32, buf_size: u32) -> u32 {
        let mut resolved = [0u8; 128];
        let path = match self.resolve_path(current, path, &mut resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let inode = match vfs::lookup_nofollow(path) {
            Ok(inode) => inode,
            Err(err) => return err,
        };
        self.readlink_inode(inode, buf, buf_size)
    }

    fn readlink_inode(&self, inode: vfs::InodeRef, buf: u32, buf_size: u32) -> u32 {
        let mut link = [0u8; 128];
        let count = match vfs::readlink_inode(inode, &mut link[..buf_size.min(128) as usize]) {
            Ok(count) => count,
            Err(err) => return err,
        };
        match unsafe { copy_to_user(UserPtr::new(buf as usize), &link[..count]) } {
            Ok(()) => count as u32,
            Err(err) => err,
        }
    }

    fn sys_poll(&mut self, current: TaskId, fds_ptr: u32, nfds: u32, _timeout_ms: i32) -> u32 {
        self.poll_console_input();
        if nfds > 32 {
            return EINVAL;
        }
        let mut ready = 0u32;
        for index in 0..nfds as usize {
            let mut pollfd = PollFd {
                fd: -1,
                events: 0,
                revents: 0,
            };
            let bytes = unsafe {
                core::slice::from_raw_parts_mut(
                    (&mut pollfd as *mut PollFd).cast::<u8>(),
                    core::mem::size_of::<PollFd>(),
                )
            };
            let ptr = UserPtr::new(
                (fds_ptr as usize).wrapping_add(index * core::mem::size_of::<PollFd>()),
            );
            if let Err(err) = unsafe { copy_from_user(bytes, ptr) } {
                return err;
            }
            if pollfd.fd < 0 {
                continue;
            }
            pollfd.revents = self.poll_fd_events(current, pollfd.fd as usize, pollfd.events);
            if pollfd.revents != 0 {
                ready += 1;
            }
            let out = unsafe {
                core::slice::from_raw_parts(
                    (&pollfd as *const PollFd).cast::<u8>(),
                    core::mem::size_of::<PollFd>(),
                )
            };
            if let Err(err) = unsafe { copy_to_user(ptr, out) } {
                return err;
            }
        }
        ready
    }

    fn sys_poll_for_task(
        &mut self,
        task_id: TaskId,
        fds_ptr: u32,
        nfds: u32,
        timeout_ms: i32,
    ) -> u32 {
        #[cfg(feature = "mmu")]
        {
            let restore = self.current;
            self.switch_address_space(task_id);
            let result = self.sys_poll(task_id, fds_ptr, nfds, timeout_ms);
            if let Some(current) = restore {
                self.switch_address_space(current);
            }
            result
        }

        #[cfg(not(feature = "mmu"))]
        {
            self.sys_poll(task_id, fds_ptr, nfds, timeout_ms)
        }
    }

    fn poll_waits_for_console(&self, current: TaskId, fds_ptr: u32, nfds: u32) -> bool {
        if nfds > 32 {
            return false;
        }
        for index in 0..nfds as usize {
            let mut pollfd = PollFd {
                fd: -1,
                events: 0,
                revents: 0,
            };
            let bytes = unsafe {
                core::slice::from_raw_parts_mut(
                    (&mut pollfd as *mut PollFd).cast::<u8>(),
                    core::mem::size_of::<PollFd>(),
                )
            };
            let ptr = UserPtr::new(
                (fds_ptr as usize).wrapping_add(index * core::mem::size_of::<PollFd>()),
            );
            if unsafe { copy_from_user(bytes, ptr) }.is_err() {
                return false;
            }
            if pollfd.fd < 0 || pollfd.events & POLLIN == 0 {
                continue;
            }
            let process_id = self.task(current).process_id;
            let Some(file) = self.processes.file(process_id, pollfd.fd as usize) else {
                continue;
            };
            if matches!(file.object, FileObject::Console | FileObject::ConsoleIn) {
                return true;
            }
        }
        false
    }

    fn sys_select(
        &mut self,
        current: TaskId,
        nfds: u32,
        readfds: u32,
        writefds: u32,
        exceptfds: u32,
        _timeout: u32,
    ) -> u32 {
        self.poll_console_input();
        if nfds > 1024 {
            return EINVAL;
        }
        let mut ready = 0u32;
        ready += self.select_set(current, nfds, readfds, POLLIN, true);
        ready += self.select_set(current, nfds, writefds, POLLOUT, true);
        if exceptfds != 0 {
            let clear = [0u8; core::mem::size_of::<crate::kernel::syscall::FdSet32>()];
            if let Err(err) = unsafe { copy_to_user(UserPtr::new(exceptfds as usize), &clear) } {
                return err;
            }
        }
        ready
    }

    fn select_set(
        &self,
        current: TaskId,
        nfds: u32,
        set_ptr: u32,
        event: i16,
        clear_absent: bool,
    ) -> u32 {
        if set_ptr == 0 {
            return 0;
        }
        let mut set = crate::kernel::syscall::FdSet32 { bits: [0; 32] };
        let bytes = unsafe {
            core::slice::from_raw_parts_mut(
                (&mut set as *mut crate::kernel::syscall::FdSet32).cast::<u8>(),
                core::mem::size_of::<crate::kernel::syscall::FdSet32>(),
            )
        };
        if unsafe { copy_from_user(bytes, UserPtr::new(set_ptr as usize)) }.is_err() {
            return 0;
        }
        let mut count = 0u32;
        for fd in 0..nfds as usize {
            let word = fd / 32;
            let bit = fd % 32;
            if set.bits[word] & (1u32 << bit) == 0 {
                continue;
            }
            if self.poll_fd_events(current, fd, event) & event != 0 {
                count += 1;
            } else if clear_absent {
                set.bits[word] &= !(1u32 << bit);
            }
        }
        let out = unsafe {
            core::slice::from_raw_parts(
                (&set as *const crate::kernel::syscall::FdSet32).cast::<u8>(),
                core::mem::size_of::<crate::kernel::syscall::FdSet32>(),
            )
        };
        let _ = unsafe { copy_to_user(UserPtr::new(set_ptr as usize), out) };
        count
    }

    fn poll_fd_events(&self, current: TaskId, fd: usize, requested: i16) -> i16 {
        let process_id = self.task(current).process_id;
        let Some(file) = self.processes.file(process_id, fd) else {
            return POLLNVAL;
        };
        let mut revents = 0i16;
        if requested & POLLIN != 0 && file.object.can_read() {
            revents |= match file.object {
                FileObject::Console | FileObject::ConsoleIn if console::available() == 0 => 0,
                FileObject::Pipe {
                    id,
                    end: PipeEnd::Read,
                } if ipc::available(id) == 0 => 0,
                _ => POLLIN,
            };
        }
        if requested & POLLOUT != 0 && file.object.can_write() {
            revents |= POLLOUT;
        }
        if matches!(file.object, FileObject::Closed) {
            revents |= POLLERR | POLLHUP;
        }
        revents
    }

    fn copy_plain_to_user<T>(&self, ptr: u32, value: &T) -> u32 {
        if ptr == 0 {
            return EFAULT;
        }
        let bytes = unsafe {
            core::slice::from_raw_parts((value as *const T).cast::<u8>(), core::mem::size_of::<T>())
        };
        match unsafe { copy_to_user(UserPtr::new(ptr as usize), bytes) } {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn resolve_path<'a>(
        &self,
        current: TaskId,
        path: &'a [u8],
        out: &'a mut [u8],
    ) -> Result<&'a [u8], u32> {
        let path = trim_nul(path);
        if path.is_empty() {
            return Err(ENOENT);
        }
        if path.starts_with(b"/") {
            let len = normalize_user_path(path, out);
            return Ok(&out[..len]);
        }

        let cwd = self
            .processes
            .cwd(self.task(current).process_id)
            .ok_or(EINVAL)?;
        let mut len = 0usize;
        for byte in cwd {
            if len >= out.len() {
                return Err(ERANGE);
            }
            out[len] = *byte;
            len += 1;
        }
        if len == 0 {
            if out.is_empty() {
                return Err(ERANGE);
            }
            out[0] = b'/';
            len = 1;
        }
        if len > 1 {
            if len >= out.len() {
                return Err(ERANGE);
            }
            out[len] = b'/';
            len += 1;
        }
        for byte in path {
            if len >= out.len() {
                return Err(ERANGE);
            }
            out[len] = *byte;
            len += 1;
        }
        Ok(&out[..len])
    }

    fn is_console_device(&self, inode: vfs::InodeRef) -> bool {
        if inode.fs != vfs::FileSystemId::Devfs {
            return false;
        }
        vfs::open(inode)
            .map(|file| file.name == "console" || file.name == "tty")
            .unwrap_or(false)
    }

    fn bump_forked_file_refs(&self, pid: ProcessId) {
        let _ = self.processes.for_each_file_object(pid, |object| {
            if let FileObject::Pipe { id, end } = object {
                ipc::add_ref(id, end);
            }
        });
    }

    fn sys_getdents(&mut self, current: TaskId, fd: u32, buf: u32, len: u32) -> u32 {
        let process_id = self.task(current).process_id;
        let Some(file) = self.processes.file(process_id, fd as usize) else {
            return EBADF;
        };
        let FileObject::Directory { inode } = file.object else {
            return EINVAL;
        };

        if len < 12 {
            return 0;
        }

        let mut entries = [vfs::DirEntry::empty(); 8];
        let count = match vfs::read_dir(inode, file.offset, &mut entries) {
            Ok(count) => count,
            Err(err) => return err,
        };
        let mut written = 0usize;
        let mut consumed = 0usize;
        let mut record = [0u8; 160];
        for entry in entries.iter().take(count) {
            let name_len = entry.name_len.min(vfs::DIR_NAME_LEN);
            let reclen = (10 + name_len + 2 + 3) & !3;
            if written + reclen > len as usize {
                break;
            }
            record[..reclen].fill(0);
            write_u32(&mut record[0..4], entry.inode.ino as u32);
            write_u32(&mut record[4..8], (file.offset + consumed + 1) as u32);
            write_u16(&mut record[8..10], reclen as u16);
            record[10..10 + name_len].copy_from_slice(&entry.name[..name_len]);
            record[10 + name_len] = 0;
            record[reclen - 1] = linux_dtype(entry.file_type);
            if let Err(err) = unsafe {
                copy_to_user(
                    UserPtr::new((buf as usize).wrapping_add(written)),
                    &record[..reclen],
                )
            } {
                return if written == 0 { err } else { written as u32 };
            }
            written += reclen;
            consumed += 1;
        }
        if let Some(open_file) = self.processes.file_mut(process_id, fd as usize) {
            open_file.offset = open_file.offset.saturating_add(consumed);
        }
        written as u32
    }

    fn sys_getdents64(&mut self, current: TaskId, fd: u32, buf: u32, len: u32) -> u32 {
        let process_id = self.task(current).process_id;
        let Some(file) = self.processes.file(process_id, fd as usize) else {
            return EBADF;
        };
        let FileObject::Directory { inode } = file.object else {
            return EINVAL;
        };

        if len < 24 {
            return 0;
        }

        let mut entries = [vfs::DirEntry::empty(); 8];
        let count = match vfs::read_dir(inode, file.offset, &mut entries) {
            Ok(count) => count,
            Err(err) => return err,
        };
        let mut written = 0usize;
        let mut consumed = 0usize;
        let mut record = [0u8; 160];
        for entry in entries.iter().take(count) {
            let name_len = entry.name_len.min(vfs::DIR_NAME_LEN);
            let reclen = align_dirent64_len(19 + name_len + 1);
            if written + reclen > len as usize {
                break;
            }
            record[..reclen].fill(0);
            write_u64(&mut record[0..8], entry.inode.ino as u64);
            write_i64(&mut record[8..16], (file.offset + consumed + 1) as i64);
            write_u16(&mut record[16..18], reclen as u16);
            record[18] = linux_dtype(entry.file_type);
            record[19..19 + name_len].copy_from_slice(&entry.name[..name_len]);
            record[19 + name_len] = 0;
            if let Err(err) = unsafe {
                copy_to_user(
                    UserPtr::new((buf as usize).wrapping_add(written)),
                    &record[..reclen],
                )
            } {
                return if written == 0 { err } else { written as u32 };
            }
            written += reclen;
            consumed += 1;
        }
        if let Some(open_file) = self.processes.file_mut(process_id, fd as usize) {
            open_file.offset = open_file.offset.saturating_add(consumed);
        }
        written as u32
    }

    fn sys_mkdir(&mut self, current: TaskId, path_ptr: u32, path_len: u32, mode: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = path_len.min(path.len() as u32) as usize;
        if len != 0 {
            if let Err(err) =
                unsafe { copy_from_user(&mut path[..len], UserPtr::new(path_ptr as usize)) }
            {
                return err;
            }
        }
        self.mkdir_path(current, &path[..len], mode)
    }

    fn sys_mkdir_cstr(&mut self, current: TaskId, path_ptr: u32, mode: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = match crate::kernel::user::copy_cstr_from_user(
            &mut path,
            UserPtr::new(path_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        self.mkdir_path(current, &path[..len], mode)
    }

    fn sys_mkdirat_cstr(&mut self, current: TaskId, dirfd: i32, path_ptr: u32, mode: u32) -> u32 {
        let mut path = [0u8; 96];
        let mut resolved = [0u8; 128];
        let path = match self.path_at_cstr(current, dirfd, path_ptr, &mut path, &mut resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let mode = (mode as u16) & !self.processes.umask(self.task(current).process_id);
        match vfs::mkdir(path, mode) {
            Ok(inode) => inode.ino as u32,
            Err(err) => err,
        }
    }

    fn sys_mknodat_cstr(&mut self, current: TaskId, dirfd: i32, path_ptr: u32, mode: u32) -> u32 {
        let mut path = [0u8; 96];
        let mut resolved = [0u8; 128];
        let path = match self.path_at_cstr(current, dirfd, path_ptr, &mut path, &mut resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let mode = (mode as u16) & !self.processes.umask(self.task(current).process_id);
        match vfs::create_file(path, mode) {
            Ok(_) => 0,
            Err(err) => err,
        }
    }

    fn mkdir_path(&mut self, current: TaskId, path: &[u8], mode: u32) -> u32 {
        let mut resolved = [0u8; 128];
        let path = match self.resolve_path(current, path, &mut resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let mode = (mode as u16) & !self.processes.umask(self.task(current).process_id);
        match vfs::mkdir(path, mode) {
            Ok(inode) => inode.ino as u32,
            Err(err) => err,
        }
    }

    fn sys_unlink(&mut self, current: TaskId, path_ptr: u32, path_len: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = path_len.min(path.len() as u32) as usize;
        if len != 0 {
            if let Err(err) =
                unsafe { copy_from_user(&mut path[..len], UserPtr::new(path_ptr as usize)) }
            {
                return err;
            }
        }
        self.unlink_path(current, &path[..len])
    }

    fn sys_unlink_cstr(&mut self, current: TaskId, path_ptr: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = match crate::kernel::user::copy_cstr_from_user(
            &mut path,
            UserPtr::new(path_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        self.unlink_path(current, &path[..len])
    }

    fn unlink_path(&mut self, current: TaskId, path: &[u8]) -> u32 {
        let mut resolved = [0u8; 128];
        let path = match self.resolve_path(current, path, &mut resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        match vfs::unlink(path) {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn sys_unlinkat_cstr(&mut self, current: TaskId, dirfd: i32, path_ptr: u32, flags: u32) -> u32 {
        if flags & !AT_REMOVEDIR != 0 {
            return EINVAL;
        }
        let mut path = [0u8; 96];
        let mut resolved = [0u8; 128];
        let path = match self.path_at_cstr(current, dirfd, path_ptr, &mut path, &mut resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        match vfs::unlink(path) {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn sys_link_cstr(&mut self, current: TaskId, old_ptr: u32, new_ptr: u32) -> u32 {
        self.linkat_path(current, AT_FDCWD, old_ptr, AT_FDCWD, new_ptr)
    }

    fn sys_linkat_cstr(
        &mut self,
        current: TaskId,
        old_dirfd: i32,
        old_ptr: u32,
        new_dirfd: i32,
        new_ptr: u32,
        flags: u32,
    ) -> u32 {
        if flags & !AT_SYMLINK_FOLLOW != 0 {
            return EINVAL;
        }
        self.linkat_path(current, old_dirfd, old_ptr, new_dirfd, new_ptr)
    }

    fn linkat_path(
        &mut self,
        current: TaskId,
        old_dirfd: i32,
        old_ptr: u32,
        new_dirfd: i32,
        new_ptr: u32,
    ) -> u32 {
        let mut old = [0u8; 96];
        let mut old_resolved = [0u8; 128];
        let old_path =
            match self.path_at_cstr(current, old_dirfd, old_ptr, &mut old, &mut old_resolved) {
                Ok(path) => path,
                Err(err) => return err,
            };
        let mut new = [0u8; 96];
        let mut new_resolved = [0u8; 128];
        let new_path =
            match self.path_at_cstr(current, new_dirfd, new_ptr, &mut new, &mut new_resolved) {
                Ok(path) => path,
                Err(err) => return err,
            };
        match vfs::link(old_path, new_path) {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn sys_rename(&mut self, current: TaskId, old_ptr: u32, new_ptr: u32) -> u32 {
        let mut old_path = [0u8; 96];
        let mut new_path = [0u8; 96];
        let old_len = match crate::kernel::user::copy_cstr_from_user(
            &mut old_path,
            UserPtr::new(old_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        let new_len = match crate::kernel::user::copy_cstr_from_user(
            &mut new_path,
            UserPtr::new(new_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        let mut old_resolved = [0u8; 128];
        let old_path = match self.resolve_path(current, &old_path[..old_len], &mut old_resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let mut new_resolved = [0u8; 128];
        let new_path = match self.resolve_path(current, &new_path[..new_len], &mut new_resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        match vfs::rename(old_path, new_path) {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn sys_renameat_cstr(
        &mut self,
        current: TaskId,
        old_dirfd: i32,
        old_ptr: u32,
        new_dirfd: i32,
        new_ptr: u32,
    ) -> u32 {
        self.renameat_path(current, old_dirfd, old_ptr, new_dirfd, new_ptr)
    }

    fn sys_renameat2_cstr(
        &mut self,
        current: TaskId,
        old_dirfd: i32,
        old_ptr: u32,
        new_dirfd: i32,
        new_ptr: u32,
        flags: u32,
    ) -> u32 {
        if flags != 0 {
            return EINVAL;
        }
        self.renameat_path(current, old_dirfd, old_ptr, new_dirfd, new_ptr)
    }

    fn renameat_path(
        &mut self,
        current: TaskId,
        old_dirfd: i32,
        old_ptr: u32,
        new_dirfd: i32,
        new_ptr: u32,
    ) -> u32 {
        let mut old = [0u8; 96];
        let mut old_resolved = [0u8; 128];
        let old_path =
            match self.path_at_cstr(current, old_dirfd, old_ptr, &mut old, &mut old_resolved) {
                Ok(path) => path,
                Err(err) => return err,
            };
        let mut new = [0u8; 96];
        let mut new_resolved = [0u8; 128];
        let new_path =
            match self.path_at_cstr(current, new_dirfd, new_ptr, &mut new, &mut new_resolved) {
                Ok(path) => path,
                Err(err) => return err,
            };
        match vfs::rename(old_path, new_path) {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn sys_creat_cstr(&mut self, current: TaskId, path_ptr: u32, _mode: u32) -> u32 {
        self.sys_open_cstr(
            current,
            path_ptr,
            O_CREAT | O_TRUNC | crate::kernel::syscall::O_WRONLY,
        )
    }

    fn sys_truncate_cstr(&mut self, current: TaskId, path_ptr: u32, size: usize) -> u32 {
        let mut path = [0u8; 96];
        let len = match crate::kernel::user::copy_cstr_from_user(
            &mut path,
            UserPtr::new(path_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };
        let mut resolved = [0u8; 128];
        let path = match self.resolve_path(current, &path[..len], &mut resolved) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let inode = match vfs::lookup(path) {
            Ok(inode) => inode,
            Err(err) => return err,
        };
        match vfs::truncate(inode, size) {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn sys_ftruncate(&mut self, current: TaskId, fd: u32, size: usize) -> u32 {
        let process_id = self.task(current).process_id;
        let Some(file) = self.processes.file(process_id, fd as usize) else {
            return EBADF;
        };
        let Some(inode) = file.object.inode() else {
            return EINVAL;
        };
        match vfs::truncate(inode, size) {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn sys_llseek(
        &mut self,
        current: TaskId,
        fd: u32,
        offset_high: u32,
        offset_low: u32,
        result_ptr: u32,
        whence: u32,
    ) -> u32 {
        let offset = ((offset_high as u64) << 32) | u64::from(offset_low);
        if offset > i32::MAX as u64 {
            return EINVAL;
        }
        let result = self.sys_lseek(current, fd, offset as i32, whence);
        if is_error(result) {
            return result;
        }
        let value = result as u64;
        let bytes = value.to_le_bytes();
        match unsafe { copy_to_user(UserPtr::new(result_ptr as usize), &bytes) } {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn try_wait_child(&mut self, current: TaskId, pid: ProcessId) -> Result<(ProcessId, i32), u32> {
        let current_pid = self.task(current).process_id;
        self.processes.take_zombie_child(current_pid, pid)
    }

    fn try_wait(&mut self, current: TaskId, pid: ProcessId) -> Result<u32, u32> {
        self.try_wait_child(current, pid)
            .map(|(child, code)| kernel_wait_status(child, code))
    }

    fn sys_wait4(
        &mut self,
        current: TaskId,
        pid: i32,
        status_ptr: u32,
        options: u32,
        rusage_ptr: u32,
    ) -> Result<u32, u32> {
        let requested = if pid > 0 {
            ProcessId::new(pid as usize)
        } else {
            ProcessId::new(0)
        };
        match self.try_wait_child(current, requested) {
            Ok((child, exit_code)) => {
                self.write_wait_status(current, child, exit_code, status_ptr, rusage_ptr)
            }
            Err(EAGAIN) if options & 1 != 0 => Ok(0),
            Err(err) => Err(err),
        }
    }

    fn write_wait_status(
        &self,
        task_id: TaskId,
        child: ProcessId,
        exit_code: i32,
        status_ptr: u32,
        rusage_ptr: u32,
    ) -> Result<u32, u32> {
        if status_ptr != 0 {
            let wait_status = linux_wait_status(exit_code);
            let bytes = wait_status.to_le_bytes();
            self.copy_to_task_user(task_id, status_ptr as usize, &bytes)?;
        }
        if rusage_ptr != 0 {
            let zero = [0u8; 72];
            self.copy_to_task_user(task_id, rusage_ptr as usize, &zero)?;
        }
        Ok(child.as_usize() as u32)
    }

    fn sys_spawn(&mut self, current: TaskId, path_ptr: u32, path_len: u32, argv_ptr: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = path_len.min(path.len() as u32) as usize;
        let result = unsafe { copy_from_user(&mut path[..len], UserPtr::new(path_ptr as usize)) };
        if let Err(err) = result {
            return err;
        }

        #[cfg(feature = "mmu")]
        {
            let (name, inode) = match self.lookup_exec_file(&path[..len]) {
                Ok(file) => file,
                Err(err) => return err,
            };
            let mut argv = [loader::UserArg::empty(); loader::MAX_INIT_ARGS];
            let mut argc = if argv_ptr == 0 {
                0
            } else {
                match self.copy_user_arg_array(argv_ptr, &mut argv) {
                    Ok(count) => count,
                    Err(err) => return err,
                }
            };
            if argc == 0 {
                argv[0] = loader::UserArg::from_bytes(&path[..len]);
                argc = 1;
            }
            let parent = self.task(current).process_id;
            let spawned = self.spawn_loaded_user_with_args(
                parent,
                name,
                1,
                1,
                |address_space| unsafe {
                    loader::load_elf_from_inode_into(address_space, name, inode)
                },
                &argv[..argc],
                &[],
            );
            return match spawned {
                Ok(task) => self.task(task).pid as u32,
                Err(err) => err,
            };
        }

        #[cfg(not(feature = "mmu"))]
        {
            let _ = current;
            ENOENT
        }
    }

    fn sys_exec(&mut self, current: TaskId, path_ptr: u32, path_len: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = path_len.min(path.len() as u32) as usize;
        let result = unsafe { copy_from_user(&mut path[..len], UserPtr::new(path_ptr as usize)) };
        if let Err(err) = result {
            return err;
        }

        #[cfg(feature = "mmu")]
        {
            let (name, inode) = match self.lookup_exec_file(&path[..len]) {
                Ok(file) => file,
                Err(err) => return err,
            };

            let argv0 = [loader::UserArg::from_bytes(
                path.as_slice().get(..len).unwrap_or(&[]),
            )];
            return self.exec_loaded_user_with_args(current, name, inode, &argv0, &[]);
        }

        #[cfg(not(feature = "mmu"))]
        {
            let _ = current;
            crate::kernel::syscall::ENOEXEC
        }
    }

    fn sys_execve(&mut self, current: TaskId, path_ptr: u32, argv_ptr: u32, envp_ptr: u32) -> u32 {
        let mut path = [0u8; 96];
        let path_len = match crate::kernel::user::copy_cstr_from_user(
            &mut path,
            UserPtr::new(path_ptr as usize),
            96,
        ) {
            Ok(len) => len,
            Err(err) => return err,
        };

        #[cfg(feature = "mmu")]
        {
            let (name, inode) = match self.lookup_exec_file(&path[..path_len]) {
                Ok(file) => file,
                Err(err) => return err,
            };
            let mut argv = [loader::UserArg::empty(); loader::MAX_INIT_ARGS];
            let mut envp = [loader::UserArg::empty(); loader::MAX_INIT_ENVS];
            let mut argc = match self.copy_user_arg_array(argv_ptr, &mut argv) {
                Ok(count) => count,
                Err(err) => return err,
            };
            if argc == 0 {
                argv[0] = loader::UserArg::from_bytes(&path[..path_len]);
                argc = 1;
            }
            let envc = match self.copy_user_arg_array(envp_ptr, &mut envp) {
                Ok(count) => count,
                Err(err) => return err,
            };
            return self.exec_loaded_user_with_args(
                current,
                name,
                inode,
                &argv[..argc],
                &envp[..envc],
            );
        }

        #[cfg(not(feature = "mmu"))]
        {
            let _ = (current, argv_ptr, envp_ptr, path_len);
            crate::kernel::syscall::ENOEXEC
        }
    }

    #[cfg(feature = "mmu")]
    fn lookup_exec_file(&self, path: &[u8]) -> Result<(&'static str, vfs::InodeRef), u32> {
        let inode = vfs::lookup(path)?;
        let Some(metadata) = vfs::metadata(inode) else {
            return Err(ENOENT);
        };
        if metadata.file_type != vfs::FileType::Regular {
            return Err(EINVAL);
        }
        if metadata.size == 0 {
            return Err(ENOENT);
        }
        Ok((exec_name(path), inode))
    }

    #[cfg(feature = "mmu")]
    fn exec_loaded_user_with_args(
        &mut self,
        current: TaskId,
        name: &'static str,
        inode: vfs::InodeRef,
        argv: &[loader::UserArg],
        envp: &[loader::UserArg],
    ) -> u32 {
        if !self.task(current).mode.is_user() {
            return EINVAL;
        }

        let process_id = self.task(current).process_id;
        let Some(old_space) = self.processes.address_space(process_id) else {
            return EINVAL;
        };
        if !old_space.kind.is_user() {
            return EINVAL;
        }

        let stack_slot = current.index() + 1;
        let (new_space, loaded, user_stack, initial_sp) = match self
            .build_user_address_space_with_args(
                stack_slot,
                |address_space| unsafe {
                    loader::load_elf_from_inode_into(address_space, name, inode)
                },
                Some(argv),
                Some(envp),
            ) {
            Ok(parts) => parts,
            Err(err) => return err,
        };

        let old_space = self
            .processes
            .replace_address_space(process_id, new_space)
            .unwrap_or(old_space);
        match self.processes.close_on_exec(process_id) {
            Ok(closed) => {
                for object in closed.iter() {
                    self.close_file_object(object);
                }
            }
            Err(err) => return err,
        }
        self.processes.rename(process_id, loaded.name);
        self.switch_address_space(current);
        loader::release_address_space_regions(old_space);

        let frame = unsafe { TrapFrame::from_saved_sp(self.task(current).context.sp) };
        frame.user_sp = initial_sp as u32;
        frame.user_lr = 0;
        frame.lr = 0;
        frame.pc = loaded.entry.as_usize() as u32;
        frame.spsr = USER_INITIAL_CPSR;
        frame.set_return_value(0);

        let task = self.task_mut(current);
        task.name = loaded.name;
        task.user_entry = loaded.entry.as_usize();
        task.user_stack_phys = user_stack;
        task.user_stack_pages = USER_STACK_PAGES;
        task.user_stack_bottom = new_space.stack_bottom.as_usize();
        task.user_stack_top = new_space.stack_top.as_usize();
        task.stats.bytes_written = 0;
        0
    }

    fn wake_waiters(&mut self, parent: ProcessId) {
        if !parent.is_valid() {
            return;
        }

        let channel = child_wait_channel(parent);
        let mut woken_tasks = [None; MAX_TASKS];
        let mut woken_len = 0;
        self.wait.wake_all(channel, |task| {
            woken_tasks[woken_len] = Some(task);
            woken_len += 1;
        });

        let mut still_waiting = [None; MAX_TASKS];
        let mut still_waiting_len = 0;
        for item in woken_tasks.iter().take(woken_len) {
            let Some(task_id) = *item else {
                continue;
            };
            if self.task(task_id).state != TaskState::Blocked {
                continue;
            }

            let requested = ProcessId::new(self.task(task_id).wait_pid);
            match self.try_wait_child(task_id, requested) {
                Ok((child, exit_code)) => {
                    let status_ptr = self.task(task_id).wait_user_buf as u32;
                    let rusage_ptr = self.task(task_id).wait_len as u32;
                    let result = if self.task(task_id).stats.last_syscall == SYS_WAIT {
                        kernel_wait_status(child, exit_code)
                    } else {
                        self.write_wait_status(task_id, child, exit_code, status_ptr, rusage_ptr)
                            .unwrap_or_else(|err| err)
                    };
                    let frame = unsafe { TrapFrame::from_saved_sp(self.task(task_id).context.sp) };
                    frame.set_return_value(result);
                    let task = self.task_mut(task_id);
                    task.state = TaskState::Ready;
                    task.wait_channel = 0;
                    task.wait_pid = 0;
                    task.wait_user_buf = 0;
                    task.wait_len = 0;
                    self.enqueue_ready(task_id);
                }
                Err(EAGAIN) => {
                    still_waiting[still_waiting_len] = Some(task_id);
                    still_waiting_len += 1;
                }
                Err(err) => {
                    let frame = unsafe { TrapFrame::from_saved_sp(self.task(task_id).context.sp) };
                    frame.set_return_value(err);
                    let task = self.task_mut(task_id);
                    task.state = TaskState::Ready;
                    task.wait_channel = 0;
                    task.wait_pid = 0;
                    task.wait_user_buf = 0;
                    task.wait_len = 0;
                    self.enqueue_ready(task_id);
                }
            }
        }

        for item in still_waiting.iter().take(still_waiting_len) {
            if let Some(task_id) = *item {
                self.wait.sleep_on(channel, task_id);
            }
        }
    }

    fn wake_console_readers(&mut self) {
        let channel = console::INPUT_WAIT_CHANNEL;
        let mut woken_tasks = [None; MAX_TASKS];
        let mut woken_len = 0;
        self.wait.wake_all(channel, |task| {
            woken_tasks[woken_len] = Some(task);
            woken_len += 1;
        });

        for item in woken_tasks.iter().take(woken_len) {
            let Some(task_id) = *item else {
                continue;
            };
            if self.task(task_id).state != TaskState::Blocked
                || self.task(task_id).wait_channel != channel
            {
                continue;
            }

            if console::available() == 0 {
                self.wait.sleep_on(channel, task_id);
                continue;
            }

            if self.task(task_id).wait_pid == usize::MAX {
                let pollfds = self.task(task_id).wait_user_buf as u32;
                let nfds = self.task(task_id).wait_len as u32;
                let result = self.sys_poll_for_task(task_id, pollfds, nfds, 0);
                let frame = unsafe { TrapFrame::from_saved_sp(self.task(task_id).context.sp) };
                frame.set_return_value(result);
                let task = self.task_mut(task_id);
                task.state = TaskState::Ready;
                task.wait_channel = 0;
                task.wait_pid = 0;
                task.wait_user_buf = 0;
                task.wait_len = 0;
                self.enqueue_ready(task_id);
                continue;
            }

            let user_buf = self.task(task_id).wait_user_buf;
            let max_len = self.task(task_id).wait_len.min(128);
            let mut chunk = [0u8; 128];
            let count = console::pop_into(&mut chunk[..max_len]);
            let result = if count == 0 {
                EAGAIN
            } else {
                match self.copy_to_task_user(task_id, user_buf, &chunk[..count]) {
                    Ok(()) => count as u32,
                    Err(err) => err,
                }
            };

            let frame = unsafe { TrapFrame::from_saved_sp(self.task(task_id).context.sp) };
            frame.set_return_value(result);
            let task = self.task_mut(task_id);
            task.state = TaskState::Ready;
            task.wait_channel = 0;
            task.wait_user_buf = 0;
            task.wait_len = 0;
            self.enqueue_ready(task_id);
        }
    }

    fn wake_pipe_readers(&mut self, id: ipc::PipeId) {
        let channel = ipc::read_wait_channel(id);
        let mut woken_tasks = [None; MAX_TASKS];
        let mut woken_len = 0;
        self.wait.wake_all(channel, |task| {
            woken_tasks[woken_len] = Some(task);
            woken_len += 1;
        });

        for item in woken_tasks.iter().take(woken_len) {
            let Some(task_id) = *item else {
                continue;
            };
            if self.task(task_id).state != TaskState::Blocked
                || self.task(task_id).wait_channel != channel
            {
                continue;
            }

            let user_buf = self.task(task_id).wait_user_buf;
            let max_len = self.task(task_id).wait_len.min(128);
            let mut chunk = [0u8; 128];
            let result = match ipc::read(id, &mut chunk[..max_len]) {
                PipeIo::Read { bytes, wake_writer } => {
                    if wake_writer {
                        self.wake_channel(ipc::write_wait_channel(id));
                    }
                    match self.copy_to_task_user(task_id, user_buf, &chunk[..bytes]) {
                        Ok(()) => bytes as u32,
                        Err(err) => err,
                    }
                }
                PipeIo::Closed => 0,
                PipeIo::WouldBlock { .. } => {
                    self.wait.sleep_on(channel, task_id);
                    continue;
                }
                PipeIo::Error(err) => err,
                PipeIo::Write { .. } => EINVAL,
            };

            let frame = unsafe { TrapFrame::from_saved_sp(self.task(task_id).context.sp) };
            frame.set_return_value(result);
            let task = self.task_mut(task_id);
            task.state = TaskState::Ready;
            task.wait_channel = 0;
            task.wait_user_buf = 0;
            task.wait_len = 0;
            self.enqueue_ready(task_id);
        }
    }

    fn close_file_object(&mut self, object: FileObject) {
        if let FileObject::Pipe { id, end } = object {
            ipc::close(id, end);
            match end {
                PipeEnd::Read => {
                    self.wake_channel(ipc::write_wait_channel(id));
                }
                PipeEnd::Write => {
                    self.wake_pipe_readers(id);
                }
            }
        }
    }

    fn copy_to_task_user(&self, task_id: TaskId, user_buf: usize, bytes: &[u8]) -> Result<(), u32> {
        #[cfg(feature = "mmu")]
        {
            let restore = self.current;
            self.switch_address_space(task_id);
            let result = unsafe { copy_to_user(UserPtr::new(user_buf), bytes) };
            if let Some(current) = restore {
                self.switch_address_space(current);
            }
            result
        }

        #[cfg(not(feature = "mmu"))]
        {
            let _ = task_id;
            unsafe { copy_to_user(UserPtr::new(user_buf), bytes) }
        }
    }

    fn find_task_by_pid(&self, pid: i32) -> Option<TaskId> {
        if pid <= 0 {
            return None;
        }
        let pid = pid as usize;
        self.tasks
            .iter()
            .find(|task| task.state != TaskState::Empty && task.process_id.as_usize() == pid)
            .map(|task| task.id)
    }

    #[cfg(feature = "mmu")]
    fn copy_user_arg_array(
        &self,
        argv_ptr: u32,
        out: &mut [loader::UserArg],
    ) -> Result<usize, u32> {
        if argv_ptr == 0 {
            return Ok(0);
        }
        let mut count = 0usize;
        while count < out.len() {
            let mut ptr_value = 0u32;
            let bytes = unsafe {
                core::slice::from_raw_parts_mut(
                    (&mut ptr_value as *mut u32).cast::<u8>(),
                    core::mem::size_of::<u32>(),
                )
            };
            unsafe {
                copy_from_user(
                    bytes,
                    UserPtr::new(
                        (argv_ptr as usize).wrapping_add(count * core::mem::size_of::<u32>()),
                    ),
                )?;
            }
            if ptr_value == 0 {
                return Ok(count);
            }
            let mut arg = loader::UserArg::empty();
            let len = crate::kernel::user::copy_cstr_from_user(
                &mut arg.bytes,
                UserPtr::new(ptr_value as usize),
                loader::INIT_ARG_LEN,
            )?;
            arg.len = len.min(loader::INIT_ARG_LEN - 1);
            out[count] = arg;
            count += 1;
        }
        Ok(count)
    }

    fn task(&self, id: TaskId) -> &Task {
        &self.tasks[id.index()]
    }

    fn task_mut(&mut self, id: TaskId) -> &mut Task {
        &mut self.tasks[id.index()]
    }
}
