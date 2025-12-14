use core::{
    arch::naked_asm,
    cell::UnsafeCell,
    panic::PanicInfo,
    ptr::{self, NonNull},
};

use crate::{alloc::Allocator, println};

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

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[derive(Debug, PartialEq, Clone)]
pub struct Pid(usize);

#[derive(Debug, PartialEq, Clone)]
struct Process {
    pid: Pid,
    stack: Stack,
    saved_sp: StackPointer,
}

impl Process {
    fn init(pid: Pid, pc: usize, allocator: &mut Allocator) -> Self {
        let stack = Stack::new(allocator);
        let sp = stack.sp.as_ptr();
        unsafe {
            ptr::write_volatile(sp.offset(0), pc);
        }
        println!(
            "[DEBUG] [Process::init] process {} initialized with pc={:p}, sp={:p}",
            pid.0, pc as *const u8, sp
        );
        Process {
            pid,
            stack,
            saved_sp: StackPointer(sp),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Stack {
    sp: NonNull<usize>,
}

const STACK_SIZE: usize = 4096;
const REGS_SAVE_COUNT: usize = 13;

impl Stack {
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
        Stack {
            sp: NonNull::new(sp).expect("stack pointer is null"),
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, PartialEq)]
pub struct StackPointer(*mut usize);

impl StackPointer {
    pub const fn null() -> Self {
        Self(ptr::null_mut())
    }
}

#[repr(transparent)]
#[derive(Clone, Debug)]
struct StackPointerSlot(*mut StackPointer);

impl StackPointerSlot {
    const fn new(slot: *mut StackPointer) -> Self {
        Self(slot)
    }

    fn as_const_ptr(self) -> *const StackPointer {
        self.0 as *const StackPointer
    }
}

const PROCS_MAX: usize = 8;
static current_proc: SyncUnsafeCell<Option<NonNull<Process>>> = SyncUnsafeCell::new(None);
static PROCS: SyncUnsafeCell<Procs> = SyncUnsafeCell::new(Procs {
    procs: [const { None }; PROCS_MAX],
});

#[derive(Debug)]
struct Procs {
    procs: [Option<Process>; PROCS_MAX],
}

pub fn create_process(allocator: &mut Allocator, pc: usize) -> Pid {
    unsafe {
        PROCS
            .get()
            .as_mut()
            .unwrap()
            .procs
            .iter_mut()
            .enumerate()
            .find(|(_, p)| p.is_none())
            .map(|(i, p)| p.insert(Process::init(Pid(i), pc as usize, allocator)))
            .expect("process creation failed")
            .pid
            .clone()
    }
}

pub fn yield_process() {
    let prev_ptr = unsafe { *current_proc.get() };
    let next_ptr = schedule(prev_ptr);
    let next_proc = unsafe { &mut *next_ptr.as_ptr() };
    println!("\n[DEBUG] [sched] next process: \n{:?}", next_proc);
    unsafe {
        *current_proc.get() = Some(next_ptr);
    }
    if prev_ptr.map_or(false, |ptr| ptr == next_ptr) {
        return;
    }

    let next_slot = StackPointerSlot::new(&mut next_proc.saved_sp).as_const_ptr();
    let prev_slot = prev_ptr.map(|ptr| {
        let proc = unsafe { &mut *ptr.as_ptr() };
        StackPointerSlot::new(&mut proc.saved_sp)
    });
    unsafe {
        match prev_slot {
            Some(slot) => switch_context(slot, next_slot),
            None => switch_context(StackPointerSlot::new(&raw mut crate::idle_proc), next_slot),
        }
    }
}

fn schedule(prev: Option<NonNull<Process>>) -> NonNull<Process> {
    let procs = unsafe { &mut *PROCS.get() };
    let start_idx = prev
        .and_then(|ptr| {
            let raw = ptr.as_ptr();
            procs
                .procs
                .iter()
                .position(|p| p.as_ref().map_or(false, |proc| proc as *const _ == raw))
        })
        .map(|idx| (idx + 1) % PROCS_MAX)
        .unwrap_or(0);
    for offset in 0..PROCS_MAX {
        let idx = (start_idx + offset) % PROCS_MAX;
        if let Some(proc) = procs.procs[idx].as_mut() {
            if proc.pid != Pid(0) {
                return NonNull::from(proc);
            }
        }
    }
    panic!("no process to schedule");
}

pub fn dump_process_list() {
    println!("[DEBUG] [proc] process list:");
    for p in &mut unsafe { PROCS.get().as_mut().unwrap().procs.clone() } {
        if let Some(_p) = p {
            println!(
                "\tpid={}, sp={:p}, pc={:p}",
                _p.pid.0,
                _p.stack.sp,
                unsafe { _p.stack.sp.offset(0).read_volatile() } as *const u8
            );
        } else {
            println!("\tNone");
        }
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn switch_context(prev_sp: StackPointerSlot, next_sp: *const StackPointer) {
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
