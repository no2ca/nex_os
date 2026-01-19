use crate::console::{self, Writer};
use syscall::{SYS_READ_BYTE, SYS_WRITE_BYTE};
use zerocopy::{AsBytes, FromBytes, FromZeroes};

#[allow(unused)]
#[derive(Debug, Clone, FromZeroes, FromBytes, AsBytes)]
#[repr(C)]
struct TrapFrame {
    ra: usize,
    gp: usize,
    tp: usize,
    t0: usize,
    t1: usize,
    t2: usize,
    t3: usize,
    t4: usize,
    t5: usize,
    t6: usize,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,
    s0: usize,
    s1: usize,
    s2: usize,
    s3: usize,
    s4: usize,
    s5: usize,
    s6: usize,
    s7: usize,
    s8: usize,
    s9: usize,
    s10: usize,
    s11: usize,
    sp: usize,
}

pub fn handle_syscall(trap_frame: *mut u8) {
    let trap_frame_slice =
        unsafe { core::slice::from_raw_parts_mut(trap_frame, size_of::<TrapFrame>()) };

    let trap_frame = TrapFrame::mut_from_prefix(trap_frame_slice).unwrap();
    let sysno = trap_frame.a3;
    match sysno {
        SYS_WRITE_BYTE => {
            let c = u8::try_from(trap_frame.a0).unwrap();
            Writer::write_byte(c).unwrap();
        }
        SYS_READ_BYTE => loop {
            let byte = console::read_byte();
            if byte >= 0 {
                trap_frame.a0 = byte as usize;
                break;
            }
        },
        _ => unimplemented!("{}", sysno),
    }
}
