#![no_std]
#![no_main]

use userlib::{self, Writer, print, println, read_byte, user_main};

user_main!(main);

fn main() {
    println!("Hello from shell!!");
    loop {
        print!("> ");
        let c = read_byte();
        Writer::write_byte(c);
        println!("\n{}", c);
    }
}
