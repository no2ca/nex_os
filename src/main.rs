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
use core::{arch::asm, panic::PanicInfo};

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
    regs: Registers,
}

impl Process {
    #[unsafe(no_mangle)]
    fn new(pid: Pid, state: ProcessState, pc: usize, allocator: &mut Allocator) -> Self {
        let stack = Stack::new(allocator);
        let regs = Registers::new(stack.sp as usize, pc);
        Process {
            pid,
            state,
            stack,
            regs,
        }
    }
}

const STACK_SIZE: usize = 4096 * 2;

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
        stack.set_ptr();
        stack
    }
    
    fn set_ptr(&mut self) {
        unsafe { self.sp = self.base.add(STACK_SIZE); }
    }
}

#[derive(Debug)]
struct Registers {
    sp: usize,
    pc: usize,
    s: [usize; 12], // s0-s11
}

impl Registers {
    fn new(sp: usize, pc: usize) -> Self {
        Self {
            sp,
            pc,
            s: [0; 12],
        }
    }
}

const PROCS_MAX: usize = 8;

#[derive(Debug)]
struct Procs {
    procs: [Option<Process>; PROCS_MAX]
}

impl Procs {
    #[unsafe(link_section = ".bss")]
    fn init() -> Self {
        Self {
            procs: [const { None }; PROCS_MAX]
        }
    }
}

fn create_process(procs: &mut Procs, allocator: &mut Allocator, pc: usize) {
    procs.procs.iter_mut().enumerate()
        .find(|(_, p)| p.is_none())
        .map(|(i, p)| p.insert(Process::new(Pid(i), ProcessState::Runnable, pc, allocator)));
}

fn dummy() {
    loop {
        
    }
}

fn main() {
    println!("kernel_entry\t\t: {:p}", kernel_entry as *const u8);

    write_csr("stvec", kernel_entry as usize);
    println!("stvec register\t\t: {:x}", read_csr("stvec"));

    unsafe {
        println!("free ram start\t\t: {:p}", &__free_ram);
    }

    let mut allocator = Allocator::new();
    let paddr0 = allocator.alloc_pages(15).unwrap();
    let paddr1 = allocator.alloc_pages(15).unwrap();
    println!("alloc_pages() test\t: {:p}", paddr0);
    println!("alloc_pages(1) test\t: {:p}", paddr1);
    if unsafe { (&__free_ram as *const u8).add(PAGE_SIZE * 1024) } == paddr1 {
        println!("Page allocation OK");
    }
    
    let mut procs = Procs::init();
    for _ in 0..PROCS_MAX {
        create_process(&mut procs, &mut allocator, dummy as usize);
    }
    for p in &mut procs.procs {
        if let Some(_p) = p {
            _p.stack.set_ptr();
            println!("pid: {}, ptr: {:p}, pc: {:p}", _p.pid.0, _p.stack.sp, _p.regs.pc as *const u8);
            println!("p: {:p}", _p as *const _);
        } else {
            println!("None");
        }
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
