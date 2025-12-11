use core::arch::asm;

pub fn read_csr(csr: &str) -> usize {
    let value: usize;
    match csr {
        "stvec" =>
            unsafe {
                asm!(
                    "csrr {0}, stvec",
                    out(reg) value,
                );
            }
        "scause" =>
            unsafe {
                asm!(
                    "csrr {0}, scause",
                    out(reg) value,
                );
            }
        "stval" =>
            unsafe {
                asm!(
                    "csrr {0}, stval",
                    out(reg) value,
                );
            }
        "sepc" =>
            unsafe {
                asm!(
                    "csrr {0}, sepc",
                    out(reg) value,
                );
            }
        _ => panic!("unreachable"),
    }
    value
}

pub fn write_csr(csr: &str, value: usize) {
    match csr {
        "stvec" =>
            unsafe {
                asm!(
                    "csrw stvec, {0}",
                    in(reg) value,
                );
            }
        _ => panic!("unreachable"),
    }
}
