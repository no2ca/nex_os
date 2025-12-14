use crate::println;
use core::ptr;

pub const PAGE_SIZE: usize = 4096;

#[derive(Debug)]
pub enum AllocError {
    OutOfMemory, // メモリが足りない
    OverFlow,    // 確保するアドレスの計算でオーバーフローした
}

pub type AllocResult<T> = Result<T, AllocError>;

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
    pub fn alloc_pages(&mut self, n: usize) -> AllocResult<*const u8> {
        let paddr: *mut u8;
        unsafe {
            let end = &__free_ram_end as *const u8;
            paddr = self.next_paddr as *mut u8;
            let offset = match n.checked_mul(PAGE_SIZE) {
                Some(offset) => offset,
                None => return Err(AllocError::OverFlow),
            };
            self.next_paddr = self.next_paddr.add(offset);
            if self.next_paddr > end {
                return Err(AllocError::OutOfMemory);
            }
            ptr::write_bytes(paddr, 0, n * PAGE_SIZE);
            let free_area =
                (self.next_paddr as usize).saturating_sub(&__free_ram as *const u8 as usize);
            let all_pages = 32 * 1024 * 1024 / PAGE_SIZE;
            println!("[DEBUG] [alloc]");
            println!(
                "\tpages allocated\t\t: {}/{}",
                free_area / PAGE_SIZE,
                all_pages
            );
            println!("\tallocated at\t\t: {:p}", paddr);
        }
        Ok(paddr)
    }
}
