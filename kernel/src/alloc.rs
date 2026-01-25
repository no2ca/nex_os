use core::{ptr, slice};

pub const PAGE_SIZE: usize = 4096;

unsafe extern "C" {
    pub static __free_ram: u8;
    pub static __free_ram_end: u8;
}

pub struct Allocator {
    next_paddr: *const u8,
}

impl Allocator {
    pub fn new() -> Self {
        Allocator {
            next_paddr: unsafe { &__free_ram as *const u8 },
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
            // 確保する分を足して次の始点を更新
            self.next_paddr = self.next_paddr.add(offset);
            let end = &__free_ram_end as *const u8;
            if self.next_paddr > end {
                panic!("Out of memory!")
            }

            // 確保する領域をゼロクリアする
            let start = self.next_paddr as *mut T;
            ptr::write_bytes(start, 0, offset);
            slice::from_raw_parts_mut(start, offset)
        }
    }
}
