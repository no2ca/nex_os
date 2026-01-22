cargo fmt --all
cargo build --features shell-test --bin shell --target user/user-riscv64gc-unknown-none-elf.json
cp ./target/user-riscv64gc-unknown-none-elf/debug/shell ./shell.elf

cargo build -r --bin kernel --target kernel/kernel-riscv64gc-unknown-none-elf.json
cp ./target/kernel-riscv64gc-unknown-none-elf/release/kernel ./kernel.elf


qemu-system-riscv64 -machine virt -bios default -nographic -serial mon:stdio --no-reboot -kernel kernel.elf

