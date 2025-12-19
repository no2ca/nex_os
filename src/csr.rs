use core::arch::asm;

#[derive(Copy, Clone, Debug)]
pub enum Csr {
    Stvec,
    Scause,
    Stval,
    Sepc,
    Sscratch,
    Satp,
}

macro_rules! read_csr_asm {
    ($value:ident, $csr:literal) => {
        asm!(
            concat!("csrr {0}, ", $csr),
            out(reg) $value,
        )
    };
}

macro_rules! write_csr_asm {
    ($csr:literal, $value:ident) => {
        asm!(
            concat!("csrw ", $csr, ", {0}"),
            in(reg) $value,
        )
    }
}

#[inline(always)]
pub fn read_csr(csr: Csr) -> usize {
    let value: usize;
    match csr {
        Csr::Stvec => unsafe { read_csr_asm!(value, "stvec") },
        Csr::Scause => unsafe { read_csr_asm!(value, "scause") },
        Csr::Stval => unsafe { read_csr_asm!(value, "stval") },
        Csr::Sepc => unsafe { read_csr_asm!(value, "sepc") },
        Csr::Sscratch => unsafe { read_csr_asm!(value, "sscratch") },
        Csr::Satp => unsafe { read_csr_asm!(value, "satp") },
    }
    value
}

/// # Panics
/// stvec, sscratch, satp 以外のレジスタへ書き込もうとすると panic する
#[inline(always)]
pub unsafe fn write_csr(csr: Csr, value: usize) {
    match csr {
        Csr::Stvec => unsafe { write_csr_asm!("stvec", value) },
        Csr::Sscratch => unsafe { write_csr_asm!("sscratch", value) },
        Csr::Satp => unsafe {
            asm!(
                "sfence.vma",
                "csrw satp, {0}",
                "sfence.vma",
                in(reg) value,
            );
        },
        other => panic!("csr {:?} is not writable", other),
    }
}
