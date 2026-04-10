---
title: "统一快速上手"
sidebar_label: "统一快速上手"
---

# 统一快速上手

从零开始在 QEMU 上跑通 TGOSKits 的三套核心系统。

## 前置条件

- Linux 开发环境（推荐 Ubuntu 20.04+）
- Rust toolchain 与交叉编译 target
- QEMU 系统模拟器

详细环境搭建步骤：[环境与平台](../../introduction/hardware) | [完整快速开始指南](../../reference/quick-start)

## 克隆仓库

```bash
git clone https://github.com/rcore-os/tgoskits.git
cd tgoskits
```

## 命令入口

| 位置 | 命令 | 用途 |
|------|------|------|
| 仓库根目录 | `cargo xtask ...` | 三套系统与统一测试的主入口 |
| 仓库根目录 | `cargo arceos ...` | `cargo xtask arceos ...` 的别名 |
| 仓库根目录 | `cargo starry ...` | `cargo xtask starry ...` 的别名 |
| 仓库根目录 | `cargo axvisor ...` | `cargo xtask axvisor ...` 的别名 |

## 最短成功路径

### ArceOS（约 1-2 分钟）

```bash
cargo xtask arceos qemu --package ax-helloworld --arch riscv64
```

ArceOS 是最短验证路径，无需额外准备即可运行。

### StarryOS（首次需准备 rootfs）

```bash
cargo xtask starry qemu --arch riscv64
```

`qemu` 命令在发现磁盘镜像缺失时会自动补齐 rootfs。

### Axvisor（需准备 Guest 镜像）

```bash
# 1. 准备 Guest 镜像和配置
cd os/axvisor && ./scripts/setup_qemu.sh arceos && cd ../..

# 2. 启动 Axvisor
cargo xtask axvisor qemu --arch aarch64
```

> **注意**：Axvisor 不能只看编译是否通过，还要确认 Guest 镜像、VM 配置和 rootfs 已对齐。
> 如需启动 Linux Guest，将 `setup_qemu.sh` 参数改为 `linux`。

## 三条路径对比

| 维度 | ArceOS | StarryOS | Axvisor |
|------|--------|----------|---------|
| **最短命令** | 1 步 | 1-2 步 | 2-3 步 |
| **额外准备** | 无 | rootfs（可自动） | Guest 镜像 + 配置 |
| **推荐架构** | riscv64 | riscv64 | aarch64 |
| **启动时间** | 最短 | 中等 | 较长 |

## 开发闭环

```bash
# Host / Std 测试
cargo xtask test

# ArceOS 回归
cargo xtask arceos test qemu --target riscv64

# StarryOS 回归
cargo xtask starry test qemu --target riscv64

# Axvisor 回归
cargo xtask axvisor test qemu --target aarch64

# 全 workspace 静态检查
cargo xtask clippy
```

## 下一步

- [环境与平台详情](../../introduction/hardware)
- [系统关系与架构](../../introduction/guest)
- [QEMU 部署详解](../../manual/deploy/qemu)
- [各系统开发指南](../../guides/arceos-guide)
