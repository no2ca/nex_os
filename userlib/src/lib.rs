#![no_std]
#![no_main]
use core::{arch::asm, panic::PanicInfo};
use syscall::{
    SYS_CREATE_PROCESS, SYS_EXIT_PROCESS, SYS_READ_BYTE, SYS_WRITE_BYTE, SYS_YIELD_PROCESS,
};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    _print(format_args!("\n{info}"));
    // TODO: ユーザーがpanicしたらプロセスはexitするようにしたい
    loop {}
}

//
// システムコール
//

fn syscall(sysno: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    let sysret: isize;
    unsafe {
        asm!(
            "ecall",
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a3") sysno,
            lateout("a0") sysret,
        );
    }
    sysret
}

//
// コンソール入出力
//

pub fn read_byte() -> u8 {
    let ret = syscall(SYS_READ_BYTE, 0, 0, 0);
    u8::try_from(ret).unwrap()
}

pub struct Writer;

impl Writer {
    pub fn write_byte(c: u8) {
        // TODO: システムコールに失敗したときのエラー処理を行っていない
        syscall(SYS_WRITE_BYTE, c as usize, 0, 0);
    }
}

use core::fmt;
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        s.bytes().for_each(|c| Writer::write_byte(c));
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    let mut writer = Writer;
    writer.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => (crate::userlib::_print(format_args!($($arg)*)););
}

#[macro_export]
macro_rules! println {
    () => (let _ =print!("\n"););
    ($($arg:tt)*) => (crate::print!("{}\n", format_args!($($arg)*)););
}

#[macro_export]
macro_rules! user_main {
    ($main_fn: ident) => {
        unsafe extern "C" {
            static __stack_top: u8;
            static mut __bss: u8;
            static __bss_end: u8;
        }

        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        #[unsafe(link_section = ".text.start")]
        extern "C" fn start() {
            unsafe {
                core::arch::naked_asm!(
                    "la sp, {stack_top}",
                    "call {main}",
                    stack_top = sym __stack_top,
                    main = sym $main_fn,
                );
            }
        }
    };
}

//
// プロセス関連
//

pub fn yield_process() {
    syscall(SYS_YIELD_PROCESS, 0, 0, 0);
}

pub fn exit_process() {
    syscall(SYS_EXIT_PROCESS, 0, 0, 0);
}

fn create_process(path: &str) {
    let ptr = path.as_ptr() as usize;
    let len = path.len();
    syscall(SYS_CREATE_PROCESS, ptr, len, 0);
}

pub fn spawn(path: &str) {
    create_process(path);
    yield_process();
}
