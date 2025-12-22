#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]
#![feature(naked_functions_rustic_abi)]
#![feature(unsafe_cell_access)]

mod alloc;
mod boot;
mod console;
mod csr;
mod loadelf;
mod procv2;
mod trap;
mod utils;
mod vmem;

use crate::{
    alloc::{__free_ram, Allocator},
    csr::{Csr, read_csr, write_csr},
    trap::kernel_entry,
};

#[unsafe(no_mangle)]
pub static SHELL_ELF: &[u8] = include_bytes!("../shell.elf");

fn dump_main_info() {
    println!("[main_info] kernel_entry\t\t: {:p}", kernel_entry as *const u8);
    println!("[main_info] stvec register\t\t: {:#x}", read_csr(Csr::Stvec));
    unsafe {
        println!("[main_info] free ram start\t\t: {:p}", &__free_ram);
    }
}

/// OpenSBIのメモリ保護機能(PMP)の動作確認用関数
///
/// 0x80050000 から 0x87ffffff までの範囲が読み取り可能であることを確認する
fn test_read_limit() {
    let ptr_low = 0x80050000 as *mut u8;
    unsafe {
        let val = ptr_low.read_volatile();
        println!("[pmp] read from {:p} pointer: {}", ptr_low, val);
    }

    let ptr_high = 0x87ffffff as *mut u8;
    unsafe {
        let val = ptr_high.read_volatile();
        println!("[pmp] read from {:p} pointer: {}", ptr_high, val);
    }
}

/// allocatorでページを確保するテスト関数
///
/// 2ページと1ページを確保してアドレスを表示する
fn test_allocator(allocator: &mut Allocator) {
    let paddr0 = allocator.alloc_pages(2).unwrap();
    let paddr1 = allocator.alloc_pages(1).unwrap();
    println!("[alloc] alloc_pages(2)\t\t: {:p}", paddr0);
    println!("[alloc] alloc_pages(1)\t\t: {:p}", paddr1);
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

// procv2のテスト
fn test_process(allocator: &mut Allocator) {
    // TODO: initプロセスの実装
    procv2::create_process(SHELL_ELF, allocator);
    procv2::create_process(SHELL_ELF, allocator);
    procv2::dump_process_list();
    procv2::test_proc_switch();
}

fn main() {
    // stvecにトラップ時のエントリポイントを設定
    unsafe {
        write_csr(Csr::Stvec, kernel_entry as usize);
    }

    // 初期情報の表示
    dump_main_info();

    // Allocatorの初期化
    let mut allocator = Allocator::new();

    // Allocatorのテスト
    test_allocator(&mut allocator);

    // 読み取りできる範囲のテスト
    test_read_limit();

    // プロセスの起動
    test_process(&mut allocator);

    // 未割当メモリへの書き込みテスト
    test_memory_exception();

    loop {
        core::hint::spin_loop();
    }
}
