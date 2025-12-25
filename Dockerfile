FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# 基本ツール
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    git \
    llvm \
    lld \
    clang \
    qemu-system \
    nasm \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Rust components
RUN rustup toolchain install nightly \
 && rustup default nightly \
 && rustup component add rust-src llvm-tools-preview

# cargo-binutils
RUN cargo install cargo-binutils

WORKDIR /work
