#!/bin/bash
set -xue

# QEMUのファイルパス
QEMU=qemu-system-riscv64

cargo fmt --all
cargo build -r
cp ./target/riscv64gc-unknown-none-elf/release/nex ./kernel.elf

# QEMUを起動
$QEMU -machine virt -bios default -nographic -serial mon:stdio --no-reboot -kernel kernel.elf
