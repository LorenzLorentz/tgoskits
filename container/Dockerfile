FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive
ARG QEMU_VERSION=10.2.1

ENV TZ=Etc/UTC \
    RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:/opt/qemu-${QEMU_VERSION}/bin:/opt/x86_64-linux-musl-cross/bin:/opt/aarch64-linux-musl-cross/bin:/opt/riscv64-linux-musl-cross/bin:/opt/loongarch64-linux-musl-cross/bin:${PATH}

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        bzip2 \
        ca-certificates \
        build-essential \
        clang \
        cmake \
        curl \
        dosfstools \
        e2fsprogs \
        git \
        libavcodec-dev \
        libavdevice-dev \
        libavfilter-dev \
        libavformat-dev \
        libavutil-dev \
        libglib2.0-dev \
        libclang-19-dev \
        libpixman-1-dev \
        libslirp-dev \
        libswresample-dev \
        libswscale-dev \
        libudev-dev \
        make \
        meson \
        ninja-build \
        pkg-config \
        python3 \
        python3-venv \
        qemu-user-static \
        wget \
        xz-utils \
        zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

RUN set -eux; \
    wget -O /tmp/qemu.tar.xz "https://download.qemu.org/qemu-${QEMU_VERSION}.tar.xz"; \
    tar -xf /tmp/qemu.tar.xz -C /tmp; \
    cd "/tmp/qemu-${QEMU_VERSION}"; \
    ./configure \
      --prefix="/opt/qemu-${QEMU_VERSION}" \
      --target-list=loongarch64-softmmu,loongarch64-linux-user,riscv64-softmmu,riscv64-linux-user,aarch64-softmmu,aarch64-linux-user,x86_64-softmmu,x86_64-linux-user \
      --enable-slirp \
      --disable-docs \
      --disable-gtk \
      --disable-sdl \
      --disable-vte \
      --disable-werror; \
    make -j"$(nproc)"; \
    make install; \
    rm -rf "/tmp/qemu-${QEMU_VERSION}" /tmp/qemu.tar.xz

RUN set -eux; \
    for arch in aarch64 riscv64 x86_64 loongarch64; do \
      file="${arch}-linux-musl-cross.tgz"; \
      if ! wget -q "https://github.com/arceos-org/setup-musl/releases/download/prebuilt/${file}" -O "/tmp/${file}"; then \
        if [ "${arch}" = "loongarch64" ]; then \
          wget -q "https://github.com/LoongsonLab/oscomp-toolchains-for-oskernel/releases/download/loongarch64-linux-musl-cross-gcc-13.2.0/${file}" -O "/tmp/${file}"; \
        else \
          wget -q "https://musl.cc/${file}" -O "/tmp/${file}"; \
        fi; \
      fi; \
      tar -xzf "/tmp/${file}" -C /opt; \
      rm -f "/tmp/${file}"; \
    done

COPY rust-toolchain.toml /tmp/rust-toolchain/rust-toolchain.toml

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain none

RUN set -eux; \
    cd /tmp/rust-toolchain; \
    rustc --version; \
    cargo --version; \
    cargo install cargo-binutils

# eBPF userspace demo toolchain (test-suit/starryos/ebpf): the aya programs
# under os/StarryOS/user/ebpf/ cross-compile their bytecode to
# `bpfel-unknown-none` via `bpf-linker`, and their std loaders link against
# musl. `bpf-linker` reads the bitcode emitted by the nightly rustc, so it must
# be built against the *same* LLVM major version that rustc ships (currently
# LLVM 22). Install llvm-22 dev libraries from apt.llvm.org and build
# bpf-linker against them.
RUN set -eux; \
    install -d /etc/apt/keyrings; \
    wget -qO /etc/apt/keyrings/llvm-snapshot.asc https://apt.llvm.org/llvm-snapshot.gpg.key; \
    . /etc/os-release; \
    echo "deb [signed-by=/etc/apt/keyrings/llvm-snapshot.asc] http://apt.llvm.org/${VERSION_CODENAME}/ llvm-toolchain-${VERSION_CODENAME}-22 main" \
        > /etc/apt/sources.list.d/llvm-22.list; \
    apt-get update; \
    apt-get install -y --no-install-recommends llvm-22-dev libpolly-22-dev; \
    rm -rf /var/lib/apt/lists/*

# - rust-src + the musl targets are needed by the pinned toolchain to build the
#   aya loaders (std musl binaries) for the demo arches.
# - aya-build shells out to `rustup run nightly cargo build --target
#   bpfel-unknown-none -Z build-std=core`, so a generic `nightly` channel with
#   rust-src must also be present.
# - bpf-linker is built with the matching llvm-22 feature.
RUN set -eux; \
    cd /tmp/rust-toolchain; \
    rustup component add rust-src; \
    rustup target add \
        x86_64-unknown-linux-musl \
        aarch64-unknown-linux-musl \
        riscv64gc-unknown-linux-musl; \
    rustup toolchain install nightly --profile minimal --component rust-src; \
    LLVM_SYS_221_PREFIX=/usr/lib/llvm-22 \
        cargo install bpf-linker --no-default-features --features llvm-22

WORKDIR /workspace

CMD ["/bin/bash"]
