use core::ptr::write_bytes;

pub const PAGE_SIZE: usize = 4096;
pub const PHYS_MEMORY_END: usize = 0x4800_0000;

unsafe extern "C" {
    static __kernel_end: u8;
}

static mut NEXT_FREE: usize = 0;
static mut ALLOCATED_PAGES: usize = 0;

pub unsafe fn init() {
    let heap_start = align_up(core::ptr::addr_of!(__kernel_end) as usize, PAGE_SIZE);
    unsafe {
        NEXT_FREE = heap_start;
        ALLOCATED_PAGES = 0;
    }
}

pub fn alloc_pages(pages: usize) -> Option<*mut u8> {
    if pages == 0 {
        return None;
    }

    let bytes = pages.checked_mul(PAGE_SIZE)?;

    unsafe {
        let start = align_up(NEXT_FREE, PAGE_SIZE);
        let end = start.checked_add(bytes)?;
        if end > PHYS_MEMORY_END {
            return None;
        }

        NEXT_FREE = end;
        ALLOCATED_PAGES += pages;
        write_bytes(start as *mut u8, 0, bytes);
        Some(start as *mut u8)
    }
}

pub fn allocated_pages() -> usize {
    unsafe { ALLOCATED_PAGES }
}

pub fn next_free() -> usize {
    unsafe { NEXT_FREE }
}

pub const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}
