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
    proc::{create_process, dump_process_list, yield_process},
    trap::kernel_entry,
};

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

static mut idle_proc: proc::StackPointer = proc::StackPointer::null();

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

/// OpenSBIのメモリ保護機能(PMP)の動作確認用関数
/// 
/// 0x80050000 から 0x87ffffff までの範囲が読み取り可能であることを確認する
fn test_read_limit() {
    let ptr_low = 0x80050000 as *mut u8;
    unsafe {
        let val = ptr_low.read_volatile();
        println!("[TEST ] [PMP] read from {:p} pointer: {}", ptr_low, val);
    }

    let ptr_high = 0x87ffffff as *mut u8;
    unsafe {
        let val = ptr_high.read_volatile();
        println!("[TEST ] [PMP] read from {:p} pointer: {}", ptr_high, val);
    }
}

/// allocatorでページを確保するテスト関数
/// 
/// 2ページと1ページを確保してアドレスを表示する
fn test_allocator(allocator: &mut Allocator) {
    let paddr0 = allocator.alloc_pages(2).unwrap();
    let paddr1 = allocator.alloc_pages(1).unwrap();
    println!("[TEST ] [alloc] alloc_pages(2)\t\t: {:p}", paddr0);
    println!("[TEST ] [alloc] alloc_pages(1)\t\t: {:p}", paddr1);
}

/// プロセスの作成とコンテキストスイッチのテスト関数
/// 
/// init_spを持つプロセスとproc_a, proc_bを持つプロセスを作成し, proc_aから実行を開始する
fn test_proc_switch(allocator: &mut Allocator) {
    create_process(allocator, &raw const idle_proc as usize);
    create_process(allocator, proc_a as usize);
    create_process(allocator, proc_b as usize);
    dump_process_list();
    yield_process();
}

/// 未割当メモリへの書き込みを試みるテスト関数
/// 
/// 0xdeadbeef アドレスに書き込みを試み, メモリ例外が発生することを確認する
fn test_memory_exception() {
    unsafe {
        let ptr = 0xdeadbeef as *mut u8;
        ptr.write_volatile(0x42);
    }
}

fn main() {
    // stvecにトラップ時のエントリポイントを設定
    unsafe {
        write_csr("stvec", kernel_entry as usize);
    }

    // 初期情報の表示
    dump_main_info();

    // Allocatorの初期化
    let mut allocator = Allocator::new();
    
    // Allocatorのテスト
    test_allocator(&mut allocator);

    // 読み取りできる範囲のテスト
    test_read_limit();

    // プロセスの作成とコンテキストスイッチのテスト
    test_proc_switch(&mut allocator);    

    // 未割当メモリへの書き込みテスト
    test_memory_exception();
    
    loop {
        core::hint::spin_loop();
    }
}
