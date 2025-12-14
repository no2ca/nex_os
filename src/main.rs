#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]
#![feature(naked_functions_rustic_abi)]
#![feature(unsafe_cell_access)]

mod alloc;
mod boot;
mod console;
mod csr;
mod trap;
mod utils;

use crate::{
    alloc::{__free_ram, Allocator, PAGE_SIZE},
    csr::{read_csr, write_csr},
    trap::kernel_entry,
};
use core::{
    arch::{asm, naked_asm},
    cell::UnsafeCell,
    panic::PanicInfo,
    ptr::{self, NonNull},
    sync::{self, atomic::AtomicPtr},
};

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
struct Pid(usize);

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
        let pages = STACK_SIZE.div_ceil(PAGE_SIZE);
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
struct StackPointer(*mut usize);

impl StackPointer {
    const fn null() -> Self {
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

impl Procs {
    fn init() -> Self {
        Self {
            procs: [const { None }; PROCS_MAX],
        }
    }
}

fn create_process<'a>(allocator: &mut Allocator, pc: usize) -> &'a mut Process {
    unsafe {
        PROCS
            .get()
            .as_mut()
            .unwrap()
            .procs
            .iter_mut()
            .enumerate()
            .find(|(_, p)| p.is_none())
            .map(|(i, p)| p.insert(Process::init(Pid(i + 1), pc, allocator)))
            .expect("process creation failed")
    }
}

fn yield_process() {
    let prev_ptr = unsafe { *current_proc.get() };
    let next_ptr = schedule(prev_ptr);
    let next_proc = unsafe { &mut *next_ptr.as_ptr() };
    println!("[DEBUG] [sched] next process: {:#?}", next_proc);
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
            None => switch_context(StackPointerSlot::new(&raw mut init_sp), next_slot),
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

fn proc_a() {
    println!("proc_a started");
    loop {
        print!("A");
        yield_process();
        for _ in 0..50_000_000 {
            core::hint::spin_loop();
        }
    }
}

#[unsafe(no_mangle)]
fn proc_b() {
    println!("\nproc_b started");
    loop {
        print!("B");
        yield_process();
        for _ in 0..5_000_000 {
            core::hint::spin_loop();
        }
    }
}

// プロセスのスタックポインタを保存する領域
static sp_a: sync::atomic::AtomicPtr<usize> = AtomicPtr::new(ptr::null_mut());
static sp_b: sync::atomic::AtomicPtr<usize> = AtomicPtr::new(ptr::null_mut());
static mut init_sp: StackPointer = StackPointer::null();

fn main() {
    println!(
        "[INFO ] [mem] kernel_entry\t\t: {:p}",
        kernel_entry as *const u8
    );

    write_csr("stvec", kernel_entry as usize);
    println!("[INFO ] [reg] stvec register\t\t: {:#x}", read_csr("stvec"));

    unsafe {
        println!("[INFO ] [mem] free ram start\t\t: {:p}", &__free_ram);
    }

    // アロケータの初期化とメモリ確保のテスト
    let mut allocator = Allocator::new();
    let paddr0 = allocator.alloc_pages(2).unwrap();
    let paddr1 = allocator.alloc_pages(1).unwrap();
    println!("[TEST ] [alloc] alloc_pages(2)\t\t: {:p}", paddr0);
    println!("[TEST ] [alloc] alloc_pages(1)\t\t: {:p}", paddr1);

    // 書き込みできる範囲のテスト
    let ptr_low = 0x80050000 as *mut u8;
    unsafe {
        let val = ptr_low.read_volatile();
        println!("read from {:p} pointer: {}", ptr_low, val);
    }

    let ptr_high = 0x87ffffff as *mut u8;
    unsafe {
        let val = ptr_high.read_volatile();
        println!("read from {:p} pointer: {}", ptr_high, val);
    }

    // プロセスの作成とコンテキストスイッチのテスト
    let idle = create_process(&mut allocator, &raw const init_sp as *const StackPointer as usize);
    idle.pid = Pid(0);
    unsafe {
        *current_proc.get() = Some(NonNull::from(idle));
    }
    let ps1 = create_process(&mut allocator, proc_a as usize);
    let ps2 = create_process(&mut allocator, proc_b as usize);

    // println!("[DEBUG] [proc] created processes:");
    // println!(
    //     "\tproc_a: pid={}, pc={:p}, sp={:p}",
    //     ps1.pid.0, proc_a as *const u8, ps1.stack.sp
    // );
    // println!(
    //     "\tproc_b: pid={}, pc={:p}, sp={:p}",
    //     ps2.pid.0, proc_b as *const u8, ps2.stack.sp
    // );

    sp_a.store(ps1.stack.sp.as_ptr(), sync::atomic::Ordering::Relaxed);
    sp_b.store(ps2.stack.sp.as_ptr(), sync::atomic::Ordering::Relaxed);

    yield_process();

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

    unsafe {
        println!(
            "[DEBUG] [reg] StackPointerSlot(sp_a)={:p}, StackPointer={:p}",
            &sp_a, sp_a
        );
        println!(
            "[DEBUG] [reg] StackPointerSlot(sp_b)={:p}, StackPointer={:p}",
            &sp_b, sp_b
        );
        println!("[TEST ] [swtch] switching to proc_a");
        // X.as_ptr で X のポインタが返るため参照にする必要はない
        switch_context(
            StackPointerSlot::new(&raw mut init_sp),
            StackPointerSlot::new(sp_a.as_ptr() as *mut StackPointer).as_const_ptr(),
        );
    }

    unsafe {
        let sp: usize;
        asm!("mv {0}, sp", out(reg) sp);
        println!("sp: {:p}", sp as *const u8);
    }

    unsafe {
        let ptr = 0xdeadbeef as *mut u8;
        ptr.write_volatile(0x42);

        loop {
            core::hint::spin_loop();
        }
    }
}
