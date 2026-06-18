use core::ptr::write_bytes;

use crate::kernel::address::{PageFrameNumber, PhysAddr};
use crate::platform::qemu_virt;

pub const PAGE_SIZE: usize = 4096;
pub const PHYS_MEMORY_START: usize = qemu_virt::RAM_START;
pub const PHYS_MEMORY_END: usize = qemu_virt::RAM_END;
pub const MAX_PHYS_PAGES: usize = (PHYS_MEMORY_END - PHYS_MEMORY_START) / PAGE_SIZE;
pub const MAX_ORDER: usize = 16;

const NO_PAGE: u16 = u16::MAX;

unsafe extern "C" {
    static __kernel_end: u8;
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PageState {
    Reserved,
    Free,
    Used,
}

impl PageState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::Free => "free",
            Self::Used => "used",
        }
    }
}

#[derive(Clone, Copy)]
pub struct PageFrame {
    pub state: PageState,
    pub order: u8,
    pub ref_count: u16,
    pub flags: u16,
    next: u16,
    prev: u16,
}

impl PageFrame {
    pub const fn reserved() -> Self {
        Self {
            state: PageState::Reserved,
            order: 0,
            ref_count: 0,
            flags: 0,
            next: NO_PAGE,
            prev: NO_PAGE,
        }
    }
}

#[derive(Clone, Copy)]
struct FreeArea {
    head: u16,
    count: usize,
}

impl FreeArea {
    const fn empty() -> Self {
        Self {
            head: NO_PAGE,
            count: 0,
        }
    }
}

static mut HEAP_START: usize = 0;
static mut HIGH_WATER: usize = 0;
static mut ALLOCATED_PAGES: usize = 0;
static mut FREE_PAGES: usize = 0;
static mut RESERVED_PAGES: usize = 0;
static mut FREE_AREAS: [FreeArea; MAX_ORDER] = [const { FreeArea::empty() }; MAX_ORDER];
static mut PAGE_FRAMES: [PageFrame; MAX_PHYS_PAGES] =
    [const { PageFrame::reserved() }; MAX_PHYS_PAGES];

pub unsafe fn init() {
    let heap_start = align_up(core::ptr::addr_of!(__kernel_end) as usize, PAGE_SIZE);
    if !(PHYS_MEMORY_START..=PHYS_MEMORY_END).contains(&heap_start) {
        panic!("kernel image is outside configured RAM");
    }

    let reserved_pages = (heap_start - PHYS_MEMORY_START) / PAGE_SIZE;
    let free_pages = MAX_PHYS_PAGES - reserved_pages;

    unsafe {
        HEAP_START = heap_start;
        HIGH_WATER = heap_start;
        ALLOCATED_PAGES = 0;
        FREE_PAGES = 0;
        RESERVED_PAGES = reserved_pages;
        FREE_AREAS = [const { FreeArea::empty() }; MAX_ORDER];

        for idx in 0..MAX_PHYS_PAGES {
            PAGE_FRAMES[idx] = PageFrame::reserved();
        }

        add_free_range(reserved_pages, free_pages);
    }
}

pub fn alloc_pages(pages: usize) -> Option<*mut u8> {
    if pages == 0 {
        return None;
    }

    let order = order_for_pages(pages)?;
    unsafe {
        let mut current_order = order;
        while current_order < MAX_ORDER && FREE_AREAS[current_order].head == NO_PAGE {
            current_order += 1;
        }

        if current_order == MAX_ORDER {
            return None;
        }

        let pfn = remove_head(current_order);
        while current_order > order {
            current_order -= 1;
            let buddy = pfn + (1usize << current_order);
            insert_block(buddy, current_order);
        }

        let block_pages = 1usize << order;
        mark_pages(pfn, block_pages, PageState::Used, order, 1);
        ALLOCATED_PAGES += block_pages;
        FREE_PAGES -= block_pages;

        let ptr = pfn_to_addr(pfn) as *mut u8;
        HIGH_WATER = HIGH_WATER.max(ptr as usize + block_pages * PAGE_SIZE);
        write_bytes(ptr, 0, block_pages * PAGE_SIZE);
        Some(ptr)
    }
}

pub unsafe fn free_pages(ptr: *mut u8, pages: usize) {
    if ptr.is_null() || pages == 0 {
        return;
    }

    let start = ptr as usize;
    if start & (PAGE_SIZE - 1) != 0 {
        panic!("free_pages: unaligned page");
    }

    let Some(order) = order_for_pages(pages) else {
        panic!("free_pages: page count is too large");
    };
    let block_pages = 1usize << order;
    if !range_in_ram(start, block_pages) {
        panic!("free_pages: range outside RAM");
    }

    let pfn = addr_to_pfn(start).unwrap_or_else(|| panic!("free_pages: address outside RAM"));
    if pfn & (block_pages - 1) != 0 {
        panic!("free_pages: block is not aligned to its buddy order");
    }

    unsafe {
        assert_range_state(pfn, block_pages, PageState::Used);
        write_bytes(ptr, 0, block_pages * PAGE_SIZE);
        ALLOCATED_PAGES = ALLOCATED_PAGES.saturating_sub(block_pages);
        FREE_PAGES += block_pages;

        let mut merged_pfn = pfn;
        let mut merged_order = order;
        while merged_order + 1 < MAX_ORDER {
            let buddy = merged_pfn ^ (1usize << merged_order);
            if buddy >= MAX_PHYS_PAGES || !is_free_block(buddy, merged_order) {
                break;
            }

            remove_block(buddy, merged_order);
            if buddy < merged_pfn {
                merged_pfn = buddy;
            }
            merged_order += 1;
        }

        insert_block(merged_pfn, merged_order);
    }
}

pub unsafe fn reserve_range(start: usize, size: usize) {
    if size == 0 {
        return;
    }

    let Some(raw_end) = start.checked_add(size) else {
        panic!("reserve_range: range overflows");
    };
    let start = crate::kernel::address::align_down(start, PAGE_SIZE);
    let end = crate::kernel::address::align_up(raw_end, PAGE_SIZE);
    if start >= end {
        return;
    }
    if start < PHYS_MEMORY_START || end > PHYS_MEMORY_END {
        panic!("reserve_range: range outside RAM");
    }

    let first = addr_to_pfn(start).unwrap_or_else(|| panic!("reserve_range: bad start"));
    let pages = (end - start) / PAGE_SIZE;
    unsafe {
        for pfn in first..first + pages {
            match PAGE_FRAMES[pfn].state {
                PageState::Reserved => {}
                PageState::Used => panic!("reserve_range: page is already used"),
                PageState::Free => {
                    PAGE_FRAMES[pfn] = PageFrame::reserved();
                    RESERVED_PAGES += 1;
                    FREE_PAGES = FREE_PAGES.saturating_sub(1);
                }
            }
        }
        rebuild_free_areas();
    }
}

pub fn heap_start() -> usize {
    unsafe { HEAP_START }
}

pub fn allocated_pages() -> usize {
    unsafe { ALLOCATED_PAGES }
}

pub fn free_page_count() -> usize {
    unsafe { FREE_PAGES }
}

pub fn reserved_pages() -> usize {
    unsafe { RESERVED_PAGES }
}

pub const fn total_pages() -> usize {
    MAX_PHYS_PAGES
}

pub fn free_ranges() -> usize {
    let mut total = 0;
    for order in 0..MAX_ORDER {
        total += unsafe { FREE_AREAS[order].count };
    }
    total
}

pub fn buddy_order_count(order: usize) -> usize {
    if order >= MAX_ORDER {
        return 0;
    }
    unsafe { FREE_AREAS[order].count }
}

pub fn largest_free_order() -> Option<usize> {
    (0..MAX_ORDER)
        .rev()
        .find(|order| unsafe { FREE_AREAS[*order].count != 0 })
}

pub fn next_free() -> usize {
    unsafe { HIGH_WATER }
}

pub fn page_state(addr: usize) -> Option<PageState> {
    let index = addr_to_pfn(addr)?;
    Some(unsafe { PAGE_FRAMES[index].state })
}

pub fn page_frame(addr: usize) -> Option<PageFrame> {
    let index = addr_to_pfn(addr)?;
    Some(unsafe { PAGE_FRAMES[index] })
}

pub fn phys_to_pfn(addr: PhysAddr) -> Option<PageFrameNumber> {
    let pfn = addr_to_pfn(addr.as_usize())?;
    Some(PageFrameNumber::new(pfn))
}

pub const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

unsafe fn add_free_range(mut start_pfn: usize, mut pages: usize) {
    while pages != 0 {
        let mut order = MAX_ORDER - 1;
        while order > 0 {
            let block_pages = 1usize << order;
            if start_pfn & (block_pages - 1) == 0 && block_pages <= pages {
                break;
            }
            order -= 1;
        }

        unsafe {
            insert_block(start_pfn, order);
            FREE_PAGES += 1usize << order;
        }
        start_pfn += 1usize << order;
        pages -= 1usize << order;
    }
}

fn order_for_pages(pages: usize) -> Option<usize> {
    let mut order = 0;
    let mut block_pages = 1usize;
    while block_pages < pages {
        order += 1;
        block_pages <<= 1;
        if order >= MAX_ORDER {
            return None;
        }
    }
    Some(order)
}

unsafe fn insert_block(pfn: usize, order: usize) {
    let block_pages = 1usize << order;
    unsafe {
        mark_pages(pfn, block_pages, PageState::Free, order, 0);
    }

    let head = unsafe { FREE_AREAS[order].head };
    unsafe {
        PAGE_FRAMES[pfn].next = head;
        PAGE_FRAMES[pfn].prev = NO_PAGE;
        if head != NO_PAGE {
            PAGE_FRAMES[head as usize].prev = pfn as u16;
        }
        FREE_AREAS[order].head = pfn as u16;
        FREE_AREAS[order].count += 1;
    }
}

unsafe fn remove_head(order: usize) -> usize {
    let head = unsafe { FREE_AREAS[order].head };
    if head == NO_PAGE {
        panic!("buddy free area empty");
    }
    unsafe {
        remove_block(head as usize, order);
    }
    head as usize
}

unsafe fn remove_block(pfn: usize, order: usize) {
    let next = unsafe { PAGE_FRAMES[pfn].next };
    let prev = unsafe { PAGE_FRAMES[pfn].prev };

    unsafe {
        if prev == NO_PAGE {
            FREE_AREAS[order].head = next;
        } else {
            PAGE_FRAMES[prev as usize].next = next;
        }

        if next != NO_PAGE {
            PAGE_FRAMES[next as usize].prev = prev;
        }

        PAGE_FRAMES[pfn].next = NO_PAGE;
        PAGE_FRAMES[pfn].prev = NO_PAGE;
        FREE_AREAS[order].count -= 1;
    }
}

unsafe fn rebuild_free_areas() {
    unsafe {
        FREE_AREAS = [const { FreeArea::empty() }; MAX_ORDER];
    }

    let mut pfn = 0usize;
    while pfn < MAX_PHYS_PAGES {
        if unsafe { PAGE_FRAMES[pfn].state } != PageState::Free {
            pfn += 1;
            continue;
        }

        let start = pfn;
        while pfn < MAX_PHYS_PAGES && unsafe { PAGE_FRAMES[pfn].state } == PageState::Free {
            pfn += 1;
        }

        unsafe {
            add_free_range_rebuild(start, pfn - start);
        }
    }
}

unsafe fn add_free_range_rebuild(mut start_pfn: usize, mut pages: usize) {
    while pages != 0 {
        let mut order = MAX_ORDER - 1;
        while order > 0 {
            let block_pages = 1usize << order;
            if start_pfn & (block_pages - 1) == 0 && block_pages <= pages {
                break;
            }
            order -= 1;
        }
        unsafe {
            insert_block(start_pfn, order);
        }
        start_pfn += 1usize << order;
        pages -= 1usize << order;
    }
}

unsafe fn is_free_block(pfn: usize, order: usize) -> bool {
    if pfn >= MAX_PHYS_PAGES {
        return false;
    }

    let frame = unsafe { PAGE_FRAMES[pfn] };
    frame.state == PageState::Free && frame.order as usize == order && frame.ref_count == 0
}

unsafe fn assert_range_state(pfn: usize, pages: usize, expected: PageState) {
    for offset in 0..pages {
        let frame = unsafe { PAGE_FRAMES[pfn + offset] };
        if frame.state != expected {
            panic!(
                "page state mismatch at {:#010x}: expected {}, got {}",
                pfn_to_addr(pfn + offset),
                expected.as_str(),
                frame.state.as_str()
            );
        }
    }
}

unsafe fn mark_pages(pfn: usize, pages: usize, state: PageState, order: usize, ref_count: u16) {
    for offset in 0..pages {
        unsafe {
            PAGE_FRAMES[pfn + offset] = PageFrame {
                state,
                order: order as u8,
                ref_count,
                flags: 0,
                next: NO_PAGE,
                prev: NO_PAGE,
            };
        }
    }
}

fn addr_to_pfn(addr: usize) -> Option<usize> {
    if !(PHYS_MEMORY_START..PHYS_MEMORY_END).contains(&addr) {
        return None;
    }

    Some((addr - PHYS_MEMORY_START) / PAGE_SIZE)
}

fn pfn_to_addr(pfn: usize) -> usize {
    PHYS_MEMORY_START + pfn * PAGE_SIZE
}

fn range_in_ram(start: usize, pages: usize) -> bool {
    let Some(bytes) = pages.checked_mul(PAGE_SIZE) else {
        return false;
    };
    let Some(end) = start.checked_add(bytes) else {
        return false;
    };

    start >= PHYS_MEMORY_START && end <= PHYS_MEMORY_END
}
