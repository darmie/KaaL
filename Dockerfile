# KaaL Development Container for Mac Silicon
# Builds KaaL with seL4 Runtime (rust-sel4)

FROM trustworthysystems/sel4:latest

# Install Rust and additional dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    qemu-system-aarch64 \
    device-tree-compiler \
    wget \
    git \
    python3 \
    python3-pip \
    ninja-build \
    tree \
    && rm -rf /var/lib/apt/lists/*

# Install Rust nightly (required for seL4 features)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly
ENV PATH="/root/.cargo/bin:${PATH}"
# Add rust-src component for -Zbuild-std
RUN rustup component add rust-src

# Clone and build seL4 kernel for QEMU ARM64
WORKDIR /opt
RUN git clone --depth 1 https://github.com/seL4/seL4.git && \
    cd seL4 && \
    mkdir build && \
    cd build && \
    cmake -G Ninja \
        -DCROSS_COMPILER_PREFIX=aarch64-linux-gnu- \
        -DKernelPlatform=qemu-arm-virt \
        -DKernelSel4Arch=aarch64 \
        -DKernelDebugBuild=TRUE \
        -DKernelPrinting=TRUE \
        .. && \
    ninja && \
    echo "=== seL4 build complete - setting up required files ===" && \
    mkdir -p gen_headers/kernel gen_headers/sel4 gen_headers/interfaces gen_headers/api && \
    cp gen_config/kernel/gen_config.json gen_headers/kernel/gen_config.json && \
    cp gen_config/kernel/gen_config.h gen_headers/kernel/gen_config.h && \
    cp libsel4/gen_config/sel4/gen_config.json gen_headers/sel4/gen_config.json && \
    cp libsel4/gen_config/sel4/gen_config.h gen_headers/sel4/gen_config.h && \
    cp /opt/seL4/libsel4/include/interfaces/object-api.xml gen_headers/interfaces/ && \
    cp /opt/seL4/libsel4/sel4_arch_include/aarch64/interfaces/object-api-sel4-arch.xml gen_headers/interfaces/ && \
    cp /opt/seL4/libsel4/arch_include/arm/interfaces/object-api-arch.xml gen_headers/interfaces/ && \
    cp /opt/seL4/libsel4/include/api/syscall.xml gen_headers/api/ && \
    echo "=== File setup complete - verifying ===" && \
    ls -la gen_headers/kernel/ && \
    ls -la gen_headers/sel4/ && \
    ls -la gen_headers/interfaces/ && \
    ls -la gen_headers/api/

# Set environment for rust-sel4 bindings
ENV SEL4_DIR=/opt/seL4
ENV SEL4_BUILD_DIR=/opt/seL4/build
ENV SEL4_PLATFORM=qemu-arm-virt
# Point to gen_headers first (has all generated files including interfaces/)
ENV SEL4_PREFIX=/opt/seL4/build
ENV SEL4_INCLUDE_DIRS=/opt/seL4/build/gen_headers:/opt/seL4/build/libsel4/include:/opt/seL4/build/libsel4/autoconf:/opt/seL4/build/libsel4/sel4_arch_include/aarch64:/opt/seL4/build/libsel4/arch_include/arm:/opt/seL4/libsel4/include:/opt/seL4/libsel4/sel4_arch_include/aarch64:/opt/seL4/libsel4/arch_include/arm:/opt/seL4/libsel4/mode_include/64:/opt/seL4/libsel4/sel4_plat_include/qemu-arm-virt

# Set working directory for KaaL
WORKDIR /workspace

# Copy KaaL source
COPY . /workspace

# Build KaaL cap_broker with runtime backend
WORKDIR /workspace
# Use runtime feature which enables sel4-platform with full seL4 syscalls
RUN cd runtime/cap_broker && \
    cargo build --release \
    --no-default-features \
    --features runtime \
    -Z unstable-options \
    -Zbuild-std=core,alloc,compiler_builtins \
    -Zbuild-std-features=compiler-builtins-mem

# Copy built artifacts to build directory for easy access
RUN mkdir -p /workspace/build && \
    find /workspace/target -name "*.a" -o -name "*.rlib" | head -10 | xargs -I {} cp {} /workspace/build/ 2>/dev/null || true

# Default command
CMD ["/bin/bash"]
