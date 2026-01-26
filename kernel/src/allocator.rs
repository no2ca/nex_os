use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    ptr, slice,
};
pub const PAGE_SIZE: usize = 4096;
extern crate alloc;

unsafe extern "C" {
    pub static __page_area_start: u8;
    pub static __page_area_end: u8;
    pub static __heap_start: u8;
    pub static __heap_end: u8;
}

pub struct PageAllocator {
    next_paddr: *const u8,
}

impl PageAllocator {
    pub fn new() -> Self {
        PageAllocator {
            next_paddr: unsafe { &__page_area_start as *const u8 },
        }
    }

    /// nページ分のメモリを割り当てて、その先頭アドレスを返す
    pub fn alloc_pages<T>(&mut self, n: usize) -> &mut [T] {
        // 確保するバイト数の計算
        let offset = match n.checked_mul(PAGE_SIZE) {
            Some(offset) => offset,
            None => panic!("Page calculation overflowed!"),
        };

        unsafe {
            // 現在の先頭を確保対象として保持しておく
            let start_paddr = self.next_paddr;
            // 確保する分を足して次の始点を更新
            self.next_paddr = self.next_paddr.add(offset);
            let end = &__page_area_end as *const u8;
            if self.next_paddr > end {
                panic!("Out of memory!")
            }

            // 確保する領域をゼロクリアする
            let start = start_paddr as *mut T;
            let count = offset / size_of::<T>();
            ptr::write_bytes(start, 0, count);
            slice::from_raw_parts_mut(start, count)
        }
    }
}

#[global_allocator]
pub static ALLOC: BumpPointerAlloc = BumpPointerAlloc::uninit();

pub struct BumpPointerAlloc {
    head: UnsafeCell<usize>,
    end: UnsafeCell<usize>,
}

impl BumpPointerAlloc {
    const fn uninit() -> Self {
        Self {
            head: UnsafeCell::new(0),
            end: UnsafeCell::new(0),
        }
    }

    pub fn init_heap(&self) {
        unsafe {
            let head = &__heap_start as *const _ as usize;
            let end = &__heap_end as *const _ as usize;
            *self.head.get() = head.into();
            *self.end.get() = end.into();
        }
    }
}

unsafe impl Sync for BumpPointerAlloc {}

unsafe impl GlobalAlloc for BumpPointerAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let head = self.head.get();

        let align = layout.align();
        unsafe {
            let res = *head % align;
            let start = if res == 0 { *head } else { *head + align - res };
            if start + align > *self.end.get() {
                ptr::null_mut()
            } else {
                *head = start + align;
                start as *mut u8
            }
        }
    }

    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
}
