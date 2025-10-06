# KaaL Build using Local seL4 Submodule
# Uses the seL4 kernel in external/seL4 and seL4_tools for boot image creation
FROM trustworthysystems/sel4:latest

# Install dependencies including Rust
RUN apt-get update && apt-get install -y \
    curl build-essential qemu-system-aarch64 \
    device-tree-compiler git python3 python3-pip \
    ninja-build cmake cpio && \
    rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain nightly
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup component add rust-src

# Copy entire KaaL project (includes external/seL4 submodule)
COPY . /kaal
WORKDIR /kaal

# Build seL4 kernel from our submodule
WORKDIR /kaal/external/seL4
RUN mkdir -p build && cd build && \
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
    cp /kaal/external/seL4/libsel4/include/interfaces/object-api.xml gen_headers/interfaces/ && \
    cp /kaal/external/seL4/libsel4/sel4_arch_include/aarch64/interfaces/object-api-sel4-arch.xml gen_headers/interfaces/ && \
    cp /kaal/external/seL4/libsel4/arch_include/arm/interfaces/object-api-arch.xml gen_headers/interfaces/ && \
    cp /kaal/external/seL4/libsel4/include/api/syscall.xml gen_headers/api/

# Set seL4 environment for Cargo builds
ENV SEL4_DIR=/kaal/external/seL4
ENV SEL4_BUILD_DIR=/kaal/external/seL4/build
ENV SEL4_PLATFORM=qemu-arm-virt
ENV SEL4_PREFIX=/kaal/external/seL4/build
ENV SEL4_INCLUDE_DIRS=/kaal/external/seL4/build/gen_headers:/kaal/external/seL4/build/libsel4/include:/kaal/external/seL4/build/libsel4/autoconf:/kaal/external/seL4/build/libsel4/sel4_arch_include/aarch64:/kaal/external/seL4/build/libsel4/arch_include/arm:/kaal/external/seL4/libsel4/include:/kaal/external/seL4/libsel4/sel4_arch_include/aarch64:/kaal/external/seL4/libsel4/arch_include/arm:/kaal/external/seL4/libsel4/mode_include/64:/kaal/external/seL4/libsel4/sel4_plat_include/qemu-arm-virt

# Build KaaL root task
WORKDIR /kaal/examples/my-kaal-system
RUN cargo build --release \
    --target aarch64-unknown-none \
    --no-default-features \
    -Z unstable-options \
    -Zbuild-std=core,alloc,compiler_builtins \
    -Zbuild-std-features=compiler-builtins-mem

# Clone seL4_tools for elfloader
WORKDIR /kaal
RUN git clone --depth 1 https://github.com/seL4/seL4_tools.git external/seL4_tools

# Create CPIO archive with kernel + root task
WORKDIR /kaal/build
RUN aarch64-linux-gnu-strip /kaal/external/seL4/build/kernel.elf -o kernel.elf && \
    aarch64-linux-gnu-strip /kaal/examples/my-kaal-system/target/aarch64-unknown-none/release/my-kaal-system -o roottask && \
    echo "kernel.elf" > files.txt && \
    echo "roottask" >> files.txt && \
    cpio -o -H newc -O archive.cpio < files.txt && \
    aarch64-linux-gnu-objcopy -I binary -O elf64-littleaarch64 -B aarch64 \
        --rename-section .data=.rodata,alloc,load,readonly,data,contents \
        --redefine-sym _binary_archive_cpio_start=_archive_start \
        --redefine-sym _binary_archive_cpio_end=_archive_end \
        --redefine-sym _binary_archive_cpio_size=_archive_size \
        archive.cpio archive.o

# Create a minimal gen_config.cmake that elfloader expects
# This converts key settings from the JSON to CMake format
RUN echo "# Auto-generated kernel config for elfloader" > /kaal/external/seL4/build/gen_config/kernel/gen_config.cmake && \
    echo "set(KernelPlatform \"qemu-arm-virt\")" >> /kaal/external/seL4/build/gen_config/kernel/gen_config.cmake && \
    echo "set(KernelSel4Arch \"aarch64\")" >> /kaal/external/seL4/build/gen_config/kernel/gen_config.cmake && \
    echo "set(KernelARMPlatform \"qemu-arm-virt\")" >> /kaal/external/seL4/build/gen_config/kernel/gen_config.cmake && \
    echo "set(KernelDebugBuild TRUE)" >> /kaal/external/seL4/build/gen_config/kernel/gen_config.cmake && \
    echo "set(KernelPrinting TRUE)" >> /kaal/external/seL4/build/gen_config/kernel/gen_config.cmake

# Build elfloader with CPIO embedded
# elfloader needs both seL4_tools cmake helpers AND kernel flags
WORKDIR /kaal/external/seL4_tools/elfloader-tool
RUN mkdir build && cd build && \
    cmake -G Ninja \
        -DCROSS_COMPILER_PREFIX=aarch64-linux-gnu- \
        -DKernelARMPlatform=qemu-arm-virt \
        -DKernelSel4Arch=aarch64 \
        -DKERNEL_FLAGS_PATH=/kaal/external/seL4/build/gen_config/kernel/gen_config.cmake \
        -DElfloaderArchive=/kaal/build/archive.o \
        -DCMAKE_MODULE_PATH="/kaal/external/seL4_tools/cmake-tool/helpers;/kaal/external/seL4/tools" \
        .. && \
    ninja && \
    echo "=== Elfloader built successfully ===" && \
    ls -la elfloader

# Extract final bootable image
CMD ["sh", "-c", "ls -la /kaal/external/seL4_tools/elfloader-tool/build/ && cat /kaal/external/seL4_tools/elfloader-tool/build/elfloader"]
