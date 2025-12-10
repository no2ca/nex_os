#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]
#![feature(naked_functions_rustic_abi)]

use core::{arch::{asm, naked_asm}, panic::PanicInfo};
use nex::{print, println, memset};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

unsafe extern "C" {
    static mut __bss: u8;
    static __bss_end: u8;
    static __stack_top: u8;
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
fn kernel_entry() {
    naked_asm!(
        "csrw sscratch, sp",
        "addi sp, sp, -4 * 31",
        "sw ra,  4 * 0(sp)",
        "sw gp,  4 * 1(sp)",
        "sw tp,  4 * 2(sp)",
        "sw t0,  4 * 3(sp)",
        "sw t1,  4 * 4(sp)",
        "sw t2,  4 * 5(sp)",
        "sw t3,  4 * 6(sp)",
        "sw t4,  4 * 7(sp)",
        "sw t5,  4 * 8(sp)",
        "sw t6,  4 * 9(sp)",
        "sw a0,  4 * 10(sp)",
        "sw a1,  4 * 11(sp)",
        "sw a2,  4 * 12(sp)",
        "sw a3,  4 * 13(sp)",
        "sw a4,  4 * 14(sp)",
        "sw a5,  4 * 15(sp)",
        "sw a6,  4 * 16(sp)",
        "sw a7,  4 * 17(sp)",
        "sw s0,  4 * 18(sp)",
        "sw s1,  4 * 19(sp)",
        "sw s2,  4 * 20(sp)",
        "sw s3,  4 * 21(sp)",
        "sw s4,  4 * 22(sp)",
        "sw s5,  4 * 23(sp)",
        "sw s6,  4 * 24(sp)",
        "sw s7,  4 * 25(sp)",
        "sw s8,  4 * 26(sp)",
        "sw s9,  4 * 27(sp)",
        "sw s10, 4 * 28(sp)",
        "sw s11, 4 * 29(sp)",

        "csrr a0, sscratch",
        "sw a0, 4 * 30(sp)",

        "mv a0, sp",
        "call handle_trap",

        "lw ra,  4 * 0(sp)",
        "lw gp,  4 * 1(sp)",
        "lw tp,  4 * 2(sp)",
        "lw t0,  4 * 3(sp)",
        "lw t1,  4 * 4(sp)",
        "lw t2,  4 * 5(sp)",
        "lw t3,  4 * 6(sp)",
        "lw t4,  4 * 7(sp)",
        "lw t5,  4 * 8(sp)",
        "lw t6,  4 * 9(sp)",
        "lw a0,  4 * 10(sp)",
        "lw a1,  4 * 11(sp)",
        "lw a2,  4 * 12(sp)",
        "lw a3,  4 * 13(sp)",
        "lw a4,  4 * 14(sp)",
        "lw a5,  4 * 15(sp)",
        "lw a6,  4 * 16(sp)",
        "lw a7,  4 * 17(sp)",
        "lw s0,  4 * 18(sp)",
        "lw s1,  4 * 19(sp)",
        "lw s2,  4 * 20(sp)",
        "lw s3,  4 * 21(sp)",
        "lw s4,  4 * 22(sp)",
        "lw s5,  4 * 23(sp)",
        "lw s6,  4 * 24(sp)",
        "lw s7,  4 * 25(sp)",
        "lw s8,  4 * 26(sp)",
        "lw s9,  4 * 27(sp)",
        "lw s10, 4 * 28(sp)",
        "lw s11, 4 * 29(sp)",
        "lw sp,  4 * 30(sp)",
        "sret",
    )
}

#[allow(unused)]
#[unsafe(no_mangle)]
fn handle_trap(_trap_frame: *const u8) -> ! {
    let scause = read_csr("scause");
    let stval = read_csr("stval");
    let user_pc = read_csr("sepc");
    panic!("unexpected trap: scause={:x}, stval={:x}, sepc={:x}", scause, stval, user_pc);
}

fn read_csr(csr: &str) -> usize {
    let value: usize;
    match csr {
        "stvec" =>
            unsafe {
                asm!(
                    "csrr {0}, stvec",
                    out(reg) value,
                );
            }
        "scause" =>
            unsafe {
                asm!(
                    "csrr {0}, scause",
                    out(reg) value,
                );
            }
        "stval" =>
            unsafe {
                asm!(
                    "csrr {0}, stval",
                    out(reg) value,
                );
            }
        "sepc" =>
            unsafe {
                asm!(
                    "csrr {0}, sepc",
                    out(reg) value,
                );
            }
        _ => panic!("unreachable"),
    }
    value
}

fn write_csr(csr: &str, value: usize) {
    match csr {
        "stvec" =>
            unsafe {
                asm!(
                    "csrw stvec, {0}",
                    in(reg) value,
                );
            }
        _ => panic!("unreachable"),
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text._start")]
pub extern "C" fn _start() {
    // スタックポインタの初期化
    unsafe {
        asm!(
            "mv sp, {0}",
            in(reg) &__stack_top as *const u8 as usize,
        );
    }

    // .bssセクションは初期化されていないグローバル変数が置かれる
    // コンパイラは0であることを想定しているため0でクリアする
    unsafe {
        let start = &raw const __bss as *const u8;
        let end = &__bss_end as *const u8;
        let size = end.offset_from(start) as usize;
        memset(start as *mut u8, 0, size);
    }

    unsafe {
        println!("\nKernel loaded address\t: {:p}", _start as *const u8);
        println!("Kernel stack top\t: {:p}", &__stack_top as *const u8);
    }

    main();
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
