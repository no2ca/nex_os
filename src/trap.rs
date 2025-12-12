use core::arch::naked_asm;

use crate::csr::read_csr;

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub fn kernel_entry() {
    naked_asm!(
        "csrw sscratch, sp",
        "addi sp, sp, -8 * 31",
        "sw ra,  8 * 0(sp)",
        "sw gp,  8 * 1(sp)",
        "sw tp,  8 * 2(sp)",
        "sw t0,  8 * 3(sp)",
        "sw t1,  8 * 4(sp)",
        "sw t2,  8 * 5(sp)",
        "sw t3,  8 * 6(sp)",
        "sw t4,  8 * 7(sp)",
        "sw t5,  8 * 8(sp)",
        "sw t6,  8 * 9(sp)",
        "sw a0,  8 * 10(sp)",
        "sw a1,  8 * 11(sp)",
        "sw a2,  8 * 12(sp)",
        "sw a3,  8 * 13(sp)",
        "sw a4,  8 * 14(sp)",
        "sw a5,  8 * 15(sp)",
        "sw a6,  8 * 16(sp)",
        "sw a7,  8 * 17(sp)",
        "sw s0,  8 * 18(sp)",
        "sw s1,  8 * 19(sp)",
        "sw s2,  8 * 20(sp)",
        "sw s3,  8 * 21(sp)",
        "sw s4,  8 * 22(sp)",
        "sw s5,  8 * 23(sp)",
        "sw s6,  8 * 24(sp)",
        "sw s7,  8 * 25(sp)",
        "sw s8,  8 * 26(sp)",
        "sw s9,  8 * 27(sp)",
        "sw s10, 8 * 28(sp)",
        "sw s11, 8 * 29(sp)",
        "csrr a0, sscratch",
        "sw a0, 8 * 30(sp)",
        "mv a0, sp",
        "call handle_trap",
        "lw ra,  8 * 0(sp)",
        "lw gp,  8 * 1(sp)",
        "lw tp,  8 * 2(sp)",
        "lw t0,  8 * 3(sp)",
        "lw t1,  8 * 4(sp)",
        "lw t2,  8 * 5(sp)",
        "lw t3,  8 * 6(sp)",
        "lw t4,  8 * 7(sp)",
        "lw t5,  8 * 8(sp)",
        "lw t6,  8 * 9(sp)",
        "lw a0,  8 * 10(sp)",
        "lw a1,  8 * 11(sp)",
        "lw a2,  8 * 12(sp)",
        "lw a3,  8 * 13(sp)",
        "lw a4,  8 * 14(sp)",
        "lw a5,  8 * 15(sp)",
        "lw a6,  8 * 16(sp)",
        "lw a7,  8 * 17(sp)",
        "lw s0,  8 * 18(sp)",
        "lw s1,  8 * 19(sp)",
        "lw s2,  8 * 20(sp)",
        "lw s3,  8 * 21(sp)",
        "lw s4,  8 * 22(sp)",
        "lw s5,  8 * 23(sp)",
        "lw s6,  8 * 24(sp)",
        "lw s7,  8 * 25(sp)",
        "lw s8,  8 * 26(sp)",
        "lw s9,  8 * 27(sp)",
        "lw s10, 8 * 28(sp)",
        "lw s11, 8 * 29(sp)",
        "lw sp,  8 * 30(sp)",
        "sret",
    )
}

#[allow(unused)]
#[unsafe(no_mangle)]
pub fn handle_trap(_trap_frame: *const u8) -> ! {
    let scause = read_csr("scause");
    let stval = read_csr("stval");
    let user_pc = read_csr("sepc");
    panic!(
        "unexpected trap: scause={:x}, stval={:x}, sepc={:x}",
        scause, stval, user_pc
    );
}
