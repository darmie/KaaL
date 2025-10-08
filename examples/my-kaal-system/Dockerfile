# KaaL Bootable Image Builder
# Uses sel4test infrastructure to create bootable seL4 images with KaaL root task
FROM trustworthysystems/sel4:latest

# Install Rust and build dependencies
RUN apt-get update && apt-get install -y \
    curl build-essential qemu-system-aarch64 \
    device-tree-compiler git python3 python3-pip \
    ninja-build repo xmllint && \
    rm -rf /var/lib/apt/lists/*

# Install Rust nightly
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain nightly
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup component add rust-src

# ===================================================================
# Stage 1: Set up sel4test build infrastructure
# ===================================================================
WORKDIR /build
RUN repo init -u https://github.com/seL4/sel4test-manifest.git -b master && \
    repo sync

# ===================================================================
# Stage 2: Build KaaL Rust library
# ===================================================================
COPY . /kaal-source
WORKDIR /kaal-source/examples/my-kaal-system

# Build KaaL as a static library (libmy_kaal_system.a)
RUN cargo build --release \
    --target aarch64-unknown-none \
    --no-default-features \
    -Z unstable-options \
    -Zbuild-std=core,alloc,compiler_builtins \
    -Zbuild-std-features=compiler-builtins-mem && \
    echo "=== KaaL Rust library built ===" && \
    ls -lh target/aarch64-unknown-none/release/*.a

# ===================================================================
# Stage 3: Replace sel4test-driver with KaaL wrapper
# ===================================================================

# Copy wrapper files to sel4test-driver location
WORKDIR /build/projects/sel4test/apps/sel4test-driver/src
RUN rm -f *.c *.h
COPY /kaal-source/examples/my-kaal-system/wrapper/main.c .

# Update sel4test-driver CMakeLists.txt to use our wrapper
WORKDIR /build/projects/sel4test/apps/sel4test-driver
RUN rm -f CMakeLists.txt
COPY /kaal-source/examples/my-kaal-system/wrapper/CMakeLists.txt .

# Create symlink to KaaL Rust library so CMake can find it
RUN mkdir -p /build/projects/sel4test/apps/sel4test-driver/target/aarch64-unknown-none/release && \
    ln -sf /kaal-source/examples/my-kaal-system/target/aarch64-unknown-none/release/libmy_kaal_system.a \
           /build/projects/sel4test/apps/sel4test-driver/target/aarch64-unknown-none/release/libmy-kaal-system.a

# ===================================================================
# Stage 4: Build bootable image with KaaL
# ===================================================================
WORKDIR /build
RUN mkdir -p build && cd build && \
    ../init-build.sh -DPLATFORM=qemu-arm-virt -DAARCH64=1 && \
    ninja

# Extract bootable image
RUN cp /build/build/images/sel4test-driver-image-arm-qemu-arm-virt /boot-image.elf && \
    echo "=== Bootable KaaL image created ===" && \
    ls -lh /boot-image.elf && \
    file /boot-image.elf

# Default command: output the bootable image
CMD ["cat", "/boot-image.elf"]
