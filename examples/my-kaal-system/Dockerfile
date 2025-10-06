# KaaL Build using seL4 Project Infrastructure
# This creates a proper seL4 project structure and uses init-build.sh
FROM trustworthysystems/sel4:latest

# Install dependencies including Rust
RUN apt-get update && apt-get install -y \
    curl build-essential qemu-system-aarch64 \
    device-tree-compiler git python3 python3-pip \
    ninja-build cmake && \
    rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain nightly
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup component add rust-src

# Set up seL4 project using proper structure
WORKDIR /project

# Clone seL4 project components
RUN git clone --depth 1 https://github.com/seL4/seL4.git kernel && \
    git clone --depth 1 https://github.com/seL4/seL4_tools.git tools && \
    git clone --depth 1 https://github.com/seL4/util_libs.git projects/util_libs && \
    git clone --depth 1 https://github.com/seL4/seL4_libs.git projects/seL4_libs

# Copy entire KaaL into project structure (maintains dependency paths)
COPY . /kaal

# Set up seL4 environment variables for Cargo builds
ENV SEL4_DIR=/project/kernel
ENV SEL4_BUILD_DIR=/project/kernel/build
ENV SEL4_PLATFORM=qemu-arm-virt

# Pre-build: Build seL4 kernel first (needed for headers)
WORKDIR /project/kernel
RUN mkdir build && cd build && \
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
    cp /project/kernel/libsel4/include/interfaces/object-api.xml gen_headers/interfaces/ && \
    cp /project/kernel/libsel4/sel4_arch_include/aarch64/interfaces/object-api-sel4-arch.xml gen_headers/interfaces/ && \
    cp /project/kernel/libsel4/arch_include/arm/interfaces/object-api-arch.xml gen_headers/interfaces/ && \
    cp /project/kernel/libsel4/include/api/syscall.xml gen_headers/api/

# Set seL4 environment variables with complete paths
ENV SEL4_PREFIX=/project/kernel/build
ENV SEL4_INCLUDE_DIRS=/project/kernel/build/gen_headers:/project/kernel/build/libsel4/include:/project/kernel/build/libsel4/autoconf:/project/kernel/build/libsel4/sel4_arch_include/aarch64:/project/kernel/build/libsel4/arch_include/arm:/project/kernel/libsel4/include:/project/kernel/libsel4/sel4_arch_include/aarch64:/project/kernel/libsel4/arch_include/arm:/project/kernel/libsel4/mode_include/64:/project/kernel/libsel4/sel4_plat_include/qemu-arm-virt

# Pre-build: Compile Rust root task (in original location to preserve Cargo paths)
WORKDIR /kaal/examples/my-kaal-system
RUN cargo build --release \
    --target aarch64-unknown-none \
    --no-default-features \
    -Z unstable-options \
    -Zbuild-std=core,alloc,compiler_builtins \
    -Zbuild-std-features=compiler-builtins-mem

# Now copy into seL4 project structure for final image creation
WORKDIR /project
RUN cp -r /kaal/examples/my-kaal-system projects/kaal-system

# Create init-build wrapper script
RUN echo '#!/bin/bash' > init-build.sh && \
    echo 'set -e' >> init-build.sh && \
    echo 'SOURCE_DIR=$(cd $(dirname "${BASH_SOURCE[0]}") && pwd)' >> init-build.sh && \
    echo ': ${BUILD_DIR:="$(pwd)"}' >> init-build.sh && \
    echo 'cd "$BUILD_DIR"' >> init-build.sh && \
    echo 'cmake -DCROSS_COMPILER_PREFIX=aarch64-linux-gnu- \\' >> init-build.sh && \
    echo '      -DPLATFORM=qemu-arm-virt \\' >> init-build.sh && \
    echo '      -DKernelSel4Arch=aarch64 \\' >> init-build.sh && \
    echo '      -DKernelDebugBuild=TRUE \\' >> init-build.sh && \
    echo '      -DKernelPrinting=TRUE \\' >> init-build.sh && \
    echo '      -G Ninja \\' >> init-build.sh && \
    echo '      "$@" \\' >> init-build.sh && \
    echo '      "${SOURCE_DIR}/projects/kaal-system"' >> init-build.sh && \
    chmod +x init-build.sh

# Build using seL4's infrastructure
WORKDIR /project/build
RUN ../init-build.sh && ninja

# Extract bootable image
CMD ["sh", "-c", "ls -la images/ && cat images/*"]
