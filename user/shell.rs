#![no_std]
#![no_main]
use core::{arch::asm, panic::PanicInfo};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

unsafe extern "C" {
    static __stack_top: u8;
    static mut __bss: u8;
    static __bss_end: u8;
}

#[unsafe(link_section = ".bss")]
#[used]
static mut _HOGE: [u8; 24] = [0; 24];

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.start")]
pub extern "C" fn start() {
    unsafe {
        asm!(
            "mv sp, {0}",
            in(reg) &__stack_top as *const u8 as usize,
        );
    }
    unsafe {
        _HOGE[0] = 1;
    }
    loop {
        core::hint::spin_loop();
    }
}
