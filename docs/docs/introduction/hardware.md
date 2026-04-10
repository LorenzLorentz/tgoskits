---
sidebar_position: 2
sidebar_label: "环境与平台"
---

# 环境与平台

TGOSKits 不是单一系统仓库，因此"支持什么硬件"需要从开发宿主环境、默认验证路径和板级支持三个维度来看。

## 开发宿主环境

当前最稳妥的开发宿主环境是 **Linux**（推荐 Ubuntu 20.04+）。基础工具链：

| 类别 | 工具 | 说明 |
|------|------|------|
| 语言 | Rust toolchain + 交叉 target | 通过 `rustup` 安装 |
| 模拟器 | QEMU (system-arm, system-riscv64, system-x86) | 系统级验证的主要环境 |
| 辅助 | `cargo-binutils`, `ostool` | 二进制分析、镜像操作 |

完整安装步骤见：[快速开始指南](/docs/design/reference/quick-start) 第 2 节

## 默认验证路径

如果目标是最快确认环境可用，建议优先使用以下三条路径：

| 系统 | 推荐命令 | 说明 |
|------|---------|------|
| **ArceOS** | `cargo xtask arceos qemu --package ax-helloworld --arch riscv64` | 最短成功路径，无需额外准备 |
| **StarryOS** | `cargo xtask starry qemu --arch riscv64` | 首次运行需要准备 rootfs（可自动补齐） |
| **Axvisor** | `cargo xtask axvisor qemu --arch aarch64` | 运行前需执行 `setup_qemu.sh` 准备 Guest 镜像 |

## 架构与板级状态

| 架构 | ArceOS / StarryOS | Axvisor | 备注 |
|------|-------------------|---------|------|
| **riscv64** | 主要验证架构 | 有配置占位 | 日常快速验证首选 |
| **aarch64** | 支持 | **主要开发架构** | Axvisor 推荐路径，含 QEMU 和多板级 |
| **x86_64** | 有代码和占位 | stub 实现 | 非当前首选体验路径 |
| **loongarch64** | 支持 | - | 实验性 |

### Axvisor 板级支持

- `qemu-aarch64`（默认推荐）
- `qemu-riscv64`
- `orangepi-5-plus`（RK3588）
- `phytiumpi`（飞腾 E2000）
- `roc-rk3568-pc`

## 选择建议

- **想快速跑起来**：优先走 QEMU + riscv64（ArceOS/StarryOS）或 aarch64（Axvisor）
- **想改系统共性能力**：先选最小 QEMU 消费者验证
- **想做板级适配**：同时准备代码、配置和镜像路径，关注 `configs/board/*.toml` 与 `configs/vms/*.toml`

## 相关文档

- [统一快速上手](/docs/quickstart/arceos-qemu)
- [QEMU 部署](../manual/deploy/qemu)
- [Arch / Target 映射](../design/build/arch)
