#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]
#![feature(naked_functions_rustic_abi)]

mod console;
mod boot;
mod csr;
mod trap;

use core::{arch::asm, panic::PanicInfo};
use crate::{
    console::memset,
    csr::{read_csr, write_csr},
    trap::kernel_entry,
};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

const PAGE_SIZE: usize = 4096;

unsafe extern "C" {
    static __free_ram: u8;
    static __free_ram_end: u8;
}

struct Allocator {
    next_paddr: *const u8,
}

impl Allocator {
    /// nページ分のメモリを割り当てて、その先頭アドレスを返す
    fn alloc_pages(&mut self, n: usize) -> *const u8 {
        let paddr: *mut u8;
        unsafe {
            paddr = self.next_paddr as *mut u8;
            self.next_paddr = self.next_paddr.add(n * PAGE_SIZE);
            if self.next_paddr > &__free_ram_end as *const u8 {
                panic!("out of memory")
            }
            memset(paddr, 0, n * PAGE_SIZE);
        }
        paddr
    }
}

fn main() {
    println!("kernel_entry\t\t: {:p}", kernel_entry as *const u8);

    write_csr("stvec", kernel_entry as usize);
    println!("stvec register\t\t: {:x}", read_csr("stvec"));
    
    unsafe { println!("free ram start\t\t: {:p}", &__free_ram); }
    
    let mut allocator = Allocator { 
        next_paddr: unsafe { &__free_ram as *const u8 } 
    };
    let paddr0 = allocator.alloc_pages(2);
    let paddr1 = allocator.alloc_pages(1);
    println!("alloc_pages(2) test\t: {:p}", paddr0);
    println!("alloc_pages(1) test\t: {:p}", paddr1);
    if unsafe { (&__free_ram as *const u8).add(PAGE_SIZE * 2) } == paddr1 {
        println!("Page allocation OK");
    }

    unsafe {
        let ptr = 0xdeadbeef as *mut u8;
        ptr.write_volatile(0x42);

        loop {
            asm!("wfi");
        }
    }
}
