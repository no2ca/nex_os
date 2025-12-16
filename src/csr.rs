use core::arch::asm;

#[derive(Copy, Clone, Debug)]
pub enum Csr {
    Stvec,
    Scause,
    Stval,
    Sepc,
    Sscratch,
}

pub fn read_csr(csr: Csr) -> usize {
    let value: usize;
    match csr {
        Csr::Stvec => unsafe {
            asm!(
                "csrr {0}, stvec",
                out(reg) value,
            );
        },
        Csr::Scause => unsafe {
            asm!(
                "csrr {0}, scause",
                out(reg) value,
            );
        },
        Csr::Stval => unsafe {
            asm!(
                "csrr {0}, stval",
                out(reg) value,
            );
        },
        Csr::Sepc => unsafe {
            asm!(
                "csrr {0}, sepc",
                out(reg) value,
            );
        },

        Csr::Sscratch => unsafe {
            asm!(
                "csrr {0}, sscratch",
                out(reg) value,
            );
        },
    }
    value
}

pub unsafe fn write_csr(csr: Csr, value: usize) {
    match csr {
        Csr::Stvec => unsafe {
            asm!(
                "csrw stvec, {0}",
                in(reg) value,
            );
        },

        Csr::Sscratch => unsafe {
            asm!(
                "csrw sscratch, {0}",
                in(reg) value,
            );
        },
        other => panic!("csr {:?} is not writable", other),
    }
}
