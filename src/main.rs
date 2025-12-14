#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]
#![feature(naked_functions_rustic_abi)]
#![feature(unsafe_cell_access)]

mod alloc;
mod boot;
mod console;
mod csr;
mod proc;
mod trap;
mod utils;

use crate::{
    alloc::{__free_ram, Allocator},
    csr::{read_csr, write_csr},
    proc::{create_process, yield_process},
    trap::kernel_entry,
};
use core::{arch::asm, ptr::NonNull};

fn proc_a() {
    println!("proc_a started");
    loop {
        print!("A");
        yield_process();
        for _ in 0..50_000_000 {
            core::hint::spin_loop();
        }
    }
}

#[unsafe(no_mangle)]
fn proc_b() {
    println!("\nproc_b started");
    loop {
        print!("B");
        yield_process();
        for _ in 0..50_000_000 {
            core::hint::spin_loop();
        }
    }
}

static mut init_sp: proc::StackPointer = proc::StackPointer::null();

fn dump_main_info() {
    println!(
        "[INFO ] [mem] kernel_entry\t\t: {:p}",
        kernel_entry as *const u8
    );
    println!("[INFO ] [reg] stvec register\t\t: {:#x}", read_csr("stvec"));
    unsafe {
        println!("[INFO ] [mem] free ram start\t\t: {:p}", &__free_ram);
    }
}

fn main() {
    unsafe {
        write_csr("stvec", kernel_entry as usize);
    }

    dump_main_info();

    // アロケータの初期化とメモリ確保のテスト
    let mut allocator = Allocator::new();
    let paddr0 = allocator.alloc_pages(2).unwrap();
    let paddr1 = allocator.alloc_pages(1).unwrap();
    println!("[TEST ] [alloc] alloc_pages(2)\t\t: {:p}", paddr0);
    println!("[TEST ] [alloc] alloc_pages(1)\t\t: {:p}", paddr1);

    // 書き込みできる範囲のテスト
    let ptr_low = 0x80050000 as *mut u8;
    unsafe {
        let val = ptr_low.read_volatile();
        println!("read from {:p} pointer: {}", ptr_low, val);
    }

    let ptr_high = 0x87ffffff as *mut u8;
    unsafe {
        let val = ptr_high.read_volatile();
        println!("read from {:p} pointer: {}", ptr_high, val);
    }

    // プロセスの作成とコンテキストスイッチのテスト
    create_process(&mut allocator, &raw const init_sp as usize);
    unsafe {
        // PROCS から該当プロセスの参照を取り、NonNull を作る
        let procs = proc::PROCS.get().as_mut().unwrap();
        let proc_ref = procs.procs[0].as_mut().unwrap();
        *proc::current_proc.get() = Some(NonNull::from(proc_ref));
    }
    create_process(&mut allocator, proc_a as usize);
    create_process(&mut allocator, proc_b as usize);

    yield_process();

    unsafe {
        let sp: usize;
        asm!("mv {0}, sp", out(reg) sp);
        println!("sp: {:p}", sp as *const u8);
    }

    unsafe {
        let ptr = 0xdeadbeef as *mut u8;
        ptr.write_volatile(0x42);

        loop {
            core::hint::spin_loop();
        }
    }
}
