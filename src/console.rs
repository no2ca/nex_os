#![allow(dead_code)]

use core::arch::asm;

struct Writer;

impl Writer {
    fn write_byte(c: u8) -> Result<(), i32> {
        let ret = sbi_call(c as i32, 0, 0, 0, 0, 0, 0, 1);
        if ret.err == 0 { Ok(()) } else { Err(ret.err) }
    }
}

use core::fmt;
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        s.bytes()
            .try_for_each(|c| Writer::write_byte(c).map_err(|_| fmt::Error))?;
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) -> Result<(), fmt::Error> {
    use core::fmt::Write;
    let mut writer = Writer;
    writer.write_fmt(args)?;
    Ok(())
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => (crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => (let _ =print!("\n"););
    ($($arg:tt)*) => (let _ = crate::print!("{}\n", format_args!($($arg)*)););
}

#[inline(always)]
pub unsafe fn memset(mut dst: *mut u8, val: u8, count: usize) {
    for _ in 0..count {
        unsafe {
            dst.write_volatile(val);
            dst = dst.add(1);
        }
    }
}

fn sbi_call(
    arg0: i32,
    arg1: i32,
    arg2: i32,
    arg3: i32,
    arg4: i32,
    arg5: i32,
    fid: i32,
    eid: i32,
) -> Sbiret {
    let err: i32;
    let value: i32;
    unsafe {
        asm!(
            "ecall",
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a3") arg3,
            in("a4") arg4,
            in("a5") arg5,
            in("a6") fid,
            in("a7") eid,
            lateout("a0") err,      // lateoutは全ての入力が消費された後に出力が利用される
            lateout("a1") value,
        );
    }
    Sbiret { err, value }
}

#[repr(C)]
struct Sbiret {
    err: i32,
    value: i32,
}
