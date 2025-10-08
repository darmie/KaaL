# KaaL Bootable Image Builder
# Builds standalone KaaL root task and creates bootable seL4 image
FROM trustworthysystems/sel4:latest

# Install Rust and build dependencies
RUN apt-get update && apt-get install -y \
    curl build-essential qemu-system-aarch64 \
    device-tree-compiler git python3 python3-pip \
    ninja-build libxml2-utils && \
    rm -rf /var/lib/apt/lists/*

# Install Google repo tool
RUN curl https://storage.googleapis.com/git-repo-downloads/repo > /usr/local/bin/repo && \
    chmod a+x /usr/local/bin/repo

# Install Rust nightly
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain nightly
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup component add rust-src

# ===================================================================
# Stage 1: Build seL4 kernel (needed for rust-sel4 bindings)
# ===================================================================
WORKDIR /opt/seL4
RUN git clone --depth 1 https://github.com/seL4/seL4.git . && \
    mkdir build && cd build && \
    cmake -G Ninja \
        -DCROSS_COMPILER_PREFIX=aarch64-linux-gnu- \
        -DKernelPlatform=qemu-arm-virt \
        -DKernelSel4Arch=aarch64 \
        -DKernelDebugBuild=TRUE \
        -DKernelPrinting=TRUE \
        .. && \
    ninja

# Set environment for rust-sel4 bindings
ENV SEL4_DIR=/opt/seL4
ENV SEL4_BUILD_DIR=/opt/seL4/build
ENV SEL4_PLATFORM=qemu-arm-virt
ENV SEL4_PREFIX=/opt/seL4/build

# ===================================================================
# Stage 2: Build KaaL root task binary
# ===================================================================
COPY . /kaal-source
WORKDIR /kaal-source/examples/my-kaal-system

# Build KaaL as standalone ELF binary
RUN cargo build --release \
    --target aarch64-unknown-none \
    --no-default-features \
    -Z unstable-options \
    -Zbuild-std=core,alloc,compiler_builtins \
    -Zbuild-std-features=compiler-builtins-mem && \
    echo "=== KaaL binary built ===" && \
    ls -lh target/aarch64-unknown-none/release/my-kaal-system && \
    file target/aarch64-unknown-none/release/my-kaal-system

# ===================================================================
# Stage 3: Set up sel4test build infrastructure for boot image
# ===================================================================
WORKDIR /build
RUN repo init -u https://github.com/seL4/sel4test-manifest.git -b master && \
    repo sync

# Replace sel4test-driver binary with KaaL binary
WORKDIR /build/projects/sel4test/apps/sel4test-driver
RUN rm -rf src/* && \
    cp /kaal-source/examples/my-kaal-system/target/aarch64-unknown-none/release/my-kaal-system \
       /tmp/kaal-roottask.elf

# Create minimal CMakeLists.txt that uses pre-built binary
RUN cat > CMakeLists.txt << 'EOF'
cmake_minimum_required(VERSION 3.7.2)
project(sel4test-driver C ASM)

# Import pre-built KaaL binary
add_executable(sel4test-driver IMPORTED GLOBAL)
set_target_properties(sel4test-driver PROPERTIES
    IMPORTED_LOCATION "/tmp/kaal-roottask.elf"
)

# Declare as root server
include(rootserver)
DeclareRootserver(sel4test-driver)
EOF

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
