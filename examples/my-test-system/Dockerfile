FROM ghcr.io/darmie/kaal-builder:latest
COPY . /project
WORKDIR /project
RUN cargo build --release \
    --target aarch64-unknown-none \
    --no-default-features \
    --features runtime \
    -Z unstable-options \
    -Zbuild-std=core,alloc,compiler_builtins \
    -Zbuild-std-features=compiler-builtins-mem
CMD ["cat", "/project/target/aarch64-unknown-none/release/my-test-system"]
