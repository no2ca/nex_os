// #![allow(unused)]

//
// プロセス管理構造体の定義
//

#[derive(Debug, PartialEq, Clone)]
struct Pid(usize);

impl Pid {
    #[inline]
    fn as_usize(&self) -> usize {
        self.0
    }
}

#[derive(Debug, PartialEq, Clone)]
enum ProcState {
    Unused,
    Runnable,
    Running,
}

#[derive(Debug, Clone, PartialEq)]
struct KernelStack {
    base: *mut u8,
    size: usize,
}

impl KernelStack {
    /// topはスタックポインタで, 64bitレジスタの値を積むのでusizeのポインタとしている
    #[inline]
    fn top(&self) -> *mut usize {
        unsafe { self.base.add(self.size) as *mut usize }
    }

    const fn null() -> Self {
        Self {
            base: core::ptr::null_mut(),
            size: 0,
        }
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

impl Context {
    const fn zero() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
struct Process {
    pid: Pid,
    state: ProcState,
    kernel_stack: KernelStack,
    context: Context,
    page_table: *mut [usize],
}

impl Process {
    const fn unused() -> Self {
        let null_ptr_slice = core::ptr::slice_from_raw_parts_mut(core::ptr::null_mut(), 0);
        Self {
            pid: Pid(usize::MAX),
            state: ProcState::Unused,
            kernel_stack: KernelStack::null(),
            context: Context::zero(),
            page_table: null_ptr_slice,
        }
    }
}

//
// プロセステーブルの定義
//

use core::cell::UnsafeCell;
use core::usize;

use crate::println;

struct ProcessTableCell<T> {
    inner: UnsafeCell<T>,
}

unsafe impl<T> Sync for ProcessTableCell<T> {}

impl<T> ProcessTableCell<T> {
    const fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }

    #[inline]
    unsafe fn get(&self) -> &T {
        unsafe { &*self.inner.get() }
    }

    #[inline]
    unsafe fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }
}

const NPROC: usize = 8;

struct ProcessTable {
    procs: [Process; NPROC],
    current: Option<usize>, // 実行中のプロセスへのインデックス
}

impl ProcessTable {
    const fn new() -> Self {
        Self {
            procs: [const { Process::unused() }; NPROC],
            current: None,
        }
    }
}

pub fn dump_process_list() {
    println!("[procv2] process list:");
    let ptable = unsafe { PTABLE.get() };
    for proc in ptable.procs.iter() {
        println!(
            "\tpid={}, state={:?}, kernel_stack_top={:p}, ",
            proc.pid.as_usize(),
            proc.state,
            proc.kernel_stack.top()
        );
    }
}

static PTABLE: ProcessTableCell<ProcessTable> = ProcessTableCell::new(ProcessTable::new());
