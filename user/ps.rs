#![no_std]
#![no_main]

use userlib::{exit_process, list_process, user_main};

user_main!(main);

fn main() {
    let _ = list_process();
    let _ = exit_process();
}
