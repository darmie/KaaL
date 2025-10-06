# Multi-stage build for KaaL project
# Stage 1: Build seL4 kernel and set up environment
FROM trustworthysystems/sel4:latest AS builder

# Install Rust and build dependencies
RUN apt-get update && apt-get install -y \
    curl build-essential qemu-system-aarch64 \
    device-tree-compiler wget git python3 python3-pip \
    ninja-build tree && \
    rm -rf /var/lib/apt/lists/*

# Install Rust nightly
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain nightly
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup component add rust-src

# Build seL4 kernel for ARM64/QEMU
WORKDIR /opt
RUN git clone --depth 1 https://github.com/seL4/seL4.git && \
    cd seL4 && mkdir build && cd build && \
    cmake -G Ninja \
        -DCROSS_COMPILER_PREFIX=aarch64-linux-gnu- \
        -DKernelPlatform=qemu-arm-virt \
        -DKernelSel4Arch=aarch64 \
        -DKernelDebugBuild=TRUE \
        -DKernelPrinting=TRUE \
        .. && \
    ninja && \
    mkdir -p gen_headers/kernel gen_headers/sel4 gen_headers/interfaces gen_headers/api && \
    cp gen_config/kernel/gen_config.json gen_headers/kernel/ && \
    cp gen_config/kernel/gen_config.h gen_headers/kernel/ && \
    cp libsel4/gen_config/sel4/gen_config.json gen_headers/sel4/ && \
    cp libsel4/gen_config/sel4/gen_config.h gen_headers/sel4/ && \
    cp /opt/seL4/libsel4/include/interfaces/object-api.xml gen_headers/interfaces/ && \
    cp /opt/seL4/libsel4/sel4_arch_include/aarch64/interfaces/object-api-sel4-arch.xml gen_headers/interfaces/ && \
    cp /opt/seL4/libsel4/arch_include/arm/interfaces/object-api-arch.xml gen_headers/interfaces/ && \
    cp /opt/seL4/libsel4/include/api/syscall.xml gen_headers/api/

# Set environment for rust-sel4 bindings
ENV SEL4_DIR=/opt/seL4
ENV SEL4_BUILD_DIR=/opt/seL4/build
ENV SEL4_PLATFORM=qemu-arm-virt
ENV SEL4_PREFIX=/opt/seL4/build
ENV SEL4_INCLUDE_DIRS=/opt/seL4/build/gen_headers:/opt/seL4/build/libsel4/include:/opt/seL4/build/libsel4/autoconf:/opt/seL4/build/libsel4/sel4_arch_include/aarch64:/opt/seL4/build/libsel4/arch_include/arm:/opt/seL4/libsel4/include:/opt/seL4/libsel4/sel4_arch_include/aarch64:/opt/seL4/libsel4/arch_include/arm:/opt/seL4/libsel4/mode_include/64:/opt/seL4/libsel4/sel4_plat_include/qemu-arm-virt

# Stage 2: Build KaaL root task
COPY . /kaal
WORKDIR /kaal/examples/my-kaal-system
RUN cargo build --release \
    --target aarch64-unknown-none \
    --no-default-features \
    -Z unstable-options \
    -Zbuild-std=core,alloc,compiler_builtins \
    -Zbuild-std-features=compiler-builtins-mem

# Stage 3: Extract kernel and root task binaries
# QEMU can load both kernel and our KaaL root task separately
WORKDIR /output
RUN cp /opt/seL4/build/kernel.elf kernel.elf && \
    cp /kaal/examples/my-kaal-system/target/aarch64-unknown-none/release/my-kaal-system roottask.elf

# Output both binaries for QEMU multiboot
CMD ["sh", "-c", "cat kernel.elf && cat roottask.elf"]
