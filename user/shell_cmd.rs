use core::str::from_utf8;

use userlib::{self, exit_process, print, println, yield_process};

use crate::{ARGS_SIZE, BUF_SIZE, HISTORY_SIZE};

pub fn builtin_hello() {
    println!("hello");
}

pub fn builtin_ohgiri() {
    println!("大喜利増やしてくれる人募集中");
}

pub fn builtin_help() {
    let help_msg = "
Available commands:
    help:\tHelp comammd (this command)
    hello:\tJust says 'hello'
    echo:\tBuiltin echo command
    history:\tShow history
    yield:\tYields current process
";
    println!("{}", help_msg);
}

pub fn builtin_echo(args: [&str; ARGS_SIZE]) {
    for (i, arg) in args[1..ARGS_SIZE].iter().enumerate() {
        if arg.is_empty() {
            break;
        }
        // 区切りはすべてスペースにする
        if i != 0 {
            print!(" ");
        }
        print!("{}", args[i + 1]);
    }
    print!("\n");
}

pub fn builtin_history(history: &[[u8; BUF_SIZE]], history_len: &[usize; HISTORY_SIZE]) {
    for (item, len) in history.iter().zip(history_len.iter()).rev() {
        if *len == 0 {
            continue;
        }
        if let Ok(s) = from_utf8(&item[..*len]) {
            println!("{}", s);
        }
    }
}

pub fn builtin_yield() {
    yield_process();
}

pub fn builtin_exit() {
    exit_process();
}
