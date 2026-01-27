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
mod timer;
mod trap;
mod utils;
mod vfs;

use core::slice;

use crate::{
    allocator::PAGE_SIZE,
    csr::{Csr, read_csr},
    trap::kernel_entry,
    vfs::{Fs, Node},
};

#[unsafe(no_mangle)]
pub static SH_ELF: &[u8] = include_bytes!("../../sh.elf");
pub static PS_ELF: &[u8] = include_bytes!("../../ps.elf");

fn dump_main_info() {
    println!(
        "[main_info] kernel_entry\t\t: {:p}",
        kernel_entry as *const u8
    );
    println!(
        "[main_info] stvec register\t\t: {:#x}",
        read_csr(Csr::Stvec)
    );
}

fn test_vfs<'a, F: Fs>(fs: F) -> &'a mut [u8] {
    let node: F::NodeType = fs.lookup("sh").unwrap();
    let n = node.size().div_ceil(PAGE_SIZE);
    let buf_ptr = allocator::PAGE_ALLOC.alloc_pages::<u8>(n).as_mut_ptr();
    let buf = unsafe { slice::from_raw_parts_mut(buf_ptr, n * PAGE_SIZE) };
    node.read(buf).unwrap();
    println!("[test_vfs] id={:?}", node.get_id());
    buf
}

fn main() {
    dump_main_info();

    allocator::ALLOC.init_heap();

    proc::create_idle_process();
    let buf = test_vfs(vfs::MemoryFs);
    proc::create_process(buf);

    proc::dump_process_list(false);
    proc::start_process();
    unreachable!()
}
