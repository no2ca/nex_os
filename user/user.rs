#![allow(unused)]

unsafe fn memset(mut dst: *mut u8, val: u8, count: usize) {
    for _ in 0..count {
        unsafe {
            dst.write_volatile(val);
            dst = dst.add(1);
        }
    }
}
