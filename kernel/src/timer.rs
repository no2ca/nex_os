use core::arch::asm;

#[allow(unused)]
#[inline(always)]
pub fn read_time() -> u64 {
    let value: u64;
    unsafe {
        asm!(
            "rdtime {0}",
            out(reg) value,
        );
    }
    value
}
