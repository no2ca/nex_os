use crate::println;

pub fn handle_syscall(_trap_frame: *const u8) {
    println!("[handle_syscall] called!");
}
