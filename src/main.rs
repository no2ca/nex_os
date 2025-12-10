#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]

use core::{arch::asm, panic::PanicInfo};
use nex::{print, println};

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

#[inline(always)]
unsafe fn memset(mut dst: *mut u8, val: u8, count: usize) {
    for _ in 0..count {
        unsafe {
            dst.write_volatile(val);
            dst = dst.add(1);
        }
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
        println!("\nEntry kernel _start()");
        println!("Kernel stack top (pys)\t: {:p}", &__stack_top as *const u8);
    }

    main();
}

fn main() {
    unsafe {
        loop {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}
