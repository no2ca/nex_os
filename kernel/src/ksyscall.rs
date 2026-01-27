use core::slice;
extern crate alloc;

use crate::{
    allocator::{self, PAGE_SIZE},
    console::{self, Writer},
    println, proc,
    vfs::{self, Fs, Node},
};
use syscall::{
    SYS_CREATE_PROCESS, SYS_EXIT_PROCESS, SYS_READ_BYTE, SYS_WRITE_BYTE, SYS_YIELD_PROCESS,
};
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

    let frame = TrapFrame::mut_from_prefix(trap_frame_slice).unwrap();
    let sysno = frame.a3;
    match sysno {
        SYS_WRITE_BYTE => {
            let c = u8::try_from(frame.a0).unwrap();
            Writer::write_byte(c).unwrap();
        }
        SYS_READ_BYTE => loop {
            let byte = console::read_byte();
            if byte >= 0 {
                frame.a0 = byte as usize;
                break;
            }
        },
        SYS_YIELD_PROCESS => {
            proc::yield_process();
        }
        SYS_EXIT_PROCESS => {
            proc::end_process();
        }
        SYS_CREATE_PROCESS => {
            let path_ptr = frame.a0 as *const u8;
            let path_len = frame.a1;
            let mut bytes = alloc::vec::Vec::with_capacity(path_len);

            unsafe {
                crate::csr::set_sum();
                for i in 0..path_len {
                    bytes.push(*path_ptr.add(i));
                }
                crate::csr::clear_sum();
            }

            let path = core::str::from_utf8(&bytes).unwrap();
            println!("[ksyscall] path='{}'", path);
            let fs = vfs::MemoryFs;
            let node = fs.lookup(path).unwrap();
            let n = node.size().div_ceil(PAGE_SIZE);
            let buf_ptr = allocator::PAGE_ALLOC.alloc_pages::<u8>(n).as_mut_ptr();
            let buf = unsafe { slice::from_raw_parts_mut(buf_ptr, n * PAGE_SIZE) };
            node.read(buf).unwrap();
            proc::create_process(buf);
        }
        _ => unimplemented!("{}", sysno),
    }
}
