#!/bin/bash
set -xue

# QEMUのファイルパス
QEMU=qemu-system-riscv64

cargo fmt --all

cargo build -r --bin sh --target user/user-riscv64gc-unknown-none-elf.json
cp ./target/user-riscv64gc-unknown-none-elf/release/sh ./sh.elf

cargo build -r --bin ps --target user/user-riscv64gc-unknown-none-elf.json
cp ./target/user-riscv64gc-unknown-none-elf/release/ps ./ps.elf

cargo build -r --bin kernel --target kernel/kernel-riscv64gc-unknown-none-elf.json
cp ./target/kernel-riscv64gc-unknown-none-elf/release/kernel ./kernel.elf

# QEMUを起動
$QEMU -machine virt -bios default -nographic -serial mon:stdio --no-reboot -kernel kernel.elf
