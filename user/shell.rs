#![no_std]
#![no_main]

use core::{
    error,
    fmt::{self, Display},
    str::{Utf8Error, from_utf8},
};

use userlib::{self, Writer, print, println, read_byte, user_main};

const HISTORY_SIZE: usize = 128;
const BUF_SIZE: usize = 128;
const ARGS_SIZE: usize = 128;

//
// 文字列を受け取る
//

#[derive(Debug)]
enum ReadLineError {
    Overflow,
}

impl Display for ReadLineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ReadLineError::Overflow => write!(f, "buffer overflow (buffer size is {})", BUF_SIZE),
        }
    }
}

impl error::Error for ReadLineError {}

//
// パースに関するエラー
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

impl error::Error for ParseError {}

//
// ビルトインコマンド
//

fn builtin_hello() {
    println!("hello");
}

fn builtin_ohgiri() {
    println!("大喜利が足りないぜ");
}

fn builtin_help() {
    let help_msg = "
Available commands:
    hello:\tJust says 'hello'
    echo:\tBuiltin echo command
    history:\tShow history
";
    println!("{}", help_msg);
}

fn builtin_echo(args: [&str; ARGS_SIZE]) {
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

fn builtin_history(history: &[[u8; BUF_SIZE]], history_len: &[usize; HISTORY_SIZE]) {
    for (item, len) in history.iter().zip(history_len.iter()).rev() {
        if *len == 0 {
            continue;
        }
        if let Ok(s) = from_utf8(&item[..*len]) {
            println!("{}", s);
        }
    }
}

//
// main
//

use thiserror::Error;
#[derive(Error, Debug)]
enum ShellError {
    #[error("Error Reading Line: {0}")]
    ReadLineError(#[from] ReadLineError),
    #[error("Parse Error: {0}")]
    ParseError(#[from] ParseError),
}

struct Console {
    history: [[u8; BUF_SIZE]; HISTORY_SIZE],
    history_len: [usize; HISTORY_SIZE],
    count: usize,
    buf: [u8; BUF_SIZE],
}

impl Console {
    fn new() -> Self {
        Self {
            history: [[0u8; BUF_SIZE]; HISTORY_SIZE],
            history_len: [0usize; HISTORY_SIZE],
            count: 0,
            buf: [0u8; BUF_SIZE],
        }
    }

    /// 一行読み取り，読み取ったバイト数を返す
    fn read_line(&mut self) -> Result<usize, ReadLineError> {
        // 入力を受け取る
        let mut index = 0;
        let mut hstry_idx = self.count;
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
                        self.buf[index - 1] = 0;
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
                    let c2 = read_byte();
                    let c3 = read_byte();
                    // 上向き矢印のとき
                    if c2 == b'[' && c3 == b'A' {
                        hstry_idx = hstry_idx.saturating_sub(1);
                        for _ in 0..index {
                            Writer::write_byte(0x8);
                            Writer::write_byte(0x20);
                            Writer::write_byte(0x8);
                        }
                        for i in 0..self.history_len[hstry_idx] {
                            Writer::write_byte(self.history[hstry_idx][i]);
                        }
                        index = self.history_len[hstry_idx];
                    }
                    // 下向き矢印のとき
                    if c2 == b'[' && c3 == b'B' {
                        // countの意味を考えると現在実行されるコマンドが入る空白の場所
                        // これよりも大きくなってほしくないため
                        if hstry_idx < self.count {
                            hstry_idx += 1;
                        }
                        for _ in 0..index {
                            Writer::write_byte(0x8);
                            Writer::write_byte(0x20);
                            Writer::write_byte(0x8);
                        }
                        for i in 0..self.history_len[hstry_idx] {
                            Writer::write_byte(self.history[hstry_idx][i]);
                        }
                        index = self.history_len[hstry_idx];
                        // println!("[debug] hstry_idx={}", hstry_idx);
                    }
                    continue;
                }
                _ => {}
            }
            // バッファに収まる場合のみ書き込む
            if index < BUF_SIZE {
                self.buf[index] = c;
            } else {
                return Err(ReadLineError::Overflow);
            }
            Writer::write_byte(c);
            index += 1;
        }

        Ok(index)
    }

    /// バッファに入っているバイト数を受け取る
    ///
    /// 引数ごとに分割された文字列スライスのリストを返す
    fn parse_input(&self, input_len: usize) -> Result<[&str; ARGS_SIZE], ParseError> {
        // バッファを文字列に変換
        let input = from_utf8(&self.buf[0..input_len])?;
        if !input.is_ascii() {
            return Err(ParseError::NonAsciiChar);
        };

        let mut items: [&str; ARGS_SIZE] = [""; ARGS_SIZE];
        for (i, item) in input.split_whitespace().enumerate() {
            items[i] = item;
        }

        Ok(items)
    }

    fn run_command(&self, cmd: [&str; ARGS_SIZE]) {
        match cmd[0] {
            "hello" => builtin_hello(),
            "help" => builtin_help(),
            "echo" => builtin_echo(cmd),
            "history" => builtin_history(&self.history, &self.history_len),
            "ohgiri" => builtin_ohgiri(),
            _ => {
                println!("{}: command not found", cmd[0]);
                // println!("DEBUG: {:?}", command_str.as_bytes());
            }
        }
    }

    #[inline]
    fn save_history(&mut self, input_len: usize) {
        let index = self.count % HISTORY_SIZE;
        self.history[index][..input_len].copy_from_slice(&self.buf[..input_len]);
        self.history_len[index] = input_len;
    }

    fn prompt(&mut self) -> Result<(), ShellError> {
        print!("> ");

        let input_len = self.read_line()?;

        if input_len == 0 {
            return Ok(());
        }
        let cmd = self.parse_input(input_len)?;
        self.run_command(cmd);

        self.save_history(input_len);
        self.count += 1;
        Ok(())
    }
}

user_main!(main);

fn main() {
    shell();
}

fn shell() {
    #[cfg(feature = "shell-test")]
    test_runner();

    let mut con = Console::new();

    loop {
        if let Err(e) = con.prompt() {
            println!("{e}");
        }
    }
}

//
// tests
//

#[cfg(feature = "shell-test")]
pub fn test_runner() {
    println!("Starting Test...");
    test_echo();
    test_many_echo();
    test_history();
    test_many_history();
    loop {}
}

#[cfg(feature = "shell-test")]
fn test_echo() {
    println!("[test] test_echo:");
    let mut cmd = [""; ARGS_SIZE];
    cmd[0] = "echo";
    cmd[1] = "foo";
    let con = Console::new();
    con.run_command(cmd);
    println!("[OK]");
}

#[cfg(feature = "shell-test")]
fn test_many_echo() {
    println!("[test] test_echo:");
    let mut cmd = [""; ARGS_SIZE];
    cmd[0] = "echo";
    cmd[1] = "foo";
    cmd[2] = "bar";
    cmd[3] = "hoge";
    cmd[4] = "piyo";
    let con = Console::new();
    con.run_command(cmd);
    println!("[OK]");
}

#[cfg(feature = "shell-test")]
fn test_history() {
    println!("[test] test_history:");
    let mut cmd = [""; ARGS_SIZE];
    cmd[0] = "history";
    let mut con = Console::new();
    let dummy = "dummy";
    for (i, b) in dummy.bytes().enumerate() {
        con.history[0][i] = b;
    }
    con.history_len[0] = dummy.len();
    con.run_command(cmd);
    println!("[OK]");
}

#[cfg(feature = "shell-test")]
fn test_many_history() {
    println!("[test] test_many_history:");
    let mut cmd = [""; ARGS_SIZE];
    cmd[0] = "history";

    let mut con = Console::new();
    let dummy = "dummy";
    for i in 0..5 {
        for (j, b) in dummy.bytes().enumerate() {
            con.history[i][j] = b;
        }
        con.history_len[i] = dummy.len();
    }
    con.run_command(cmd);
    println!("[OK]");
}
