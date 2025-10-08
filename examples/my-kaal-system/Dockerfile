# KaaL Bootable Image Builder
# Uses sel4test infrastructure with C wrapper calling Rust root task
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
# Stage 1: Build seL4 kernel for rust-sel4 bindings
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
    ninja && \
    cd /opt/seL4/build/libsel4/include && \
    mkdir -p kernel sel4 api interfaces && \
    cp ../../gen_config/kernel/gen_config.json kernel/ && \
    cp ../../gen_config/kernel/gen_config.h kernel/ && \
    cp ../gen_config/sel4/gen_config.json sel4/ && \
    cp ../gen_config/sel4/gen_config.h sel4/ && \
    cp /opt/seL4/libsel4/include/api/syscall.xml api/ && \
    cp /opt/seL4/libsel4/include/interfaces/object-api.xml interfaces/ && \
    cp /opt/seL4/libsel4/sel4_arch_include/aarch64/interfaces/object-api-sel4-arch.xml interfaces/ && \
    cp /opt/seL4/libsel4/arch_include/arm/interfaces/object-api-arch.xml interfaces/

# Set environment for rust-sel4 bindings
ENV SEL4_DIR=/opt/seL4
ENV SEL4_BUILD_DIR=/opt/seL4/build
ENV SEL4_PLATFORM=qemu-arm-virt
ENV SEL4_PREFIX=/opt/seL4/build

# Verify the headers were copied correctly
RUN ls -la /opt/seL4/build/libsel4/include/kernel/ && \
    ls -la /opt/seL4/build/libsel4/include/sel4/ && \
    echo "Headers verified"

# ===================================================================
# Stage 2: Build KaaL Rust library
# ===================================================================
COPY . /kaal-source
WORKDIR /kaal-source/examples/my-kaal-system

# Build KaaL as static library
RUN cargo build --release \
    --target aarch64-unknown-none \
    --no-default-features \
    -Z unstable-options \
    -Zbuild-std=core,alloc,compiler_builtins \
    -Zbuild-std-features=compiler-builtins-mem && \
    echo "=== KaaL library built ===" && \
    ls -lh target/aarch64-unknown-none/release/libmy_kaal_system.a

# ===================================================================
# Stage 3: Set up sel4test with our wrapper
# ===================================================================
WORKDIR /build
RUN repo init -u https://github.com/seL4/sel4test-manifest.git -b master && \
    repo sync

# Replace sel4test-driver source with our wrapper
WORKDIR /build/projects/sel4test/apps/sel4test-driver
RUN rm -rf src/* CMakeLists.txt

# Copy wrapper files
COPY examples/my-kaal-system/wrapper/main.c src/
COPY examples/my-kaal-system/wrapper/CMakeLists.txt .

# Symlink KaaL library where CMake expects it
RUN mkdir -p target/aarch64-unknown-none/release && \
    ln -s /kaal-source/examples/my-kaal-system/target/aarch64-unknown-none/release/libmy_kaal_system.a \
          target/aarch64-unknown-none/release/libmy_kaal_system.a

# ===================================================================
# Stage 4: Build bootable image
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
