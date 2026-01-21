#![no_std]
#![no_main]

use core::{
    fmt::{self},
    str::{Utf8Error, from_utf8},
};

use userlib::{self, Writer, print, println, read_byte, user_main};

const HISTORY_SIZE: usize = 128;
const BUF_SIZE: usize = 128;
const MAX_ARGS: usize = 128;

//
// 文字列を受け取る
//

#[derive(Debug)]
enum ReadLineError {
    Overflow,
}

fn read_line(buf: &mut [u8]) -> Result<usize, ReadLineError> {
    // 入力を受け取る
    let mut index = 0;
    loop {
        let c = read_byte();

        match c {
            // 改行キーが押されたとき
            b'\r' => {
                print!("\n");
                break;
            }

            // Backspaceが押されたとき
            0x7f | 0x8 => {
                // 1つ前の文字を消す
                if index > 0 {
                    buf[index - 1] = 0;
                    index -= 1;
                    // カーソルを左に1つ動かす
                    Writer::write_byte(0x8);
                    // 消したい文字をスペースで置き換える
                    Writer::write_byte(0x20);
                    // カーソルを左に動かして入力できるようにする
                    Writer::write_byte(0x8);
                }
                continue;
            }

            // エスケープ文字のとき
            0x1b => {
                let _c2 = read_byte();
                let _c3 = read_byte();
                continue;
            }

            _ => {}
        }

        // バッファに収まる場合のみ書き込む
        if index < BUF_SIZE {
            buf[index] = c;
        } else {
            return Err(ReadLineError::Overflow);
        }

        Writer::write_byte(c);
        index += 1;
    }

    Ok(index)
}

//
// パースを行う
//

#[derive(Debug)]
enum ParseError {
    Utf8Error(Utf8Error),
    NonAsciiChar,
}

impl From<Utf8Error> for ParseError {
    fn from(err: Utf8Error) -> ParseError {
        ParseError::Utf8Error(err)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ParseError::Utf8Error(e) => write!(f, "{e}"),
            ParseError::NonAsciiChar => write!(f, "non ascii character is not supported"),
        }
    }
}

fn parse_input(buf: &[u8], len: usize) -> Result<[&str; MAX_ARGS], ParseError> {
    // バッファを文字列に変換
    let input = from_utf8(&buf[0..len])?;
    if !input.is_ascii() {
        return Err(ParseError::NonAsciiChar);
    };

    let mut items: [&str; MAX_ARGS] = [""; MAX_ARGS];
    for (i, item) in input.split_whitespace().enumerate() {
        items[i] = item;
    }

    Ok(items)
}

//
// コマンドを実行する
//

fn run_command(command: [&str; MAX_ARGS], history: &[[u8; MAX_ARGS]]) {
    match command[0] {
        "hello" => builtin_hello(),
        "help" => builtin_help(),
        "echo" => builtin_echo(command),
        "history" => builtin_history(history),
        _ => {
            println!("{}: comannd not found", command[0]);
            // println!("DEBUG: {:?}", command_str.as_bytes());
        }
    }
}

//
// ビルトインコマンド
//

fn builtin_hello() {
    println!("hello");
}

fn builtin_help() {
    let help_msg = r#"
Available commands:
    hello: Just says "hello"
    echo: Builtin echo command
"#;
    println!("{}", help_msg);
}

fn builtin_echo(args: [&str; MAX_ARGS]) {
    for (i, arg) in args[1..MAX_ARGS].iter().enumerate() {
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

fn builtin_history(history: &[[u8; MAX_ARGS]]) {
    for item in history.iter().rev() {
        let s = from_utf8(item).unwrap();
        if !s.is_empty() {
            println!("{}", s);
        }
    }
}

//
// main
//

user_main!(main);

fn main() {
    println!("Hello from shell!!");
    let mut history = [[0u8; BUF_SIZE]; HISTORY_SIZE];
    let mut i = 0;
    loop {
        print!("> ");

        // 入力を読む
        let mut buf = [0u8; BUF_SIZE];
        let len = match read_line(&mut buf) {
            Ok(len) => len,
            Err(e) => {
                print!("\n");
                println!("Error Reading Line: {:?}", e);
                continue;
            }
        };

        // 入力が無いとき
        if len == 0 {
            continue;
        }

        // 入力を文字列に変換する
        let command = match parse_input(&buf, len) {
            Ok(input) => input,
            Err(e) => {
                println!("Parse Error: {}", e);
                continue;
            }
        };

        // historyを保存する
        history[i % HISTORY_SIZE][..len].copy_from_slice(&buf[..len]);

        run_command(command, &history[0..i]);

        i += 1;
    }
}
