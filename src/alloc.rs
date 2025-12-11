use crate::{console::memset, println};

pub const PAGE_SIZE: usize = 4096;

unsafe extern "C" {
    pub static __free_ram: u8;
    pub static __free_ram_end: u8;
}

pub struct Allocator {
    pub(crate) next_paddr: *const u8,
}

impl Allocator {
    /// nページ分のメモリを割り当てて、その先頭アドレスを返す
    pub fn alloc_pages(&mut self, n: usize) -> *const u8 {
        let paddr: *mut u8;
        unsafe {
            paddr = self.next_paddr as *mut u8;
            self.next_paddr = self.next_paddr.add(n * PAGE_SIZE);
            if self.next_paddr > &__free_ram_end as *const u8 {
                panic!("out of memory")
            }
            memset(paddr, 0, n * PAGE_SIZE);
            println!("[alloc_pages]");
            let free_area = (&__free_ram_end as *const u8).offset_from(self.next_paddr);
            let ram_size = 64 * 1024 * 1024;
            let all_pages = ram_size / 4096;
            println!(
                "\tremaining pages: {} / {}",
                free_area / PAGE_SIZE as isize,
                all_pages
            );
            println!(
                "\tused (%): {}",
                ((ram_size - free_area) * 100 / ram_size) + 1
            );
        }
        paddr
    }
}
