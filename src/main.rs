#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]

use core::{arch::asm, panic::PanicInfo};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

unsafe extern "C" {
    static mut __bss: u8;
    static __bss_end: u8;
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

#[repr(C)]
struct Sbiret {
    err: i32,
    value: i32,
}

fn sbi_call(
    arg0: i32,
    arg1: i32,
    arg2: i32,
    arg3: i32,
    arg4: i32,
    arg5: i32,
    fid: i32,
    eid: i32,
) -> Sbiret {
    let err: i32;
    let value: i32;
    unsafe {
        asm!(
            "ecall",
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a3") arg3,
            in("a4") arg4,
            in("a5") arg5,
            in("a6") fid,
            in("a7") eid,
            lateout("a0") err,      // lateoutは全ての入力が消費された後に出力が利用される
            lateout("a1") value,
        );
    }
    Sbiret { err, value }
}

fn write(c: u8) {
    sbi_call(c as i32, 0, 0, 0, 0, 0, 0, 1);
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub extern "C" fn boot() -> ! {
    unsafe {
        let start = &raw const __bss as *const u8;
        let end = &__bss_end as *const u8;
        let size = end.offset_from(start) as usize;
        memset(start as *mut u8, 0xff, size);
    }
    
    let msg = b"Hello World!\n";
    msg.into_iter().for_each(|c| write(*c));

    unsafe {
        loop {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}
