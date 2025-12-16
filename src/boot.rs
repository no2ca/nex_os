use core::arch::asm;

use crate::println;
use crate::utils::memset;

unsafe extern "C" {
    static mut __bss: u8;
    static __bss_end: u8;
    static __stack_top: u8;
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text._start")]
pub extern "C" fn _start() {
    unsafe {
        asm!(
            "mv sp, {0}",
            in(reg) &__stack_top as *const u8 as usize,
        );
    }

    unsafe {
        let start = &raw const __bss as *const u8;
        let end = &__bss_end as *const u8;
        let size = end.offset_from(start) as usize;
        memset(start as *mut u8, 0, size);
    }

    unsafe {
        println!(
            "\n[mem] Kernel loaded address\t: {:p}",
            _start as *const u8
        );
        println!(
            "[mem] Kernel stack top\t\t: {:p}",
            &__stack_top as *const u8
        );
    }

    crate::main();
}
