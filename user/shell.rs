#![no_std]
#![no_main]

use core::str::from_utf8;

use userlib::{self, Writer, print, println, read_byte, user_main};

const BUF_SIZE: usize = 1024;

user_main!(main);

fn main() {
    println!("Hello from shell!!");
    loop {
        print!("> ");

        // 入力を受け取る
        let mut buf = [0u8; BUF_SIZE];
        let mut index = 0;
        loop {
            let c = read_byte();

            // 改行キーが押されたとき
            if c == b'\r' {
                print!("\n");
                break;
            }

            // Backspaceが押されたとき
            if c == 127 || c == 8 {
                // 1つ前の文字を消す
                if !buf.get(index - 1).is_none() {
                    buf[index - 1] = 0;
                    index -= 1;
                    // カーソルを左に1つ動かす
                    Writer::write_byte(8);
                    // 消したい文字をスペースで置き換える
                    Writer::write_byte(0x20);
                    // カーソルを左に動かして入力できるようにする
                    Writer::write_byte(8);
                }
                continue;
            }

            // バッファに収まる場合のみ書き込む
            if !buf.get(index).is_none() {
                buf[index] = c;
            }

            Writer::write_byte(c);
            index += 1;
        }

        // 入力の解釈

        // 改行のみのとき
        if index == 0 {
            continue;
        }

        // コマンドが長すぎるとき
        let input_length = index;
        if input_length > BUF_SIZE {
            println!("input too long!");
            println!(
                "buffer size is {} bytes, got {} bytes",
                BUF_SIZE, input_length
            );
            continue;
        }

        // バッファを文字列に変換
        let command_str = match from_utf8(&buf[0..input_length]) {
            Ok(s) => s,
            Err(e) => {
                println!("Parse Error: {}", e);
                continue;
            }
        };

        let help_msg = r#"
Available commands:
        hello: Just says "hello"

"#;

        match command_str {
            "hello" => println!("hello"),
            "help" => println!("{}", help_msg),
            _ => {
                println!("{}: comannd not found", command_str);
                println!("DEBUG: {:?}", command_str.as_bytes());
            }
        }
    }
}
