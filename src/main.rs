#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]
#![feature(naked_functions_rustic_abi)]

mod alloc;
mod boot;
mod console;
mod csr;
mod trap;

use crate::{
    alloc::{__free_ram, Allocator, PAGE_SIZE},
    console::memset,
    csr::{read_csr, write_csr},
    trap::kernel_entry,
};
use core::{arch::asm, panic::PanicInfo};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

fn main() {
    println!("kernel_entry\t\t: {:p}", kernel_entry as *const u8);

    write_csr("stvec", kernel_entry as usize);
    println!("stvec register\t\t: {:x}", read_csr("stvec"));

    unsafe {
        println!("free ram start\t\t: {:p}", &__free_ram);
    }

    let mut allocator = Allocator {
        next_paddr: unsafe { &__free_ram as *const u8 },
    };
    let paddr0 = allocator.alloc_pages(1024);
    let paddr1 = allocator.alloc_pages(1);
    println!("alloc_pages(2) test\t: {:p}", paddr0);
    println!("alloc_pages(1) test\t: {:p}", paddr1);
    if unsafe { (&__free_ram as *const u8).add(PAGE_SIZE * 1024) } == paddr1 {
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
