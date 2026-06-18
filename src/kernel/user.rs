#![allow(dead_code)]

use crate::kernel::address::{USER_TOP, VirtAddr};

#[derive(Clone, Copy)]
pub enum UserAccess {
    Read,
    Write,
}

#[derive(Clone, Copy)]
pub struct UserPtr {
    addr: VirtAddr,
}

impl UserPtr {
    pub const fn new(addr: usize) -> Self {
        Self {
            addr: VirtAddr::new(addr),
        }
    }

    pub const fn addr(self) -> usize {
        self.addr.as_usize()
    }

    pub fn check(self, len: usize, _access: UserAccess) -> Result<(), u32> {
        let start = self.addr.as_usize();
        let Some(end) = start.checked_add(len) else {
            return Err(crate::kernel::syscall::EFAULT);
        };

        if start >= USER_TOP || end > USER_TOP {
            return Err(crate::kernel::syscall::EFAULT);
        }

        #[cfg(feature = "mmu")]
        if !crate::arch::aarch32::mmu::user_range_accessible(
            start,
            len,
            matches!(_access, UserAccess::Write),
        ) {
            return Err(crate::kernel::syscall::EFAULT);
        }

        Ok(())
    }
}

pub unsafe fn copy_from_user(dst: &mut [u8], src: UserPtr) -> Result<(), u32> {
    src.check(dst.len(), UserAccess::Read)?;
    unsafe {
        core::ptr::copy_nonoverlapping(src.addr() as *const u8, dst.as_mut_ptr(), dst.len());
    }
    Ok(())
}

pub unsafe fn copy_to_user(dst: UserPtr, src: &[u8]) -> Result<(), u32> {
    dst.check(src.len(), UserAccess::Write)?;
    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), dst.addr() as *mut u8, src.len());
    }
    Ok(())
}

pub fn copy_cstr_from_user(dst: &mut [u8], src: UserPtr, max_len: usize) -> Result<usize, u32> {
    let max_len = max_len.min(dst.len());
    src.check(max_len, UserAccess::Read)?;

    let mut copied = 0usize;
    while copied < max_len {
        let mut byte = [0u8; 1];
        unsafe {
            copy_from_user(&mut byte, UserPtr::new(src.addr() + copied))?;
        }
        dst[copied] = byte[0];
        if byte[0] == 0 {
            return Ok(copied);
        }
        copied += 1;
    }

    Ok(copied)
}
