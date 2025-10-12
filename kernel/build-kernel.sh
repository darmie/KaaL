#!/bin/bash
set -e
cd "$(dirname "$0")"
export RUSTC_WRAPPER=""
export RUSTC="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin/rustc"
$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin/cargo build \
  --release \
  --bin kaal-kernel \
  --target aarch64-unknown-none \
  -Z build-std=core,alloc

echo "✓ Kernel built successfully"
/bin/ls -lh target/aarch64-unknown-none/release/kaal-kernel
file target/aarch64-unknown-none/release/kaal-kernel
