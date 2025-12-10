#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]

use core::{arch::asm, panic::PanicInfo};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".bss")]
#[used]
static mut bss_placeholder: [u8; 24] = [0; 24];

unsafe extern "C" {
    static mut __bss: u8;
    static __bss_end: u8;
}

unsafe fn memset(mut dst: *mut u8, val: u8, count: usize) {
    for _ in 0..count {
        unsafe { 
            dst.write_volatile(val);
            dst = dst.add(1);
        }
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub extern "C" fn boot() -> ! {
    unsafe {
        let start = &raw const __bss as *const u8;
        let end = &__bss_end as *const u8;
        let size = end.offset_from(start) as usize;
        memset(start as *mut u8, 0xff, size);

        let ptr = &raw mut __bss as *mut u8;
        ptr.write_volatile(size as u8);
        if start == ptr {
            ptr.add(1).write_volatile(0xaa);
        } else {
            ptr.add(1).write_volatile(0xbb);
        }

        loop {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}
