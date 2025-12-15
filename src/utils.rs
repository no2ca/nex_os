use crate::println;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[inline(always)]
pub unsafe fn memset(mut dst: *mut u8, val: u8, count: usize) {
    for _ in 0..count {
        unsafe {
            dst.write_volatile(val);
            dst = dst.add(1);
        }
    }
}

pub fn is_aligned(addr: usize, align: usize) -> bool {
    addr % align == 0
}

pub fn align_up(addr: usize, align: usize) -> usize {
    if is_aligned(addr, align) {
        addr
    } else {
        addr + (align - (addr % align))
    }
}
