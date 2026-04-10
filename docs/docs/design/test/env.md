---
sidebar_position: 2
sidebar_label: "测试环境"
---

# 测试环境

测试环境准备的关键不在"装得越多越好"，而在"为当前目标路径补齐依赖"。

## 通用依赖

- Rust toolchain（含交叉编译 target）
- QEMU 系统模拟器
- 基本构建工具链（gcc, make, cmake 等）

## 系统特定依赖

| 系统 | 额外依赖 |
|------|---------|
| **StarryOS** | rootfs 相关工具与镜像准备脚本 |
| **Axvisor** | Guest 镜像、VM 配置、rootfs、`setup_qemu.sh` |

完整环境安装命令：[快速开始指南](/docs/design/reference/quick-start)
