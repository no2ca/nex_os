#![allow(unused)]
use core::ptr::NonNull;

#[derive(Debug, PartialEq, Clone)]
struct Pid(usize);

impl Pid {
    #[inline]
    fn as_usize(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
struct KernelStack {
    base: NonNull<u8>,
    size: usize,
}

impl KernelStack {
    #[inline]
    fn top(&self) -> *mut usize {
        unsafe { self.base.as_ptr().add(self.size) as *mut usize }
    }
}

#[derive(Debug, PartialEq, Clone)]
#[repr(C)]
struct Context {
    // レジスタを順序通りに並べる
    ra: usize, // return address
    sp: usize, // stack pointer
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
}

#[derive(Debug, PartialEq, Clone)]
struct Process {
    pid: Pid,
    kernel_stack: KernelStack,
    context: Context,
    page_table: *mut [usize],
}
