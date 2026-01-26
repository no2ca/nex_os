#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]
#![feature(naked_functions_rustic_abi)]
#![feature(unsafe_cell_access)]

mod allocator;
mod boot;
mod console;
mod csr;
mod ksyscall;
mod loadelf;
mod mem;
mod proc;
mod trap;
mod utils;
mod vfs;

use crate::{
    allocator::{__free_ram, Allocator},
    csr::{Csr, read_csr},
    trap::kernel_entry,
    vfs::{Fs, Node},
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

static mut buf: [u8; 1024 * 1024 * 6] = [0u8; 1024 * 1024 * 6];
fn test_vfs<F: Fs>(fs: F) {
    let node: F::NodeType = fs.lookup("shell").unwrap();
    unsafe {
        node.read(&mut *&raw mut buf).unwrap();
    }
    println!("[test_vfs] id={:?}", node.get_id());
}

fn main() {
    dump_main_info();
    test_vfs(vfs::MemoryFs);

    // ALLOC.init_heap();
    let mut allocator = Allocator::new();
    proc::create_idle_process(&mut allocator);
    unsafe { proc::create_process(&*&raw const buf, &mut allocator) };
    proc::dump_process_list(false);
    proc::start_process();

    unreachable!()
}
