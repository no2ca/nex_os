use core::arch::asm;

use crate::csr::{Csr, write_csr};
use crate::log_info;
use crate::trap::kernel_entry;
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
        // スタックポインタの設定
        asm!(
            "mv sp, {0}",
            in(reg) &__stack_top as *const u8 as usize,
        );

        // stvecにトラップ時のエントリポイントを設定
        write_csr(Csr::Stvec, kernel_entry as usize);

        // bssセクションのクリア
        let start = &raw const __bss as *const u8;
        let end = &__bss_end as *const u8;
        let size = end.offset_from(start) as usize;
        memset(start as *mut u8, 0, size);

        log_info!("boot", "kernel loaded address\t: {:p}", _start as *const u8);

        log_info!(
            "boot",
            "kernel stack top\t\t: {:p}",
            &__stack_top as *const u8
        );
    }

    crate::main();
}
