#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]
#![feature(naked_functions_rustic_abi)]
#![feature(unsafe_cell_access)]

mod alloc;
mod boot;
mod console;
mod csr;
mod ksyscall;
mod loadelf;
mod mem;
mod procv2;
mod trap;
mod utils;

use crate::{
    alloc::{__free_ram, Allocator},
    csr::{Csr, read_csr},
    trap::kernel_entry,
};

#[unsafe(no_mangle)]
pub static SHELL_ELF: &[u8] = include_bytes!("../../shell.elf");

fn dump_main_info() {
    println!(
        "[main_info] kernel_entry\t\t: {:p}",
        kernel_entry as *const u8
    );
    println!(
        "[main_info] stvec register\t\t: {:#x}",
        read_csr(Csr::Stvec)
    );
    unsafe {
        println!("[main_info] free ram start\t\t: {:p}", &__free_ram);
    }
}

fn main() {
    dump_main_info();
    let mut allocator = Allocator::new();
    procv2::create_idle_process(&mut allocator);
    procv2::create_process(SHELL_ELF, &mut allocator);
    procv2::dump_process_list();
    procv2::start_process();
    unreachable!()
}
