#![allow(dead_code)]

use core::mem::{align_of, size_of};
use core::ptr::NonNull;

use crate::kernel::memory::{self, PAGE_SIZE};

pub const SLAB_CLASS_COUNT: usize = 8;
pub const SLAB_CLASSES: [usize; SLAB_CLASS_COUNT] = [16, 32, 64, 128, 256, 512, 1024, 2048];

#[derive(Clone, Copy)]
pub struct SlabStats {
    pub class_size: usize,
    pub pages: usize,
    pub objects: usize,
    pub allocated: usize,
    pub free: usize,
}

#[derive(Clone, Copy)]
struct FreeObject {
    next: Option<NonNull<FreeObject>>,
}

#[derive(Clone, Copy)]
struct SlabClass {
    size: usize,
    free_head: Option<NonNull<FreeObject>>,
    pages: usize,
    objects: usize,
    allocated: usize,
}

impl SlabClass {
    const fn new(size: usize) -> Self {
        Self {
            size,
            free_head: None,
            pages: 0,
            objects: 0,
            allocated: 0,
        }
    }

    fn free_count(self) -> usize {
        self.objects.saturating_sub(self.allocated)
    }
}

pub struct SlabAllocator {
    classes: [SlabClass; SLAB_CLASS_COUNT],
}

impl SlabAllocator {
    pub const fn new() -> Self {
        Self {
            classes: [
                SlabClass::new(16),
                SlabClass::new(32),
                SlabClass::new(64),
                SlabClass::new(128),
                SlabClass::new(256),
                SlabClass::new(512),
                SlabClass::new(1024),
                SlabClass::new(2048),
            ],
        }
    }

    pub fn alloc(&mut self, layout_size: usize, layout_align: usize) -> Option<NonNull<u8>> {
        let class = self.class_mut(layout_size, layout_align)?;
        if class.free_head.is_none() {
            grow_class(class)?;
        }

        let object = class.free_head?;
        let next = unsafe { object.as_ref().next };
        class.free_head = next;
        class.allocated += 1;
        Some(object.cast())
    }

    pub unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout_size: usize, layout_align: usize) {
        let Some(class) = self.class_mut(layout_size, layout_align) else {
            panic!("slab: invalid dealloc layout");
        };

        let object = ptr.cast::<FreeObject>();
        unsafe {
            object.as_ptr().write(FreeObject {
                next: class.free_head,
            });
        }
        class.free_head = Some(object);
        class.allocated = class.allocated.saturating_sub(1);
    }

    pub fn stats(&self, index: usize) -> Option<SlabStats> {
        let class = self.classes.get(index).copied()?;
        Some(SlabStats {
            class_size: class.size,
            pages: class.pages,
            objects: class.objects,
            allocated: class.allocated,
            free: class.free_count(),
        })
    }

    fn class_mut(&mut self, layout_size: usize, layout_align: usize) -> Option<&mut SlabClass> {
        let size = layout_size.max(size_of::<FreeObject>());
        let align = layout_align.max(align_of::<FreeObject>());
        self.classes
            .iter_mut()
            .find(|class| class.size >= size && class.size % align == 0)
    }
}

fn grow_class(class: &mut SlabClass) -> Option<()> {
    let page = memory::alloc_pages(1)?;
    let objects = PAGE_SIZE / class.size;
    if objects == 0 {
        unsafe {
            memory::free_pages(page, 1);
        }
        return None;
    }

    for index in 0..objects {
        let ptr = unsafe { page.add(index * class.size) }.cast::<FreeObject>();
        unsafe {
            ptr.write(FreeObject {
                next: class.free_head,
            });
        }
        class.free_head = NonNull::new(ptr);
    }

    class.pages += 1;
    class.objects += objects;
    Some(())
}

static mut SLAB_ALLOCATOR: SlabAllocator = SlabAllocator::new();

pub fn alloc(size: usize, align: usize) -> Option<NonNull<u8>> {
    crate::arch::aarch32::cpu::with_irq_disabled(|| unsafe {
        let allocator = &raw mut SLAB_ALLOCATOR;
        (*allocator).alloc(size, align)
    })
}

pub unsafe fn dealloc(ptr: NonNull<u8>, size: usize, align: usize) {
    crate::arch::aarch32::cpu::with_irq_disabled(|| unsafe {
        let allocator = &raw mut SLAB_ALLOCATOR;
        (*allocator).dealloc(ptr, size, align);
    });
}

pub fn stats(index: usize) -> Option<SlabStats> {
    crate::arch::aarch32::cpu::with_irq_disabled(|| unsafe {
        let allocator = &raw const SLAB_ALLOCATOR;
        (*allocator).stats(index)
    })
}
