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

fn main() {
    println!("kernel_entry\t\t: {:p}", kernel_entry as *const u8);
    write_csr("stvec", kernel_entry as usize);
    println!("stvec\t\t\t: {:x}", read_csr("stvec"));

    unsafe {
        let ptr = 0xdeadbeef as *mut u8;
        ptr.write_volatile(0x42);

        loop {
            asm!("wfi");
        }
    }
}
