cmake_minimum_required(VERSION 3.20)
project(ebpf-profile NONE)

# The profile demo is an aya eBPF userspace program (its own nested Cargo
# workspace under os/StarryOS/user/ebpf/profile/). It is *generated* by
# `cargo xtask starry user-ebpf build`, not checked in as a binary, so build it
# here at configure time and install the resulting musl ELF + its launcher into
# the guest rootfs.
get_filename_component(WORKSPACE_ROOT
    "${CMAKE_CURRENT_SOURCE_DIR}/../../../../../.." ABSOLUTE)

# Map the cross toolchain's processor to the `cargo xtask` arch name and the
# matching `*-unknown-linux-musl` Rust target the program is built for.
if(CMAKE_SYSTEM_PROCESSOR MATCHES "x86_64|AMD64|amd64")
    set(EBPF_ARCH x86_64)
    set(EBPF_MUSL_TARGET x86_64-unknown-linux-musl)
elseif(CMAKE_SYSTEM_PROCESSOR MATCHES "aarch64|arm64")
    set(EBPF_ARCH aarch64)
    set(EBPF_MUSL_TARGET aarch64-unknown-linux-musl)
elseif(CMAKE_SYSTEM_PROCESSOR MATCHES "riscv64")
    set(EBPF_ARCH riscv64)
    set(EBPF_MUSL_TARGET riscv64gc-unknown-linux-musl)
elseif(CMAKE_SYSTEM_PROCESSOR MATCHES "loongarch64")
    set(EBPF_ARCH loongarch64)
    set(EBPF_MUSL_TARGET loongarch64-unknown-linux-musl)
else()
    set(EBPF_ARCH x86_64)
    set(EBPF_MUSL_TARGET x86_64-unknown-linux-musl)
endif()

message(STATUS "profile: building eBPF program via cargo xtask (arch=${EBPF_ARCH})")
# Clear the guest cross-build pkg-config redirection before shelling out to
# cargo: building the host `xtask` binary pulls in host crates whose build
# scripts run pkg-config, and the guest sysroot has no host libraries.
execute_process(
    COMMAND "${CMAKE_COMMAND}" -E env
            --unset=PKG_CONFIG_LIBDIR
            --unset=PKG_CONFIG_SYSROOT_DIR
            --unset=PKG_CONFIG_PATH
            cargo xtask starry user-ebpf build --program profile --arch ${EBPF_ARCH}
    WORKING_DIRECTORY "${WORKSPACE_ROOT}"
    RESULT_VARIABLE EBPF_BUILD_RESULT)
if(NOT EBPF_BUILD_RESULT EQUAL 0)
    message(FATAL_ERROR
        "cargo xtask starry user-ebpf build --program profile failed (exit ${EBPF_BUILD_RESULT})")
endif()

install(PROGRAMS
    "${WORKSPACE_ROOT}/os/StarryOS/user/ebpf/profile/target/${EBPF_MUSL_TARGET}/release/profile"
    DESTINATION usr/bin)
install(PROGRAMS profile.sh DESTINATION usr/bin)
