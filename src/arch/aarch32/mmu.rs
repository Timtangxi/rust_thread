use aarch32_cpu::asm;

use crate::kernel::address::{DEVICE_HIGH_BASE, KERNEL_HIGH_BASE, align_down, align_up};
use crate::kernel::memory::{self, PAGE_SIZE};
use crate::platform::fdt;

const L1_ENTRIES: usize = 4096;
const L2_ENTRIES: usize = 256;
pub const L1_TABLE_PAGES: usize = 4;
pub const L2_TABLE_PAGES: usize = 1;
const L2_TABLES: usize = 384;

const PAGE_MASK: usize = PAGE_SIZE - 1;

const L1_DESC_PAGE_TABLE: u32 = 0b01;
const L1_DOMAIN_SHIFT: u32 = 5;

const SMALL_PAGE_DESC: u32 = 0b10;
const SMALL_PAGE_B: u32 = 1 << 2;
const SMALL_PAGE_C: u32 = 1 << 3;
const SMALL_PAGE_AP0_SHIFT: u32 = 4;
const SMALL_PAGE_AP2_SHIFT: u32 = 9;
const SMALL_PAGE_TEX_SHIFT: u32 = 6;
const SMALL_PAGE_SHAREABLE: u32 = 1 << 10;
const SMALL_PAGE_NG: u32 = 1 << 11;
const SMALL_PAGE_XN: u32 = 1 << 0;
const SMALL_PAGE_BASE_MASK: u32 = 0xffff_f000;

const AP_PRIV_RW: u32 = 0b01;
const AP_PRIV_RO: u32 = 0b101;
const AP_USER_RW: u32 = 0b011;
const AP_USER_RO: u32 = 0b110;

const DOMAIN0_CLIENT: u32 = 0b01;
const ASID_MASK: u32 = 0xff;

const SCTLR_M: u32 = 1 << 0;
const SCTLR_C: u32 = 1 << 2;
const SCTLR_I: u32 = 1 << 12;

const ENABLE_ICACHE: bool = true;
const ENABLE_DCACHE: bool = false;

unsafe extern "C" {
    static _start: u8;
    static __text_start: u8;
    static __text_end: u8;
    static __rodata_start: u8;
    static __rodata_end: u8;
    static __user_text_start: u8;
    static __user_text_end: u8;
    static __user_rodata_start: u8;
    static __user_rodata_end: u8;
    static __data_start: u8;
    static __data_end: u8;
    static __bss_start: u8;
    static __bss_end: u8;
    static __kernel_end: u8;
}

#[repr(align(16384))]
struct LevelOneTable([u32; L1_ENTRIES]);

#[repr(align(1024))]
struct LevelTwoTables([[u32; L2_ENTRIES]; L2_TABLES]);

static mut L1_TABLE: LevelOneTable = LevelOneTable([0; L1_ENTRIES]);
static mut L2_TABLES_STORE: LevelTwoTables = LevelTwoTables([[0; L2_ENTRIES]; L2_TABLES]);
static mut L2_USED: [bool; L2_TABLES] = [false; L2_TABLES];

#[derive(Clone, Copy)]
pub struct MmuInfo {
    pub enabled: bool,
    pub table_base: usize,
    pub kernel_start: usize,
    pub kernel_end: usize,
    pub ram_pages: usize,
    pub kernel_text_pages: usize,
    pub kernel_rodata_pages: usize,
    pub user_text_pages: usize,
    pub user_rodata_pages: usize,
    pub kernel_data_pages: usize,
    pub device_pages: usize,
    pub high_linear_pages: usize,
    pub high_device_pages: usize,
    pub l2_tables: usize,
    pub icache_enabled: bool,
    pub dcache_enabled: bool,
    pub sctlr: u32,
}

#[derive(Clone, Copy)]
pub enum UserMapping {
    Rx,
    RoData,
    RwData,
    Stack,
}

#[derive(Clone, Copy)]
struct PageAttrs {
    bits: u32,
}

impl PageAttrs {
    const fn normal_rw_xn() -> Self {
        Self {
            bits: SMALL_PAGE_DESC
                | SMALL_PAGE_XN
                | ap_bits(AP_PRIV_RW)
                | (0b001 << SMALL_PAGE_TEX_SHIFT)
                | SMALL_PAGE_C
                | SMALL_PAGE_B
                | SMALL_PAGE_SHAREABLE,
        }
    }

    const fn normal_rx() -> Self {
        Self {
            bits: SMALL_PAGE_DESC
                | ap_bits(AP_PRIV_RO)
                | (0b001 << SMALL_PAGE_TEX_SHIFT)
                | SMALL_PAGE_C
                | SMALL_PAGE_B
                | SMALL_PAGE_SHAREABLE,
        }
    }

    const fn normal_ro_xn() -> Self {
        Self {
            bits: SMALL_PAGE_DESC
                | SMALL_PAGE_XN
                | ap_bits(AP_PRIV_RO)
                | (0b001 << SMALL_PAGE_TEX_SHIFT)
                | SMALL_PAGE_C
                | SMALL_PAGE_B
                | SMALL_PAGE_SHAREABLE,
        }
    }

    const fn device_rw_xn() -> Self {
        Self {
            bits: SMALL_PAGE_DESC
                | SMALL_PAGE_XN
                | ap_bits(AP_PRIV_RW)
                | SMALL_PAGE_B
                | SMALL_PAGE_SHAREABLE
                | SMALL_PAGE_NG,
        }
    }

    const fn user_rx() -> Self {
        Self {
            bits: SMALL_PAGE_DESC
                | ap_bits(AP_USER_RO)
                | (0b001 << SMALL_PAGE_TEX_SHIFT)
                | SMALL_PAGE_C
                | SMALL_PAGE_B
                | SMALL_PAGE_SHAREABLE
                | SMALL_PAGE_NG,
        }
    }

    const fn user_ro_xn() -> Self {
        Self {
            bits: SMALL_PAGE_DESC
                | SMALL_PAGE_XN
                | ap_bits(AP_USER_RO)
                | (0b001 << SMALL_PAGE_TEX_SHIFT)
                | SMALL_PAGE_C
                | SMALL_PAGE_B
                | SMALL_PAGE_SHAREABLE
                | SMALL_PAGE_NG,
        }
    }

    const fn user_rw_xn() -> Self {
        Self {
            bits: SMALL_PAGE_DESC
                | SMALL_PAGE_XN
                | ap_bits(AP_USER_RW)
                | (0b001 << SMALL_PAGE_TEX_SHIFT)
                | SMALL_PAGE_C
                | SMALL_PAGE_B
                | SMALL_PAGE_SHAREABLE
                | SMALL_PAGE_NG,
        }
    }
}

const fn ap_bits(ap: u32) -> u32 {
    ((ap & 0b11) << SMALL_PAGE_AP0_SHIFT) | (((ap >> 2) & 1) << SMALL_PAGE_AP2_SHIFT)
}

struct PageTableBuilder {
    l1: *mut u32,
}

impl PageTableBuilder {
    unsafe fn new() -> Self {
        let l1 = unsafe { l1_ptr() };
        for idx in 0..L1_ENTRIES {
            unsafe {
                l1.add(idx).write_volatile(0);
            }
        }

        let l2 = unsafe { l2_base_ptr() };
        for idx in 0..(L2_TABLES * L2_ENTRIES) {
            unsafe {
                l2.add(idx).write_volatile(0);
            }
        }

        for idx in 0..L2_TABLES {
            unsafe {
                L2_USED[idx] = false;
            }
        }

        Self { l1 }
    }

    unsafe fn map_range(&mut self, start: usize, end: usize, attrs: PageAttrs) -> usize {
        unsafe { self.map_range_to(start, start, end - start, attrs) }
    }

    unsafe fn map_range_to(
        &mut self,
        virt_start: usize,
        phys_start: usize,
        size: usize,
        attrs: PageAttrs,
    ) -> usize {
        if size == 0 {
            return 0;
        }

        if virt_start & PAGE_MASK != 0 || phys_start & PAGE_MASK != 0 {
            panic!("mmu: mapped ranges must be page aligned");
        }

        let mut addr = align_down(virt_start, PAGE_SIZE);
        let end = align_up(virt_start + size, PAGE_SIZE);
        let mut phys = phys_start;
        let mut pages = 0;

        while addr < end {
            unsafe {
                self.map_page(addr, phys, attrs);
            }
            pages += 1;
            addr += PAGE_SIZE;
            phys += PAGE_SIZE;
        }

        pages
    }

    unsafe fn map_page(&mut self, virt: usize, phys: usize, attrs: PageAttrs) {
        if virt & PAGE_MASK != 0 || phys & PAGE_MASK != 0 {
            panic!("mmu: small page mapping must be 4 KiB aligned");
        }

        let l1_index = virt >> 20;
        let l2_index = (virt >> 12) & 0xff;
        let l2 = unsafe { self.ensure_l2(l1_index) };
        let entry = (phys as u32 & SMALL_PAGE_BASE_MASK) | attrs.bits;
        unsafe {
            l2.add(l2_index).write_volatile(entry);
        }
    }

    unsafe fn ensure_l2(&mut self, l1_index: usize) -> *mut u32 {
        let current = unsafe { self.l1.add(l1_index).read_volatile() };
        if current & 0b11 == L1_DESC_PAGE_TABLE {
            return (current as usize & 0xffff_fc00) as *mut u32;
        }

        let table_index = unsafe { allocate_l2_table() };
        let table = unsafe { l2_table_ptr(table_index) };
        let entry = (table as u32 & 0xffff_fc00) | (0 << L1_DOMAIN_SHIFT) | L1_DESC_PAGE_TABLE;
        unsafe {
            self.l1.add(l1_index).write_volatile(entry);
        }
        table
    }
}

pub unsafe fn init() -> MmuInfo {
    let table_base = unsafe { l1_ptr() } as usize;
    if table_base & (16 * 1024 - 1) != 0 {
        panic!("mmu: first-level table is not 16 KiB aligned");
    }

    let boot = fdt::boot_info();
    let ram_start = align_down(boot.ram_start, PAGE_SIZE);
    let ram_end = align_up(boot.ram_end, PAGE_SIZE);
    let mmio_start = align_down(boot.mmio_start, PAGE_SIZE);
    let mmio_end = align_up(boot.mmio_end, PAGE_SIZE);

    let mut builder = unsafe { PageTableBuilder::new() };
    let ram_pages = unsafe { builder.map_range(ram_start, ram_end, PageAttrs::normal_rw_xn()) };
    let device_pages =
        unsafe { builder.map_range(mmio_start, mmio_end, PageAttrs::device_rw_xn()) };
    let high_linear_pages = unsafe {
        builder.map_range_to(
            KERNEL_HIGH_BASE,
            ram_start,
            ram_end - ram_start,
            PageAttrs::normal_rw_xn(),
        )
    };
    let high_device_pages = unsafe {
        builder.map_range_to(
            DEVICE_HIGH_BASE,
            mmio_start,
            mmio_end - mmio_start,
            PageAttrs::device_rw_xn(),
        )
    };

    let text_start = sym_addr(&raw const __text_start);
    let text_end = sym_addr(&raw const __text_end);
    let rodata_start = sym_addr(&raw const __rodata_start);
    let rodata_end = sym_addr(&raw const __rodata_end);
    let user_text_start = sym_addr(&raw const __user_text_start);
    let user_text_end = sym_addr(&raw const __user_text_end);
    let user_rodata_start = sym_addr(&raw const __user_rodata_start);
    let user_rodata_end = sym_addr(&raw const __user_rodata_end);
    let data_start = sym_addr(&raw const __data_start);
    let data_end = sym_addr(&raw const __data_end);
    let bss_start = sym_addr(&raw const __bss_start);
    let bss_end = sym_addr(&raw const __bss_end);
    let kernel_start = sym_addr(&raw const _start);
    let kernel_end = sym_addr(&raw const __kernel_end);

    let kernel_text_pages =
        unsafe { builder.map_range(text_start, text_end, PageAttrs::normal_rx()) };
    let kernel_rodata_pages =
        unsafe { builder.map_range(rodata_start, rodata_end, PageAttrs::normal_ro_xn()) };
    let user_text_pages =
        unsafe { builder.map_range(user_text_start, user_text_end, PageAttrs::user_rx()) };
    let user_rodata_pages =
        unsafe { builder.map_range(user_rodata_start, user_rodata_end, PageAttrs::user_ro_xn()) };
    let data_pages = unsafe { builder.map_range(data_start, data_end, PageAttrs::normal_rw_xn()) };
    let bss_pages = unsafe { builder.map_range(bss_start, bss_end, PageAttrs::normal_rw_xn()) };
    unsafe {
        map_kernel_high_aliases(
            &mut builder,
            ram_start,
            text_start,
            text_end,
            PageAttrs::normal_rx(),
        );
        map_kernel_high_aliases(
            &mut builder,
            ram_start,
            rodata_start,
            rodata_end,
            PageAttrs::normal_ro_xn(),
        );
        map_kernel_high_aliases(
            &mut builder,
            ram_start,
            data_start,
            data_end,
            PageAttrs::normal_rw_xn(),
        );
        map_kernel_high_aliases(
            &mut builder,
            ram_start,
            bss_start,
            bss_end,
            PageAttrs::normal_rw_xn(),
        );
    }

    unsafe {
        enable(table_base);
    }

    let sctlr = read_sctlr();
    MmuInfo {
        enabled: sctlr & SCTLR_M != 0,
        table_base,
        kernel_start,
        kernel_end,
        ram_pages,
        kernel_text_pages,
        kernel_rodata_pages,
        user_text_pages,
        user_rodata_pages,
        kernel_data_pages: data_pages + bss_pages,
        device_pages,
        high_linear_pages,
        high_device_pages,
        l2_tables: used_l2_tables(),
        icache_enabled: sctlr & SCTLR_I != 0,
        dcache_enabled: sctlr & SCTLR_C != 0,
        sctlr,
    }
}

pub unsafe fn map_user_pages_in(
    table_base: usize,
    virt_start: usize,
    phys_start: usize,
    pages: usize,
    mapping: UserMapping,
) -> usize {
    if pages == 0 {
        return 0;
    }

    let mut builder = PageTableBuilder {
        l1: table_base as *mut u32,
    };
    let mapped = unsafe {
        builder.map_range_to(
            virt_start,
            phys_start,
            pages * PAGE_SIZE,
            user_attrs(mapping),
        )
    };
    finish_page_table_update();
    mapped
}

pub unsafe fn unmap_pages_in(table_base: usize, virt_start: usize, pages: usize) {
    if pages == 0 {
        return;
    }

    if virt_start & PAGE_MASK != 0 {
        panic!("mmu: unmap address must be 4 KiB aligned");
    }

    let mut virt = virt_start;
    for _ in 0..pages {
        let l1_index = virt >> 20;
        let l2_index = (virt >> 12) & 0xff;
        let l1 = table_base as *mut u32;
        let l1_entry = unsafe { l1.add(l1_index).read_volatile() };
        if l1_entry & 0b11 == L1_DESC_PAGE_TABLE {
            let l2 = (l1_entry as usize & 0xffff_fc00) as *mut u32;
            unsafe {
                l2.add(l2_index).write_volatile(0);
            }
        }
        virt += PAGE_SIZE;
    }
    finish_page_table_update();
}

pub unsafe fn create_user_table() -> Option<usize> {
    let ptr = memory::alloc_pages(L1_TABLE_PAGES)?;
    let root = ptr as *mut u32;
    let kernel = unsafe { l1_ptr() };
    unsafe {
        core::ptr::copy_nonoverlapping(kernel, root, L1_ENTRIES);
    }
    Some(ptr as usize)
}

pub unsafe fn free_user_table(root: usize) {
    if root != 0 && root != table_base() {
        unsafe {
            memory::free_pages(root as *mut u8, L1_TABLE_PAGES);
        }
    }
}

pub unsafe fn ensure_user_l2(table_base: usize, virt: usize) -> Option<usize> {
    let l1_index = virt >> 20;
    let l1 = table_base as *mut u32;
    let current = unsafe { l1.add(l1_index).read_volatile() };
    if current & 0b11 == L1_DESC_PAGE_TABLE {
        return Some(0);
    }

    let table = memory::alloc_pages(L2_TABLE_PAGES)? as *mut u32;
    for idx in 0..L2_ENTRIES {
        unsafe {
            table.add(idx).write_volatile(0);
        }
    }

    let entry = (table as u32 & 0xffff_fc00) | (0 << L1_DOMAIN_SHIFT) | L1_DESC_PAGE_TABLE;
    unsafe {
        l1.add(l1_index).write_volatile(entry);
    }
    Some(table as usize)
}

pub unsafe fn free_user_l2(table: usize) {
    if table != 0 {
        unsafe {
            memory::free_pages(table as *mut u8, L2_TABLE_PAGES);
        }
    }
}

pub fn user_range_accessible(start: usize, len: usize, write: bool) -> bool {
    if len == 0 {
        return true;
    }

    let Some(end) = start.checked_add(len) else {
        return false;
    };
    if end < start {
        return false;
    }

    let mut virt = align_down(start, PAGE_SIZE);
    let end = align_up(end, PAGE_SIZE);
    while virt < end {
        let Some(entry) = page_entry_in(current_table_base(), virt) else {
            return false;
        };
        if !entry_user_accessible(entry, write) {
            return false;
        }
        virt += PAGE_SIZE;
    }
    true
}

pub fn table_base() -> usize {
    unsafe { l1_ptr() as usize }
}

pub fn current_table_base() -> usize {
    (read_ttbr0() as usize) & 0xffff_c000
}

pub fn switch_table(table_base: usize, asid: u16) {
    let asid = u32::from(asid) & ASID_MASK;
    if current_table_base() == table_base && read_contextidr() & ASID_MASK == asid {
        return;
    }

    asm::dsb();
    write_contextidr(asid);
    write_ttbr0(table_base as u32);
    invalidate_all_tlb();
    asm::isb();
}

fn page_entry_in(table_base: usize, virt: usize) -> Option<u32> {
    let l1_index = virt >> 20;
    let l2_index = (virt >> 12) & 0xff;
    let l1 = table_base as *const u32;
    let l1_entry = unsafe { l1.add(l1_index).read_volatile() };
    if l1_entry & 0b11 != L1_DESC_PAGE_TABLE {
        return None;
    }

    let l2 = (l1_entry as usize & 0xffff_fc00) as *const u32;
    let entry = unsafe { l2.add(l2_index).read_volatile() };
    if entry & SMALL_PAGE_DESC != SMALL_PAGE_DESC {
        return None;
    }
    Some(entry)
}

fn user_attrs(mapping: UserMapping) -> PageAttrs {
    match mapping {
        UserMapping::Rx => PageAttrs::user_rx(),
        UserMapping::RoData => PageAttrs::user_ro_xn(),
        UserMapping::RwData | UserMapping::Stack => PageAttrs::user_rw_xn(),
    }
}

fn entry_user_accessible(entry: u32, write: bool) -> bool {
    let ap =
        (((entry >> SMALL_PAGE_AP2_SHIFT) & 1) << 2) | ((entry >> SMALL_PAGE_AP0_SHIFT) & 0b11);
    if write {
        ap == AP_USER_RW
    } else {
        ap == AP_USER_RW || ap == AP_USER_RO
    }
}

fn finish_page_table_update() {
    asm::dsb();
    invalidate_all_tlb();
    asm::isb();
}

unsafe fn map_kernel_high_aliases(
    builder: &mut PageTableBuilder,
    ram_start: usize,
    phys_start: usize,
    phys_end: usize,
    attrs: PageAttrs,
) -> usize {
    let virt_start = KERNEL_HIGH_BASE + (phys_start - ram_start);
    unsafe { builder.map_range_to(virt_start, phys_start, phys_end - phys_start, attrs) }
}

pub fn invalidate_all_tlb() {
    unsafe {
        core::arch::asm!(
            "mov r0, #0",
            "mcr p15, 0, r0, c8, c7, 0",
            out("r0") _,
            options(nostack, preserves_flags)
        );
    }
}

pub fn invalidate_instruction_cache() {
    unsafe {
        core::arch::asm!(
            "mov r0, #0",
            "mcr p15, 0, r0, c7, c5, 0",
            out("r0") _,
            options(nostack, preserves_flags)
        );
    }
}

pub fn invalidate_branch_predictor() {
    unsafe {
        core::arch::asm!(
            "mov r0, #0",
            "mcr p15, 0, r0, c7, c5, 6",
            out("r0") _,
            options(nostack, preserves_flags)
        );
    }
}

pub fn clean_invalidate_data_cache() {
    unsafe {
        core::arch::asm!(
            "mov r0, #0",
            "mcr p15, 0, r0, c7, c14, 0",
            out("r0") _,
            options(nostack, preserves_flags)
        );
    }
}

unsafe fn enable(table_base: usize) {
    asm::dsb();
    clean_invalidate_data_cache();
    invalidate_instruction_cache();
    invalidate_branch_predictor();
    invalidate_all_tlb();

    write_ttbcr(0);
    write_ttbr0(table_base as u32);
    write_dacr(DOMAIN0_CLIENT);

    asm::dsb();
    asm::isb();

    let mut control = read_sctlr();
    control |= SCTLR_M;
    if ENABLE_ICACHE {
        control |= SCTLR_I;
    } else {
        control &= !SCTLR_I;
    }

    if ENABLE_DCACHE {
        control |= SCTLR_C;
    } else {
        control &= !SCTLR_C;
    }

    write_sctlr(control);
    asm::isb();
    asm::dsb();
}

unsafe fn l1_ptr() -> *mut u32 {
    unsafe { core::ptr::addr_of_mut!(L1_TABLE.0) as *mut u32 }
}

unsafe fn l2_base_ptr() -> *mut u32 {
    unsafe { core::ptr::addr_of_mut!(L2_TABLES_STORE.0) as *mut u32 }
}

unsafe fn l2_table_ptr(index: usize) -> *mut u32 {
    unsafe { core::ptr::addr_of_mut!(L2_TABLES_STORE.0[index]) as *mut u32 }
}

unsafe fn allocate_l2_table() -> usize {
    for idx in 0..L2_TABLES {
        if unsafe { !L2_USED[idx] } {
            unsafe {
                L2_USED[idx] = true;
            }
            return idx;
        }
    }

    panic!("mmu: out of static second-level page tables");
}

fn used_l2_tables() -> usize {
    let mut count = 0;
    for idx in 0..L2_TABLES {
        if unsafe { L2_USED[idx] } {
            count += 1;
        }
    }
    count
}

fn sym_addr(sym: *const u8) -> usize {
    sym as usize
}

fn read_sctlr() -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!("mrc p15, 0, {0}, c1, c0, 0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

fn write_sctlr(value: u32) {
    unsafe {
        core::arch::asm!("mcr p15, 0, {0}, c1, c0, 0", in(reg) value, options(nostack, preserves_flags));
    }
}

fn write_ttbr0(value: u32) {
    unsafe {
        core::arch::asm!("mcr p15, 0, {0}, c2, c0, 0", in(reg) value, options(nostack, preserves_flags));
    }
}

fn read_ttbr0() -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!("mrc p15, 0, {0}, c2, c0, 0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

fn write_contextidr(value: u32) {
    unsafe {
        core::arch::asm!("mcr p15, 0, {0}, c13, c0, 1", in(reg) value, options(nostack, preserves_flags));
    }
}

fn read_contextidr() -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!("mrc p15, 0, {0}, c13, c0, 1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

fn write_ttbcr(value: u32) {
    unsafe {
        core::arch::asm!("mcr p15, 0, {0}, c2, c0, 2", in(reg) value, options(nostack, preserves_flags));
    }
}

fn write_dacr(value: u32) {
    unsafe {
        core::arch::asm!("mcr p15, 0, {0}, c3, c0, 0", in(reg) value, options(nostack, preserves_flags));
    }
}
