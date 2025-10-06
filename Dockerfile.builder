# KaaL Builder Image
#
# This creates a reusable build environment for KaaL-based projects.
# Users volume-mount their project and build inside this container.
#
# Usage:
#   docker run --rm -v $(pwd):/project kaal-builder
#
# Or use in user's Dockerfile:
#   FROM kaal-builder
#   COPY . /project
#   RUN cd /project && cargo build --release --features runtime

FROM trustworthysystems/sel4:latest

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
    # Setup headers for rust-sel4
    mkdir -p gen_headers/kernel gen_headers/sel4 gen_headers/interfaces gen_headers/api && \
    cp gen_config/kernel/gen_config.{json,h} gen_headers/kernel/ && \
    cp libsel4/gen_config/sel4/gen_config.{json,h} gen_headers/sel4/ && \
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

# Working directory for user projects
WORKDIR /project

# Default command shows usage
CMD ["/bin/bash", "-c", "echo 'KaaL Builder v0.1.0\n\nUsage:\n  docker run --rm -v $(pwd):/project kaal-builder cargo build --release --features runtime\n\nOr create your own Dockerfile:\n  FROM kaal-builder\n  COPY . /project\n  RUN cargo build --release --features runtime\n'"]
