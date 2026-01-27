use core::arch::asm;

// QEMU (virt) timebase is typically 10MHz on RISC-V.
// Update this if your platform reports a different timebase.
pub const TIMEBASE_HZ: u64 = 10_000_000;

const fn digits_u64(mut n: u64) -> usize {
    let mut digits = 1usize;
    while n >= 10 {
        n /= 10;
        digits += 1;
    }
    digits
}

pub const TIMEBASE_DECIMALS: usize = digits_u64(TIMEBASE_HZ - 1);

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

#[inline(always)]
pub fn read_time_seconds() -> u64 {
    read_time() / TIMEBASE_HZ
}

#[inline(always)]
pub fn read_time_parts() -> (u64, u64) {
    let ticks = read_time();
    (ticks / TIMEBASE_HZ, ticks % TIMEBASE_HZ)
}
