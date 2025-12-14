#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]
#![feature(naked_functions_rustic_abi)]

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
    panic::PanicInfo,
    ptr::{self, NonNull},
    sync::{
        self,
        atomic::AtomicPtr,
    },
};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[derive(Debug)]
struct Pid(usize);

#[derive(Debug)]
struct Process {
    pid: Pid,
    stack: Stack,
}

impl Process {
    fn init(pid: Pid, pc: usize, allocator: &mut Allocator) -> Self {
        let stack = Stack::new(allocator);
        let sp = stack.sp.as_ptr();
        unsafe {
            ptr::write_volatile(sp.offset(0), pc);
        }
        println!(
            "[DEBUG] [proc] process {} initialized with pc={:p}, sp={:p}",
            pid.0, pc as *const u8, sp
        );
        Process { pid, stack }
    }
}

#[derive(Debug)]
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
            (base as *mut u8)
                .add(STACK_SIZE - REGS_SAVE_COUNT * core::mem::size_of::<usize>())
                as *mut usize
        };
        Stack {
            sp: NonNull::new(sp).expect("stack pointer is null"),
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
struct StackPointer(*mut usize);

impl StackPointer {
    const fn null() -> Self {
        Self(ptr::null_mut())
    }

    fn as_ptr(self) -> *mut usize {
        self.0
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
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

fn create_process<'a>(
    procs: &'a mut Procs,
    allocator: &mut Allocator,
    pc: usize,
) -> &'a mut Process {
    procs
        .procs
        .iter_mut()
        .enumerate()
        .find(|(_, p)| p.is_none())
        .map(|(i, p)| p.insert(Process::init(Pid(i), pc, allocator)))
        .expect("process creation failed")
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
        unsafe {
            switch_context(
                StackPointerSlot::new(sp_a.as_ptr() as *mut StackPointer),
                StackPointerSlot::new(sp_b.as_ptr() as *mut StackPointer).as_const_ptr(),
            );
        }
        for _ in 0..5_000_000 {
            core::hint::spin_loop();
        }
    }
}

#[unsafe(no_mangle)]
fn proc_b() {
    println!("\nproc_b started");
    loop {
        print!("B");
        unsafe {
            switch_context(
                StackPointerSlot::new(sp_b.as_ptr() as *mut StackPointer),
                StackPointerSlot::new(sp_a.as_ptr() as *mut StackPointer).as_const_ptr(),
            );
        }
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
    println!("[INFO ] [mem] kernel_entry\t\t: {:p}", kernel_entry as *const u8);

    write_csr("stvec", kernel_entry as usize);
    println!("[INFO ] [reg] stvec register\t\t: {:#x}", read_csr("stvec"));

    unsafe {
        println!("[INFO ] [mem] free ram start\t\t: {:p}", &__free_ram);
    }

    let mut allocator = Allocator::new();
    let paddr0 = allocator.alloc_pages(2).unwrap();
    let paddr1 = allocator.alloc_pages(1).unwrap();
    println!("[TEST ] [alloc] alloc_pages(2)\t\t: {:p}", paddr0);
    println!("[TEST ] [alloc] alloc_pages(1)\t\t: {:p}", paddr1);

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

    let mut procs = Procs::init();
    sp_a.store(
        create_process(&mut procs, &mut allocator, proc_a as usize)
            .stack
            .sp
            .as_ptr(),
        sync::atomic::Ordering::Relaxed,
    );
    sp_b.store(
        create_process(&mut procs, &mut allocator, proc_b as usize)
            .stack
            .sp
            .as_ptr(),
        sync::atomic::Ordering::Relaxed,
    );

    println!("[DEBUG] [proc] process list:");
    for p in &mut procs.procs {
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
