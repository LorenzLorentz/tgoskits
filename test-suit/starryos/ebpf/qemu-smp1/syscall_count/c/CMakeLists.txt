cmake_minimum_required(VERSION 3.20)
project(ebpf-syscall-count C)
set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

# Deterministic getpid(2) driver: the verification script runs this to make a
# fixed number of sys_getpid calls so the kprobe HashMap count is assertable.
add_executable(getpid_spin src/getpid_spin.c)
target_compile_options(getpid_spin PRIVATE -Wall -Wextra -Werror)
install(TARGETS getpid_spin RUNTIME DESTINATION usr/bin)

# The syscall_count demo reuses the syscall_ebpf aya program (its own nested
# Cargo workspace under os/StarryOS/user/ebpf/syscall_ebpf/). It is *generated*
# by `cargo xtask starry user-ebpf build`, not checked in, so build it here at
# configure time and install the resulting musl ELF + its launcher.
get_filename_component(WORKSPACE_ROOT
    "${CMAKE_CURRENT_SOURCE_DIR}/../../../../../.." ABSOLUTE)

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

message(STATUS "syscall_count: building eBPF program via cargo xtask (arch=${EBPF_ARCH})")
execute_process(
    COMMAND "${CMAKE_COMMAND}" -E env
            --unset=PKG_CONFIG_LIBDIR
            --unset=PKG_CONFIG_SYSROOT_DIR
            --unset=PKG_CONFIG_PATH
            cargo xtask starry user-ebpf build --program syscall_ebpf --arch ${EBPF_ARCH}
    WORKING_DIRECTORY "${WORKSPACE_ROOT}"
    RESULT_VARIABLE EBPF_BUILD_RESULT)
if(NOT EBPF_BUILD_RESULT EQUAL 0)
    message(FATAL_ERROR
        "cargo xtask starry user-ebpf build --program syscall_ebpf failed (exit ${EBPF_BUILD_RESULT})")
endif()

install(PROGRAMS
    "${WORKSPACE_ROOT}/os/StarryOS/user/ebpf/syscall_ebpf/target/${EBPF_MUSL_TARGET}/release/syscall_ebpf"
    DESTINATION usr/bin)
install(PROGRAMS syscall_count.sh DESTINATION usr/bin)
