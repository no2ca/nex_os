#![no_std]
#![no_main]
use core::{arch::asm, panic::PanicInfo};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

unsafe extern "C" {
    static __stack_top: u8;
    static mut __bss: u8;
    static __bss_end: u8;
}

//
// システムコール
//

const SYS_WRITE_BYTE: usize = 1;

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
// コンソール出力
//

struct Writer;

impl Writer {
    fn write_byte(c: u8) {
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
    ($($arg:tt)*) => (let _ = crate::_print(format_args!($($arg)*)););
}

#[macro_export]
macro_rules! println {
    () => (let _ =print!("\n"););
    ($($arg:tt)*) => (crate::print!("{}\n", format_args!($($arg)*)););
}

//
// シェルのプログラム
//

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.start")]
extern "C" fn start() {
    // スタックトップの設定
    unsafe {
        asm!(
            "mv sp, {0}",
            in(reg) &__stack_top as *const u8 as usize,
        );
    }

    main();
}

fn main() {
    println!("Hello from shell!!");
    loop {
        core::hint::spin_loop();
    }
}
