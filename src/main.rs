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
use core::{arch::{asm, naked_asm}, panic::PanicInfo};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[derive(Debug)]
struct Pid(usize);

#[derive(Debug)]
enum ProcessState {
    Unused,
    Runnable
}

#[derive(Debug)]
struct Process {
    pid: Pid,
    state: ProcessState,
    stack: Stack,
}

impl Process {
    #[unsafe(no_mangle)]
    fn new(pid: Pid, state: ProcessState, pc: usize, allocator: &mut Allocator) -> Self {
        let stack = Stack::new(allocator);
        unsafe {
            let sp = stack.sp as *mut usize;
            let mut frame: [usize; 13] = [0; 13];
            frame[0] = pc;
            core::ptr::copy_nonoverlapping(
                frame.as_ptr(),
                sp,
                frame.len(),
            );
            println!("[proc] pid: {}  sp: {:p}  pc: {:p}", pid.0, sp as *const u8, pc as *const u8);
        }
        Process {
            pid,
            state,
            stack,
        }
    }
}

const STACK_SIZE: usize = PAGE_SIZE;

#[derive(Debug)]
struct Stack {
    base: *mut u8,
    sp: *mut u8,
}

impl Stack {
    fn new(allocator: &mut Allocator) -> Self {
        let pages = STACK_SIZE / PAGE_SIZE;
        let base = allocator
            .alloc_pages(pages)
            .expect("stack allocation failed") as *mut u8;
        let mut stack = Stack {
            base,
            sp: core::ptr::null_mut(),
        };
        stack.set_sp();
        stack
    }
    
    fn set_sp(&mut self) {
        unsafe { self.sp = self.base.add(STACK_SIZE - 13 * core::mem::size_of::<usize>()); }
    }
}

const PROCS_MAX: usize = 8;

#[derive(Debug)]
struct Procs {
    procs: [Option<Process>; PROCS_MAX]
}

impl Procs {
    fn init() -> Self {
        Self {
            procs: [const { None }; PROCS_MAX]
        }
    }
}

fn create_process<'a>(procs: &'a mut Procs, allocator: &mut Allocator, pc: usize) -> &'a mut Process {
    procs.procs.iter_mut().enumerate()
        .find(|(_, p)| p.is_none())
        .map(|(i, p)| p.insert(Process::new(Pid(i), ProcessState::Runnable, pc, allocator)))
        .expect("process creation failed")
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn switch_context(prev_sp: *mut usize, next_sp: *const usize) {
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
            switch_context(&raw mut sp_a, &raw const sp_b);
        }
        for _ in 0..5_000_000 {
            core::hint::spin_loop();
        }
    }
}

fn proc_b() {
    println!("\nproc_b started");
    loop {
        print!("B");
        unsafe {
            switch_context(&raw mut sp_b, &raw const sp_a);
        }
        for _ in 0..5_000_000 {
            core::hint::spin_loop();
        }
    }
}

static mut sp_a : usize = 0;
static mut sp_b : usize = 0;
static mut init_sp: usize = 0;

fn main() {
    println!("[mem] kernel_entry\t\t: {:p}", kernel_entry as *const u8);

    write_csr("stvec", kernel_entry as usize);
    println!("[reg] stvec register\t\t: {:x}", read_csr("stvec"));

    unsafe {
        println!("[mem] free ram start\t\t: {:p}", &__free_ram);
    }

    let mut allocator = Allocator::new();
    let paddr0 = allocator.alloc_pages(2).unwrap();
    let paddr1 = allocator.alloc_pages(1).unwrap();
    println!("[alloc] alloc_pages(2)\t\t: {:p}", paddr0);
    println!("[alloc] alloc_pages(1)\t\t: {:p}", paddr1);
    
    let mut procs = Procs::init();
    unsafe {
        sp_a = create_process(&mut procs, &mut allocator, proc_a as usize).stack.sp as usize;
        sp_b = create_process(&mut procs, &mut allocator, proc_b as usize).stack.sp as usize;
    }


    println!("[proc] process list:");
    for p in &mut procs.procs {
        if let Some(_p) = p {
            println!("\tpid={}, sp={:p}, pc={:p}", _p.pid.0, _p.stack.sp, unsafe { (_p.stack.sp as *const usize).offset(0).read_volatile() } as *const u8);
        } else {
            println!("\tNone");
        }
    }
    
    unsafe {
        println!("[test] switching to proc_a");
        switch_context(&raw mut init_sp, &raw mut sp_a);
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
