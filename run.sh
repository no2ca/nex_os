#!/bin/bash
set -xue

# QEMUのファイルパス
QEMU=qemu-system-riscv64

cargo fmt --all

cargo build -r --bin shell --target user-riscv64gc-unknown-none-elf.json
cp ./target/user-riscv64gc-unknown-none-elf/release/shell ./shell.elf

cargo build -r --bin kernel --target riscv64gc-unknown-none-elf
cp ./target/riscv64gc-unknown-none-elf/release/kernel ./kernel.elf

# QEMUを起動
$QEMU -machine virt -bios default -nographic -serial mon:stdio --no-reboot -kernel kernel.elf
