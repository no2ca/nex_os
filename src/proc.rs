use core::{
    arch::naked_asm,
    cell::UnsafeCell,
    ptr::{self, NonNull},
};

use crate::{alloc::Allocator, csr, println};

struct SyncUnsafeCell<T>(UnsafeCell<T>);
unsafe impl<T> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    const fn new(val: T) -> Self {
        Self(UnsafeCell::new(val))
    }

    fn get(&self) -> *mut T {
        self.0.get()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Pid(usize);

#[derive(Debug, PartialEq, Clone)]
struct Process {
    pid: Pid,
    kernel_stack: KernelStack,
    saved_sp: SavedSp,
}

impl Process {
    fn init(pid: Pid, pc: usize, allocator: &mut Allocator) -> Self {
        let kernel_stack = KernelStack::new(allocator);
        let kernel_stack_top = kernel_stack.top.as_ptr();
        unsafe {
            ptr::write_volatile(kernel_stack_top.offset(0), pc);
        }

        println!(
            "[DEBUG] [Process::init] process {} initialized with pc={:p}, sp={:p}",
            pid.0, pc as *const u8, kernel_stack_top
        );
        
        // 次のプロセスが最初に読み取る領域を設定する
        let saved_sp = SavedSp::new(kernel_stack_top);

        Process {
            pid,
            kernel_stack,
            saved_sp,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct KernelStack {
    top: NonNull<usize>,
}

const STACK_SIZE: usize = 4096;
const REGS_SAVE_COUNT: usize = 13;

impl KernelStack {
    fn new(allocator: &mut Allocator) -> Self {
        // 必要なページ数を計算するときは切り上げが必要
        let pages = STACK_SIZE.div_ceil(crate::alloc::PAGE_SIZE);
        let base = allocator
            .alloc_pages(pages)
            .expect("stack allocation failed") as *mut usize;

        // *mut T の add は T の個数で計算されるためキャストしている
        let sp = unsafe {
            (base as *mut u8).add(STACK_SIZE - REGS_SAVE_COUNT * core::mem::size_of::<usize>())
                as *mut usize
        };
        KernelStack {
            top: NonNull::new(sp).expect("stack pointer is null"),
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SavedSp(*mut usize);

impl SavedSp {
    pub const fn new(ptr: *mut usize) -> Self {
        Self(ptr)
    }

    pub const fn null() -> Self {
        Self(ptr::null_mut())
    }
}

const PROCS_MAX: usize = 8;

struct CurrentProc(SyncUnsafeCell<Option<NonNull<Process>>>);

impl CurrentProc {
    const fn new() -> Self {
        Self(SyncUnsafeCell::new(None))
    }

    fn load(&self) -> Option<NonNull<Process>> {
        unsafe { *self.0.get() }
    }

    fn store(&self, ptr: Option<NonNull<Process>>) {
        unsafe { *self.0.get() = ptr }
    }
}

static CURRENT: CurrentProc = CurrentProc::new();

#[derive(Debug)]
struct ProcessTable {
    procs: [Option<Process>; PROCS_MAX],
}

impl ProcessTable {
    const fn new() -> Self {
        Self {
            procs: [const { None }; PROCS_MAX],
        }
    }

    fn insert(&mut self, pc: usize, allocator: &mut Allocator) -> Option<Pid> {
        self.procs
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.is_none())
            .map(|(i, slot)| {
                let pid = Pid(i);
                *slot = Some(Process::init(pid.clone(), pc, allocator));
                pid
            })
    }

    fn find_idx_by_ptr(&self, raw: *const Process) -> Option<usize> {
        self.procs
            .iter()
            .position(|p| p.as_ref().map_or(false, |proc| proc as *const _ == raw))
    }

    fn next(&mut self, prev: Option<NonNull<Process>>) -> NonNull<Process> {
        let start_idx = prev
            .and_then(|ptr| self.find_idx_by_ptr(ptr.as_ptr()))
            .map(|idx| (idx + 1) % PROCS_MAX)
            .unwrap_or(0);

        for offset in 0..PROCS_MAX {
            let idx = (start_idx + offset) % PROCS_MAX;
            if let Some(proc) = self.procs[idx].as_mut() {
                if proc.pid != Pid(0) {
                    return NonNull::from(proc);
                }
            }
        }

        panic!("no process to schedule");
    }

    fn iter(&self) -> impl Iterator<Item = &Option<Process>> {
        self.procs.iter()
    }
}

static PROCS: SyncUnsafeCell<ProcessTable> = SyncUnsafeCell::new(ProcessTable::new());

fn with_procs_mut<R>(f: impl FnOnce(&mut ProcessTable) -> R) -> R {
    unsafe { f(&mut *PROCS.get()) }
}

fn with_procs<R>(f: impl FnOnce(&ProcessTable) -> R) -> R {
    unsafe { f(&*PROCS.get()) }
}

pub fn create_process(allocator: &mut Allocator, pc: usize) -> Pid {
    with_procs_mut(|table| {
        table
            .insert(pc as usize, allocator)
            .expect("process creation failed")
    })
}

pub fn yield_process() {
    let prev_ptr = CURRENT.load();
    let next_ptr = with_procs_mut(|table| table.next(prev_ptr));
    let next_proc = unsafe { &mut *next_ptr.as_ptr() };

    println!("\n[DEBUG] [sched] next process: \n{:?}", next_proc);
    CURRENT.store(Some(next_ptr));

    if prev_ptr.map_or(false, |ptr| ptr == next_ptr) {
        return;
    }

    let next_slot = unsafe { ptr::addr_of!((*next_ptr.as_ptr()).saved_sp) };
    let prev_slot = prev_ptr.map(|ptr| unsafe { ptr::addr_of_mut!((*ptr.as_ptr()).saved_sp) });

    println!(
        "[DEBUG] [proc] prev_slot={:p}, next_slot={:p}",
        prev_slot.unwrap_or(ptr::null_mut::<SavedSp>()),
        next_slot
    );
    
    unsafe {
        match prev_slot {
            Some(slot) => switch_context(slot, next_slot),
            None => switch_context(ptr::addr_of_mut!(crate::idle_proc), next_slot),
        }
    }
}

pub fn dump_process_list() {
    println!("[DEBUG] [proc] process list:");
    with_procs(|table| {
        for p in table.iter() {
            if let Some(proc) = p {
                println!(
                    "\tpid={}, sp={:p}, pc={:p}",
                    proc.pid.0,
                    proc.kernel_stack.top.as_ptr(),
                    unsafe { proc.kernel_stack.top.as_ptr().offset(0).read_volatile() } as *const u8
                );
            } else {
                println!("\tNone");
            }
        }
    });
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn switch_context(
    prev_sp: *mut SavedSp,
    next_sp: *const SavedSp,
) {
    naked_asm!(
        "addi sp, sp, -13 * 8",
        "sd ra,  0  * 8(sp)",
        "sd s0,  1  * 8(sp)",
        "sd s1,  2  * 8(sp)",
        "sd s2,  3  * 8(sp)",
        "sd s3,  4  * 8(sp)",
        "sd s4,  5  * 8(sp)",
        "sd s5,  6  * 8(sp)",
        "sd s6,  7  * 8(sp)",
        "sd s7,  8  * 8(sp)",
        "sd s8,  9  * 8(sp)",
        "sd s9,  10 * 8(sp)",
        "sd s10, 11 * 8(sp)",
        "sd s11, 12 * 8(sp)",
        "sd sp, (a0)",
        "ld sp, (a1)",
        "ld ra,  0  * 8(sp)",
        "ld s0,  1  * 8(sp)",
        "ld s1,  2  * 8(sp)",
        "ld s2,  3  * 8(sp)",
        "ld s3,  4  * 8(sp)",
        "ld s4,  5  * 8(sp)",
        "ld s5,  6  * 8(sp)",
        "ld s6,  7  * 8(sp)",
        "ld s7,  8  * 8(sp)",
        "ld s8,  9  * 8(sp)",
        "ld s9,  10 * 8(sp)",
        "ld s10, 11 * 8(sp)",
        "ld s11, 12 * 8(sp)",
        "addi sp, sp, 13 * 8",
        "ret",
    );
}
