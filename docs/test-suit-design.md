# test-suit 测试用例设计文档

## 1. 顶层目录结构

`test-suit/` 是所有 OS 测试用例的统一入口，按操作系统划分为独立目录。每个目录由对应的 `cargo xtask <os>` 子命令负责发现、构建和运行测试。

```
test-suit/
├── starryos/        # StarryOS 测试用例
├── axvisor/         # (暂无 test-suit 下的用例，板级测试配置在 os/axvisor/configs/ 中)
└── arceos/          # ArceOS 测试用例
```

---

## 2. StarryOS

StarryOS 测试分为**普通测试**（`normal/`）和**压力测试**（`stress/`）两组，每组下每个子目录代表一个独立的测试用例。用例可以无源码（仅平台配置文件），也可以包含 C 或 Rust 源码（分别放在 `c/` 或 `rust/` 子目录中）。目录名即测试用例名，由 xtask 自动扫描发现。

```
test-suit/starryos/
├── normal/                               # 普通测试用例
│   ├── smoke/                            # 无源码用例：仅平台配置文件
│   │   ├── qemu-aarch64.toml
│   │   ├── qemu-riscv64.toml
│   │   ├── qemu-loongarch64.toml
│   │   ├── qemu-x86_64.toml
│   │   └── board-orangepi-5-plus.toml
│   ├── my_c_test/                        # 含 C 源码用例
│   │   ├── c/                            # C 源码目录
│   │   │   └── main.c
│   │   ├── qemu-x86_64.toml             # 平台配置与 c/ 同级
│   │   └── qemu-aarch64.toml
│   └── my_rust_test/                     # 含 Rust 源码用例
│       ├── rust/                         # Rust 源码目录
│       │   ├── Cargo.toml
│       │   └── src/
│       │       └── main.rs
│       ├── qemu-x86_64.toml             # 平台配置与 rust/ 同级
│       └── qemu-riscv64.toml
└── stress/                               # 压力测试用例
    └── stress-ng-0/
        ├── qemu-aarch64.toml
        ├── qemu-riscv64.toml
        ├── qemu-loongarch64.toml
        └── qemu-x86_64.toml
```

### 2.1 测试分组

| 分组 | 路径 | 说明 | 运行命令 |
|------|------|------|----------|
| normal | `test-suit/starryos/normal/` | 普通功能测试 | `cargo xtask starry test qemu --target <arch>` |
| stress | `test-suit/starryos/stress/` | 压力/负载测试 | `cargo xtask starry test qemu --target <arch> --stress` |

### 2.2 C 测试用例

#### 2.2.1 目录结构

```
test-suit/starryos/normal/
└── my_c_test/                        # 含 C 源码用例
    ├── c/                            # C 源码目录
    │   ├── CMakeLists.txt            # CMake 构建脚本（必需）
    │   ├── main.c                    # C 入口文件
    │   └── ...                       # 其他 C 源文件或头文件
    ├── qemu-x86_64.toml             # QEMU 测试配置与 c/ 同级
    ├── qemu-aarch64.toml
    └── board-orangepi-5-plus.toml   # 板级测试配置（可选）
```

#### 2.2.2 测例源码

| 文件/目录 | 必需 | 说明 |
|-----------|------|------|
| `c/` | 是（C 测试） | C 源码目录，包含所有 `.c`、`.h` 文件和 CMake 脚本 |
| `c/CMakeLists.txt` | 是 | CMake 构建脚本，定义目标架构的交叉编译规则 |
| `c/main.c` | 是 | C 入口文件，包含 `main()` 函数 |
| `c/*.c` | 是 | 其他 C 源文件 |

#### 2.2.3 QEMU 测试配置

`qemu-{arch}.toml` QEMU 测试配置，放在用例根目录下（与 `c/` 同级），定义 QEMU 启动参数、Shell 交互行为以及测试结果判定规则。

**示例** — `normal/smoke/qemu-x86_64.toml`：

```toml
args = [
    "-nographic",
    "-device",
    "virtio-blk-pci,drive=disk0",
    "-drive",
    "id=disk0,if=none,format=raw,file=${workspace}/target/x86_64-unknown-none/rootfs-x86_64.img",
    "-device",
    "virtio-net-pci,netdev=net0",
    "-netdev",
    "user,id=net0",
]
uefi = false
to_bin = false
shell_prefix = "root@starry:"
shell_init_cmd = "pwd && echo 'All tests passed!'"
success_regex = ["(?m)^All tests passed!\\s*$"]
fail_regex = ['(?i)\bpanic(?:ked)?\b']
timeout = 15
```

**示例** — `stress/stress-ng-0/qemu-x86_64.toml`：

```toml
args = [
    "-nographic",
    "-device",
    "virtio-blk-pci,drive=disk0",
    "-drive",
    "id=disk0,if=none,format=raw,file=${workspace}/target/x86_64-unknown-none/rootfs-x86_64.img",
    "-device",
    "virtio-net-pci,netdev=net0",
    "-netdev",
    "user,id=net0",
]
uefi = false
to_bin = false
shell_prefix = "starry:~#"
shell_init_cmd = '''
apk update && \
apk add stress-ng && \
stress-ng --cpu 8 --timeout 10s && \
stress-ng --sigsegv 8 --sigsegv-ops 1000    && \
pwd && ls -al && echo 'All tests passed!'
'''
success_regex = ["(?m)^All tests passed!\\s*$"]
fail_regex = ['(?i)\bpanic(?:ked)?\b', '(m)^stress-ng: info: .*failed: [1-9]\d*\s*$']
timeout = 50
```

**字段说明：**

| 字段 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `args` | `[String]` | 是 | — | QEMU 命令行参数，支持 `${workspace}` 占位符 |
| `uefi` | `bool` | 否 | `false` | 是否使用 UEFI 启动 |
| `to_bin` | `bool` | 否 | `false` | 是否将 ELF 转换为 raw binary |
| `shell_prefix` | `String` | 否 | — | Shell 提示符前缀，用于检测 shell 就绪 |
| `shell_init_cmd` | `String` | 否 | — | Shell 就绪后执行的命令，支持多行 `'''` |
| `success_regex` | `[String]` | 是 | — | 成功判定正则列表，任一匹配即判定成功 |
| `fail_regex` | `[String]` | 否 | `[]` | 失败判定正则列表，任一匹配即判定失败 |
| `timeout` | `u64` | 否 | — | 超时秒数 |

#### 2.2.4 板级测试配置

`board-{board_name}.toml` 板级测试配置，放在用例根目录下（与 `c/` 同级），用于物理开发板上的测试，通过串口交互判定结果。与 QEMU 配置相比没有 `args`、`uefi`、`to_bin` 字段，但增加了 `board_type` 标识板型。

**示例** — `normal/smoke/board-orangepi-5-plus.toml`：

```toml
board_type = "OrangePi-5-Plus"
shell_prefix = "root@starry:/root #"
shell_init_cmd = "pwd && echo 'test pass'"
success_regex = ["(?m)^test pass\\s*$"]
fail_regex = []
timeout = 300
```

**字段说明：**

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `board_type` | `String` | 是 | 板型标识，需对应 `os/StarryOS/configs/board/{board_name}.toml` |
| `shell_prefix` | `String` | 是 | Shell 提示符前缀 |
| `shell_init_cmd` | `String` | 是 | Shell 就绪后执行的命令 |
| `success_regex` | `[String]` | 是 | 成功判定正则列表 |
| `fail_regex` | `[String]` | 否 | 失败判定正则列表 |
| `timeout` | `u64` | 是 | 超时秒数，物理板通常需要更长时间（如 300s） |

#### 2.2.5 执行 QEMU 测试

##### 2.2.5.1 命令行参数

```
cargo xtask starry test qemu --target <arch> [--stress] [--test-case <case>]
```

| 参数 | 说明 |
|------|------|
| `--target` / `-t` | 目标架构或完整 target triple（如 `aarch64`、`riscv64`、`x86_64`、`loongarch64`，或 `aarch64-unknown-none-softfloat`、`riscv64gc-unknown-none-elf`） |
| `--stress` | 运行 stress 组测试，缺省运行 normal 组 |
| `--test-case` / `-c` | 仅运行指定用例 |

##### 2.2.5.2 发现机制

xtask 扫描 `test-suit/starryos/{normal|stress}/` 下所有子目录，检查其中是否存在 `qemu-{arch}.toml` 文件。若存在，则将该子目录名作为用例名，并将该 TOML 文件作为 QEMU 运行配置加载。

```
发现路径: test-suit/starryos/<group>/<case-name>/qemu-<arch>.toml
```

例如，对于架构 `aarch64`：
- `test-suit/starryos/normal/smoke/qemu-aarch64.toml` → 用例名 `smoke`
- `test-suit/starryos/stress/stress-ng-0/qemu-aarch64.toml` → 用例名 `stress-ng-0`

##### 2.2.5.3 构建

xtask 定位用例目录中的 `c/CMakeLists.txt`，配置交叉编译工具链（根据目标架构选择对应的 sysroot 和 compiler），然后执行 `cmake --build` 编译 C 程序。

CMake 脚本需要满足以下要求：

- 使用 `cmake_minimum_required()` 指定最低版本
- 通过 `project()` 声明项目名称和语言
- 定义可执行目标，将所有 `.c` 源文件加入编译
- 使用交叉编译工具链（xtask 会通过 `CMAKE_TOOLCHAIN_FILE` 传入）

**示例** — `c/CMakeLists.txt`：

```cmake
cmake_minimum_required(VERSION 3.20)
project(my_c_test C)

add_executable(my_c_test main.c)
```

源码要求：
- 入口函数为标准 `int main(void)` 或 `int main(int argc, char *argv[])`
- 可引用标准 C 库头文件（`<stdio.h>`、`<stdlib.h>`、`<string.h>` 等）
- 可引用 POSIX 头文件（`<pthread.h>`、`<unistd.h>`、`<sys/socket.h>` 等）
- 所有 `.c` 和 `.h` 文件放在 `c/` 目录下

##### 2.2.5.4 rootfs 准备与注入

rootfs 镜像是 StarryOS 测试的基础运行环境，提供完整的 Linux 用户态文件系统（含 shell、apk 包管理器等）。xtask 在测试运行前自动下载 rootfs，并将编译产物注入其中。

**1. 下载 rootfs**

xtask 根据目标架构选择对应的 rootfs 镜像，检查本地是否已存在。若不存在，自动从远程仓库下载压缩包并解压：

```
下载地址: https://github.com/Starry-OS/rootfs/releases/download/20260214/rootfs-{arch}.img.xz
存放路径: {workspace}/target/{target}/rootfs-{arch}.img
```

各架构对应的 rootfs 文件：

| 架构 | rootfs 文件 | 存放路径 |
|------|------------|----------|
| `x86_64` | `rootfs-x86_64.img` | `target/x86_64-unknown-none/` |
| `aarch64` | `rootfs-aarch64.img` | `target/aarch64-unknown-none-softfloat/` |
| `riscv64` | `rootfs-riscv64.img` | `target/riscv64gc-unknown-none-elf/` |
| `loongarch64` | `rootfs-loongarch64.img` | `target/loongarch64-unknown-none-softfloat/` |

下载流程：
1. 检查 `{target}/rootfs-{arch}.img` 是否存在
2. 若不存在，下载 `rootfs-{arch}.img.xz` 到 `{target}/` 目录
3. 解压 `.xz` 文件得到 `.img` 镜像
4. 删除 `.xz` 压缩包

也可通过命令手动下载：

```
cargo xtask starry rootfs --arch <arch>
```

**2. 注入编译产物**

对于含 C/Rust 源码的测试用例，xtask 将编译产物注入到对应架构的 rootfs 镜像中，使其在系统启动后可直接通过 shell 执行。

**3. 配置 QEMU 磁盘参数**

xtask 自动将 rootfs 镜像路径注入到 QEMU 的 `-drive` 参数中，替换 TOML 配置里的 `${workspace}` 占位符。如果配置中没有声明磁盘设备参数，xtask 会自动添加默认的 `virtio-blk-pci` 和 `virtio-net-pci` 设备。

##### 2.2.5.5 执行测例

1. 加载 `qemu-{arch}.toml` 配置，构造 QEMU 启动命令
2. 启动 QEMU，开始捕获串口输出
3. 若设置了 `shell_prefix`，等待该前缀出现后发送 `shell_init_cmd`
4. 每收到新输出时，先检查 `fail_regex`（任一匹配 → 失败），再检查 `success_regex`（任一匹配 → 成功）
5. 超时未判定 → 失败

#### 2.2.6 执行开发板测试

##### 2.2.6.1 命令行参数

```
cargo xtask starry test board [--test-group <group>] [--board-test-config <path>] [--board-type <type>] [--server <addr>] [--port <port>]
```

| 参数 | 说明 |
|------|------|
| `--test-group` / `-t` | 指定测试组名（如 `smoke-orangepi-5-plus`） |
| `--board-test-config` | 指定板级测试配置文件路径；当前要求与 `--test-group` 一起使用 |
| `--board-type` / `-b` | 指定板型（如 `OrangePi-5-Plus`） |
| `--server` | 串口服务器地址 |
| `--port` | 串口服务器端口 |

##### 2.2.6.2 发现机制

xtask 扫描 `test-suit/starryos/normal/` 下所有子目录，检查其中是否存在 `board-{board_name}.toml` 文件。若存在，进一步验证对应的构建配置 `os/StarryOS/configs/board/{board_name}.toml` 是否存在，从中提取架构和 target 信息。

```
测试配置:   test-suit/starryos/normal/<case>/board-<board_name>.toml
构建配置:   os/StarryOS/configs/board/<board_name>.toml
```

##### 2.2.6.3 构建

与 QEMU 测试相同，xtask 使用 CMake 交叉编译 C 程序。

##### 2.2.6.4 rootfs 准备与注入

与 QEMU 测试相同，详见 [2.2.5.4 rootfs 准备与注入](#2254-rootfs-准备与注入)。

##### 2.2.6.5 执行测例

1. 加载 `board-{board_name}.toml` 配置，通过串口服务器连接物理板
2. 等待 `shell_prefix` 出现后发送 `shell_init_cmd`
3. 检查 `fail_regex` 和 `success_regex` 判定结果
4. 超时未判定 → 失败

#### 2.2.7 新增测试用例

**新增普通测试：**

1. 在 `test-suit/starryos/normal/` 下创建用例目录（如 `my_c_feature/`）
2. 创建 `c/` 子目录，放入 `CMakeLists.txt` 和 `.c` 源文件
3. 为每个支持的架构创建 `qemu-{arch}.toml`
4. 如需在物理板上测试，创建 `board-{board_name}.toml`

**新增压力测试：**

1. 在 `test-suit/starryos/stress/` 下创建用例目录
2. 创建 `c/` 子目录，放入 `CMakeLists.txt` 和 `.c` 源文件
3. 为每个支持的架构创建 `qemu-{arch}.toml`
4. 压力测试通常使用更长的 `timeout` 和更复杂的 `shell_init_cmd`

### 2.3 Rust 测试用例

#### 2.3.1 目录结构

```
test-suit/starryos/normal/
└── my_rust_test/                     # 含 Rust 源码用例
    ├── rust/                         # Rust 源码目录（标准 Cargo 项目）
    │   ├── Cargo.toml                # 包定义
    │   └── src/
    │       └── main.rs               # 入口源码
    ├── qemu-x86_64.toml             # QEMU 测试配置与 rust/ 同级
    └── qemu-riscv64.toml
```

#### 2.3.2 测例源码

| 文件/目录 | 必需 | 说明 |
|-----------|------|------|
| `rust/` | 是（Rust 测试） | Rust 源码目录，标准 Cargo 项目结构 |
| `rust/Cargo.toml` | 是 | 包定义文件 |
| `rust/src/main.rs` | 是 | 入口源码文件 |
| `rust/src/*.rs` | 是 | 其他源码文件 |

源码要求：
- 入口函数为标准 `fn main()`
- 可使用 `#![no_std]` 和 `#![no_main]` 配合自定义入口（视 OS 支持而定）
- `Cargo.toml` 中声明所需的依赖和 features

#### 2.3.3 QEMU 测试配置

配置文件格式与 C 测试用例相同，详见 [2.2.3 QEMU 测试配置](#223-qemu-测试配置)。

#### 2.3.4 板级测试配置

配置文件格式与 C 测试用例相同，详见 [2.2.4 板级测试配置](#224-板级测试配置)。

#### 2.3.5 执行 QEMU 测试

##### 2.3.5.1 命令行参数

与 C 测试用例相同：`cargo xtask starry test qemu --target <arch> [--stress] [--test-case <case>]`

详见 [2.2 C 测试用例 → 执行 QEMU 测试](#22-c-测试用例)。

##### 2.3.5.2 发现机制

与 C 测试用例相同，xtask 扫描 `test-suit/starryos/{normal|stress}/` 下所有子目录中的 `qemu-{arch}.toml`。

##### 2.3.5.3 构建

xtask 定位用例目录中的 `rust/Cargo.toml`，根据目标架构配置交叉编译目标，执行 `cargo build` 编译 Rust 程序。

##### 2.3.5.4 rootfs 准备与注入

与 C 测试用例相同，详见 [2.2.5.4 rootfs 准备与注入](#2254-rootfs-准备与注入)。

##### 2.3.5.5 执行测例

与 C 测试用例相同，详见 [2.2.5.5 执行测例](#2255-执行测例)。

#### 2.3.6 执行开发板测试

##### 2.3.6.1 命令行参数

与 C 测试用例相同：`cargo xtask starry test board [--test-group <group>] [--board-type <type>] [--server <addr>] [--port <port>]`

详见 [2.2 C 测试用例 → 执行开发板测试](#22-c-测试用例)。

##### 2.3.6.2 发现机制

与 C 测试用例相同，xtask 扫描 `test-suit/starryos/normal/` 下所有子目录中的 `board-{board_name}.toml`。

##### 2.3.6.3 构建

与 QEMU 测试相同，xtask 使用 `cargo build` 交叉编译 Rust 程序。

##### 2.3.6.4 rootfs 准备与注入

与 QEMU 测试相同，详见 [2.2.5.4 rootfs 准备与注入](#2254-rootfs-准备与注入)。

##### 2.3.6.5 执行测例

与 C 测试用例相同，详见 [2.2.6.5 执行测例](#2265-执行测例)。

#### 2.3.7 新增测试用例

**新增普通测试：**

1. 在 `test-suit/starryos/normal/` 下创建用例目录
2. 创建 `rust/` 子目录，放入 `Cargo.toml` 和 `src/main.rs`
3. 为每个支持的架构创建 `qemu-{arch}.toml`
4. 如需在物理板上测试，创建 `board-{board_name}.toml`

**新增压力测试：**

1. 在 `test-suit/starryos/stress/` 下创建用例目录
2. 创建 `rust/` 子目录，放入 `Cargo.toml` 和 `src/main.rs`
3. 为每个支持的架构创建 `qemu-{arch}.toml`

### 2.4 无源码用例

无源码用例不需要编写 C 或 Rust 代码，而是利用 StarryOS 文件系统中包管理器（如 `apk add`）直接安装已有的可执行程序，然后通过 Shell 交互驱动测试。此类用例只需提供平台配置文件（`qemu-{arch}.toml` 或 `board-{board_name}.toml`），测试逻辑完全由 `shell_init_cmd` 中的命令序列定义。

典型的无源码用例是 `stress-ng-0`：系统启动后，`shell_init_cmd` 中通过 `apk add stress-ng` 安装压力测试工具，再执行对应的测试命令。

#### 2.4.1 目录结构

```
test-suit/starryos/
├── normal/
│   └── smoke/                            # 无源码用例：仅平台配置文件
│       ├── qemu-aarch64.toml
│       ├── qemu-riscv64.toml
│       ├── qemu-loongarch64.toml
│       ├── qemu-x86_64.toml
│       └── board-orangepi-5-plus.toml
└── stress/
    └── stress-ng-0/                      # 无源码用例：apk 安装后执行
        ├── qemu-aarch64.toml
        ├── qemu-riscv64.toml
        ├── qemu-loongarch64.toml
        └── qemu-x86_64.toml
```

#### 2.4.2 配置文件

配置文件格式与 C/Rust 测试用例相同，详见 [2.2.3 QEMU 测试配置](#223-qemu-测试配置) 和 [2.2.4 板级测试配置](#224-板级测试配置)。

关键区别在于：
- 目录中不包含 `c/` 或 `rust/` 子目录
- 测试逻辑完全由 `shell_init_cmd` 定义，通常包含安装和执行两个阶段

**示例** — `stress/stress-ng-0/qemu-x86_64.toml`：

```toml
shell_init_cmd = '''
apk update && \
apk add stress-ng && \
stress-ng --cpu 8 --timeout 10s && \
stress-ng --sigsegv 8 --sigsegv-ops 1000 && \
pwd && ls -al && echo 'All tests passed!'
'''
```

#### 2.4.3 执行流程

1. xtask 扫描发现用例目录中的 `qemu-{arch}.toml`
2. 由于没有 `c/` 或 `rust/` 子目录，跳过构建和 rootfs 注入步骤
3. 直接使用 StarryOS 预构建的 rootfs 镜像启动 QEMU
4. 等待 `shell_prefix` 出现后发送 `shell_init_cmd`（安装并运行测试程序）
5. 通过 `success_regex` / `fail_regex` 判定结果

#### 2.4.4 新增无源码用例

**新增普通测试：**

1. 在 `test-suit/starryos/normal/` 下创建用例目录（如 `my_smoke_test/`）
2. 为每个支持的架构创建 `qemu-{arch}.toml`，在 `shell_init_cmd` 中编写安装和测试命令
3. 如需在物理板上测试，创建 `board-{board_name}.toml`

**新增压力测试：**

1. 在 `test-suit/starryos/stress/` 下创建用例目录
2. 为每个支持的架构创建 `qemu-{arch}.toml`
3. 压力测试通常使用更长的 `timeout` 和更复杂的 `shell_init_cmd`

---

## 3. Axvisor

Axvisor 目前没有在 `test-suit/` 目录下放置用例配置文件。其测试基础设施通过**硬编码的板级测试组**定义，配置分布在 `os/axvisor/configs/` 中。

### 3.1 测试类型

| 类型 | 说明 | 运行命令 |
|------|------|----------|
| QEMU 测试 | 在 QEMU 中启动 hypervisor 并运行 Guest | `cargo xtask axvisor test qemu --target <arch>` |
| U-Boot 测试 | 通过 U-Boot 引导 hypervisor | `cargo xtask axvisor test uboot --board <board> --guest <guest>` |
| 板级测试 | 在物理开发板上运行 | `cargo xtask axvisor test board [--test-group <group>]` |

### 3.2 QEMU 测试

QEMU 测试的 Shell 交互配置是硬编码的，不从 TOML 文件读取：

| 架构 | Shell 前缀 | 初始化命令 | 成功判定 |
|------|-----------|-----------|----------|
| `aarch64` | `~ #` | `pwd && echo 'guest test pass!'` | `(?m)^guest test pass!\s*$` |
| `x86_64` | `>>` | `hello_world` | `Hello world from user mode program!` |

**失败判定正则**（所有架构通用）：
- `(?i)\bpanic(?:ked)?\b`
- `(?i)kernel panic`
- `(?i)login incorrect`
- `(?i)permission denied`

**命令行参数：**

```
cargo xtask axvisor test qemu --target <arch>
```

| 参数 | 说明 |
|------|------|
| `--target` | 目标架构（如 `aarch64`、`x86_64`） |

### 3.3 U-Boot 测试

U-Boot 测试通过硬编码的板型/客户机映射表定义：

| 板型 | 客户机 | 构建配置 | VM 配置 |
|------|--------|----------|---------|
| `orangepi-5-plus` | `linux` | `os/axvisor/configs/board/orangepi-5-plus.toml` | `os/axvisor/configs/vms/linux-aarch64-orangepi5p-smp1.toml` |
| `phytiumpi` | `linux` | `os/axvisor/configs/board/phytiumpi.toml` | `os/axvisor/configs/vms/linux-aarch64-e2000-smp1.toml` |
| `roc-rk3568-pc` | `linux` | `os/axvisor/configs/board/roc-rk3568-pc.toml` | `os/axvisor/configs/vms/linux-aarch64-rk3568-smp1.toml` |

**命令行参数：**

```
cargo xtask axvisor test uboot --board <board> --guest <guest>
```

| 参数 | 说明 |
|------|------|
| `--board` / `-b` | 板型名称 |
| `--guest` | 客户机类型 |
| `--uboot-config` | 自定义 U-Boot 配置文件路径 |

### 3.4 板级测试

板级测试通过硬编码的测试组定义，每组包含构建配置、VM 配置和板级测试配置：

| 测试组 | 构建配置 | VM 配置 | 板级测试配置 |
|--------|----------|---------|-------------|
| `phytiumpi-linux` | `os/axvisor/configs/board/phytiumpi.toml` | `os/axvisor/configs/vms/linux-aarch64-e2000-smp1.toml` | `os/axvisor/configs/board-test/phytiumpi-linux.toml` |
| `orangepi-5-plus-linux` | `os/axvisor/configs/board/orangepi-5-plus.toml` | `os/axvisor/configs/vms/linux-aarch64-orangepi5p-smp1.toml` | `os/axvisor/configs/board-test/orangepi-5-plus-linux.toml` |
| `roc-rk3568-pc-linux` | `os/axvisor/configs/board/roc-rk3568-pc.toml` | `os/axvisor/configs/vms/linux-aarch64-rk3568-smp1.toml` | `os/axvisor/configs/board-test/roc-rk3568-pc-linux.toml` |
| `rdk-s100-linux` | `os/axvisor/configs/board/rdk-s100.toml` | `os/axvisor/configs/vms/linux-aarch64-s100-smp1.toml` | `os/axvisor/configs/board-test/rdk-s100-linux.toml` |

**命令行参数：**

```
cargo xtask axvisor test board [--test-group <group>] [--board-type <type>] [--server <addr>] [--port <port>]
```

| 参数 | 说明 |
|------|------|
| `--test-group` / `-t` | 指定测试组名（如 `orangepi-5-plus-linux`） |
| `--board-type` / `-b` | 指定板型 |
| `--board-test-config` | 自定义板级测试配置路径 |
| `--server` | 串口服务器地址 |
| `--port` | 串口服务器端口 |

### 3.5 新增测试用例

目前 Axvisor 的测试配置是硬编码在 `scripts/axbuild/src/axvisor/` 中的。新增测试用例需要：

1. 在 `os/axvisor/configs/board/` 下准备构建配置
2. 在 `os/axvisor/configs/vms/` 下准备 VM 配置
3. 在 `os/axvisor/configs/board-test/` 下准备板级测试配置
4. 在 `scripts/axbuild/src/axvisor/` 中注册新的测试组

---

## 4. ArceOS

ArceOS 测试在 OS 级别按语言分为 `c/` 和 `rust/` 两个目录。每个测试用例占一个子目录，C 测试包含 `.c` 源文件及可选的构建辅助文件，Rust 测试则是标准的 Cargo 项目。两类测试均通过**硬编码列表**注册在 xtask 中，由 xtask 分别调度构建与 QEMU 运行。

```
test-suit/arceos/
├── c/                        # C 语言测试用例
│   ├── helloworld/           # 每个 C 测试用例一个目录
│   │   ├── main.c            # C 源码（必须存在至少一个 .c 文件）
│   │   ├── axbuild.mk        # 可选，指定 app-objs
│   │   ├── features.txt      # 可选，每行一个 feature（如 alloc, paging, net）
│   │   ├── test_cmd          # 可选，定义 test_one 调用序列
│   │   └── expect_info.out   # 可选，预期输出
│   ├── httpclient/
│   ├── memtest/
│   └── pthread/
│       ├── basic/
│       ├── parallel/
│       ├── pipe/
│       └── sleep/
└── rust/                     # Rust 测试用例
    ├── display/              # 每个 Rust 测试用例一个目录
    │   ├── Cargo.toml
    │   ├── src/
    │   │   └── ...
    │   ├── build-x86_64-unknown-none.toml       # 构建配置
    │   └── qemu-x86_64.toml                     # QEMU 运行配置
    ├── exception/
    ├── fs/
    │   └── shell/
    ├── memtest/
    ├── net/
    │   ├── echoserver/
    │   ├── httpclient/
    │   ├── httpserver/
    │   └── udpserver/
    └── task/
        ├── affinity/
        ├── ipi/
        ├── irq/
        ├── parallel/
        ├── priority/
        ├── sleep/
        ├── tls/
        ├── wait_queue/
        └── yield/
```

### 4.1 命令行参数

```
cargo xtask arceos test qemu --target <arch> [--package <pkg>] [--only-rust] [--only-c]
```

| 参数 | 说明 |
|------|------|
| `--target` | 目标架构或完整 target triple（如 `x86_64` 或 `x86_64-unknown-none`） |
| `--package` / `-p` | 仅运行指定的 Rust 测试包（可多次使用） |
| `--only-rust` | 仅运行 Rust 测试 |
| `--only-c` | 仅运行 C 测试 |

默认行为（不带筛选参数）时，先运行所有 Rust 测试，再运行所有 C 测试。

### 4.2 C 测试用例

C 测试通过硬编码目录列表 (`C_TEST_NAMES`) 发现，每个目录必须包含至少一个 `.c` 源文件。

#### 4.2.1 现有 C 测试

| 目录名 | 说明 |
|--------|------|
| `helloworld` | 基础 Hello World |
| `memtest` | 内存分配/释放测试 |
| `httpclient` | HTTP 客户端（需 `alloc`、`paging`、`net` feature） |
| `pthread/basic` | 线程创建、join、mutex 基础测试 |
| `pthread/parallel` | 多线程并行测试 |
| `pthread/pipe` | 管道通信测试 |
| `pthread/sleep` | 线程睡眠测试 |

#### 4.2.2 文件说明

| 文件 | 必需 | 说明 |
|------|------|------|
| `*.c` | 是 | C 源码，目录内必须存在至少一个 |
| `axbuild.mk` | 否 | 指定 `app-objs`，缺省默认取目录内所有 `.c` 编译 |
| `features.txt` | 否 | 每行一个 feature flag（如 `alloc`、`paging`、`net`），构建时传入 |
| `test_cmd` | 否 | 定义 `test_one "MAKE_VARS" "EXPECT_OUTPUT"` 调用序列，控制多组测试变体 |
| `expect_*.out` | 否 | 预期输出文件，用于与实际 QEMU 输出做文本对比 |

#### 4.2.3 `test_cmd` 格式

`test_cmd` 文件中每行定义一次测试调用：
```
test_one "MAKE_VARS" "expect_output_file"
```

- `MAKE_VARS`：传递给 `make` 的变量赋值（如 `LOG=info`、`SMP=4 LOG=info`）
- `expect_output_file`：可选，预期输出文件名（相对于当前测试目录）

**示例** — `helloworld/test_cmd`：

```bash
test_one "LOG=info" "expect_info.out"
test_one "SMP=4 LOG=info" "expect_info_smp4.out"
rm -f $APP/*.o
```

该文件定义了两组测试：单核 `LOG=info` 和四核 `SMP=4 LOG=info`，每组分别对比对应的预期输出。

#### 4.2.4 C 测试执行流程

1. 解析 `features.txt` → 提取 feature flags
2. 解析 `test_cmd` → 提取 `test_one` 调用序列
3. 对每组调用：
   - 运行 `make defconfig` 并传入 features
   - 运行 `make build` 构建镜像
   - 运行 `make justrun` 在 QEMU 中启动并捕获输出
   - 若指定了 `expect_output_file`，将实际输出与预期输出对比

### 4.3 Rust 测试用例

Rust 测试通过硬编码包列表 (`ARCEOS_TEST_PACKAGES`) 发现，每个包是一个标准 Cargo 项目。

#### 4.3.1 现有 Rust 测试

| 包名 | 分类 | 说明 |
|------|------|------|
| `arceos-memtest` | 内存 | 内存分配测试 |
| `arceos-exception` | 异常 | 异常处理测试 |
| `arceos-affinity` | 任务 | CPU 亲和性测试 |
| `arceos-ipi` | 任务 | 核间中断（IPI）测试 |
| `arceos-irq` | 任务 | 中断状态测试 |
| `arceos-parallel` | 任务 | 并行任务测试 |
| `arceos-priority` | 任务 | 任务优先级调度测试 |
| `arceos-sleep` | 任务 | 任务睡眠测试 |
| `arceos-tls` | 任务 | 线程本地存储测试 |
| `arceos-wait-queue` | 任务 | 等待队列测试 |
| `arceos-yield` | 任务 | 任务让出测试 |
| `arceos-fs-shell` | 文件系统 | 交互式 FS Shell 测试 |
| `arceos-net-echoserver` | 网络 | TCP Echo 服务器测试 |
| `arceos-net-httpclient` | 网络 | HTTP 客户端测试 |
| `arceos-net-httpserver` | 网络 | HTTP 服务器测试 |
| `arceos-net-udpserver` | 网络 | UDP 服务器测试 |
| `arceos-display` | 显示 | GPU/Framebuffer 显示测试 |

#### 4.3.2 文件说明

| 文件 | 必需 | 说明 |
|------|------|------|
| `Cargo.toml` | 是 | 包定义，通常依赖 `ax-std`（optional feature） |
| `src/main.rs` | 是 | 入口源码 |
| `build-{target}.toml` | 视情况 | 构建配置（features、log 级别、环境变量、CPU 数量） |
| `.axconfig.toml` | 否 | Axconfig 运行时配置（部分用例使用） |
| `qemu-{arch}.toml` | 视情况 | QEMU 运行配置 |

#### 4.3.3 Rust 测试执行流程

1. 从 `ARCEOS_TEST_PACKAGES` 中按 `--package` 参数过滤
2. 定位包目录：`test-suit/arceos/rust/<package-name>/`
3. 加载构建配置 `build-{target}.toml`
4. 执行 `cargo build --release --target <target> --features <features>`
5. 加载 QEMU 配置 `qemu-{arch}.toml`
6. 启动 QEMU 运行镜像，通过正则判定成功/失败

### 4.4 构建配置 (`build-{target}.toml`)

定义 ArceOS Rust 测试的构建参数。文件名中的 `target` 需与编译目标匹配。

**示例** — `task/affinity/build-x86_64-unknown-none.toml`：

```toml
features = ["ax-std"]
log = "Warn"
max_cpu_num = 4

[env]
AX_GW = "10.0.2.2"
AX_IP = "10.0.2.15"
```

**示例** — `net/httpclient/build-x86_64-unknown-none.toml`：

```toml
features = ["ax-std", "net"]
log = "Warn"
max_cpu_num = 4

[env]
AX_GW = "10.0.2.2"
AX_IP = "10.0.2.15"
```

**字段说明：**

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `features` | `[String]` | 否 | 启用的 Cargo features，通常包含 `"ax-std"` |
| `log` | `String` | 否 | 日志级别（如 `"Warn"`、`"Info"`） |
| `max_cpu_num` | `u32` | 否 | 最大 CPU 数量 |
| `[env]` | Table | 否 | 构建时环境变量，如 `AX_GW`、`AX_IP` |

### 4.5 QEMU 运行配置 (`qemu-{arch}.toml`)

与 StarryOS 格式相同。部分测试通过源码中的 `println!("All tests passed!")` 输出判定，无需 shell 交互。

**示例** — `task/affinity/qemu-x86_64.toml`（无 shell 交互）：

```toml
args = [
    "-machine", "q35",
    "-cpu", "max",
    "-m", "128M",
    "-smp", "4",
    "-nographic",
    "-serial", "mon:stdio",
]
uefi = false
to_bin = false
success_regex = ["All tests passed!"]
fail_regex = ['(?i)\bpanic(?:ked)?\b']
```

**示例** — `fs/shell/qemu-x86_64.toml`（有 shell 交互）：

```toml
args = [
    "-machine", "q35",
    "-cpu", "max",
    "-m", "128M",
    "-smp", "4",
    "-nographic",
    "-device", "virtio-blk-pci,drive=disk0",
    "-drive", "id=disk0,if=none,format=raw,file=${workspace}/test-suit/arceos/rust/fs/shell/disk.img",
    "-serial", "mon:stdio",
]
uefi = false
to_bin = false
shell_prefix = "arceos:"
shell_init_cmd = "pwd && echo 'FS shell tests passed!'"
success_regex = ["FS shell tests passed!"]
fail_regex = ["(?i)\\bpanic(?:ked)?\\b"]
timeout = 3
```

**示例** — `net/httpclient/qemu-x86_64.toml`（需要网络设备）：

```toml
args = [
    "-machine", "q35",
    "-cpu", "max",
    "-m", "128M",
    "-smp", "4",
    "-nographic",
    "-device", "virtio-net-pci,netdev=net0",
    "-netdev", "user,id=net0",
    "-serial", "mon:stdio",
]
uefi = false
to_bin = false
success_regex = ["HTTP client tests run OK!"]
fail_regex = ["(?i)\\bpanic(?:ked)?\\b"]
```

### 4.6 新增测试用例

#### 4.6.1 新增 Rust 测试

1. 在 `test-suit/arceos/rust/` 下创建包目录，包含 `Cargo.toml` 和 `src/main.rs`
2. `Cargo.toml` 中将 `ax-std` 设为 optional 依赖，并声明所需的 features
3. 为每个支持的架构创建 `qemu-{arch}.toml`
4. 如需自定义构建参数，创建 `build-{target}.toml`
5. 在 `scripts/axbuild/src/test_qemu.rs` 的 `ARCEOS_TEST_PACKAGES` 中注册包名

#### 4.6.2 新增 C 测试

1. 在 `test-suit/arceos/c/` 下创建目录，包含 `.c` 源文件
2. 可选添加 `features.txt`（每行一个 feature）
3. 可选添加 `test_cmd`（定义测试变体）
4. 可选添加 `axbuild.mk`（指定编译对象）
5. 在 `scripts/axbuild/src/arceos/mod.rs` 的 `C_TEST_NAMES` 中注册目录名

---

## 5. 配置文件格式汇总

测试用例通过 TOML 配置文件控制构建和运行行为。按用途分为三类：QEMU 运行配置、板级测试配置和构建配置。

### 5.1 QEMU 运行配置

**文件名格式**：`qemu-{arch}.toml`

**适用范围**：StarryOS、ArceOS

**完整字段参考：**

| 字段 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `args` | `[String]` | 是 | — | QEMU 命令行参数，支持 `${workspace}` 占位符 |
| `uefi` | `bool` | 否 | `false` | 是否使用 UEFI 启动 |
| `to_bin` | `bool` | 否 | `false` | 是否将 ELF 转换为 raw binary |
| `shell_prefix` | `String` | 否 | — | Shell 提示符前缀，用于检测 shell 就绪 |
| `shell_init_cmd` | `String` | 否 | — | Shell 就绪后执行的命令，支持多行 `'''` 语法 |
| `success_regex` | `[String]` | 是 | — | 成功判定正则列表，任一匹配即判定成功 |
| `fail_regex` | `[String]` | 否 | `[]` | 失败判定正则列表，任一匹配即判定失败 |
| `timeout` | `u64` | 否 | — | 超时秒数 |

**判定逻辑**：

1. QEMU 启动后开始捕获串口输出
2. 每收到新输出时，先检查 `fail_regex`：任一匹配 → 判定**失败**
3. 再检查 `success_regex`：任一匹配 → 判定**成功**
4. 若设置了 `shell_prefix`，先等待该前缀出现在输出中，然后发送 `shell_init_cmd`
5. 超时未判定 → 判定**失败**

### 5.2 板级测试配置

**文件名格式**：`board-{board_name}.toml`

**适用范围**：StarryOS

**完整字段参考：**

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `board_type` | `String` | 是 | 板型标识，需对应 `os/<OS>/configs/board/{board_name}.toml` |
| `shell_prefix` | `String` | 是 | Shell 提示符前缀 |
| `shell_init_cmd` | `String` | 是 | Shell 就绪后执行的命令 |
| `success_regex` | `[String]` | 是 | 成功判定正则列表 |
| `fail_regex` | `[String]` | 否 | 失败判定正则列表 |
| `timeout` | `u64` | 是 | 超时秒数 |

### 5.3 构建配置

**文件名格式**：`build-{target}.toml`

**适用范围**：ArceOS Rust 测试

**完整字段参考：**

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `features` | `[String]` | 否 | 启用的 Cargo features |
| `log` | `String` | 否 | 日志级别 |
| `max_cpu_num` | `u32` | 否 | 最大 CPU 数量 |
| `[env]` | Table | 否 | 构建时环境变量 |

---

## 6. 命名规范

统一目录和配置文件的命名规则，确保跨 OS 一致性。

### 6.1 目录命名

- 使用小写字母、数字、连字符和下划线
- 测试用例目录名应简短且有描述性：`smoke`、`stress-ng-0`、`helloworld`

### 6.2 配置文件命名

| 文件 | 格式 | 示例 |
|------|------|------|
| QEMU 配置 | `qemu-{arch}.toml` | `qemu-aarch64.toml`、`qemu-x86_64.toml` |
| 板级配置 | `board-{board_name}.toml` | `board-orangepi-5-plus.toml` |
| 构建配置 | `build-{target}.toml` | `build-x86_64-unknown-none.toml`、`build-aarch64-unknown-none-softfloat.toml` |

### 6.3 支持的架构

| 架构缩写 | 完整 Target | QEMU 参数 | 说明 |
|----------|-------------|-----------|------|
| `x86_64` | `x86_64-unknown-none` | `-machine q35 -cpu max` | x86_64 Q35 平台 |
| `aarch64` | `aarch64-unknown-none-softfloat` | `-cpu cortex-a53` | ARM Cortex-A53 |
| `riscv64` | `riscv64gc-unknown-none-elf` | `-cpu rv64` | RISC-V 64 位 |
| `loongarch64` | `loongarch64-unknown-none-softfloat` | `-machine virt -cpu la464` | LoongArch LA464 |
