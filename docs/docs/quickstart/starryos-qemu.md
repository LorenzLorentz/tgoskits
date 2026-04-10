---
sidebar_position: 2
sidebar_label: "StarryOS on QEMU"
title: "StarryOS 在 QEMU 上启动"
---

# StarryOS 在 QEMU 上启动

StarryOS 的启动相比 ArceOS 多了一层 rootfs 准备，但在当前仓库里依然有较稳定的 QEMU 入口。

## 前置条件

- 已完成基础环境准备
- 可在仓库根目录使用 `cargo xtask`
- 本地已安装 QEMU

相关文档：[环境与平台](../introduction/hardware) | [构建系统说明](../design/reference/build-system)

## 推荐平台：riscv64

当前文档和示例最常围绕 riscv64 展开。

```bash
# 首次运行时，qemu 会在缺少 rootfs 时自动补准备
cargo xtask starry qemu --arch riscv64
```

如果你希望显式准备 rootfs，也可以先运行：

```bash
cargo xtask starry rootfs --arch riscv64
cargo xtask starry qemu --arch riscv64
```

## 可选平台：aarch64

StarryOS 的默认架构就是 aarch64，因此也适合在这个平台继续验证：

```bash
cargo xtask starry rootfs --arch aarch64
cargo xtask starry qemu --arch aarch64
```

## 何时选择哪个平台

| 平台 | 适合什么情况 |
| --- | --- |
| `riscv64` | 跟随当前高频文档示例、快速熟悉系统路径 |
| `aarch64` | 与其他系统路径对齐、继续看平台或部署相关问题 |

## 下一步

- [StarryOS 开发指南](../design/systems/starryos-guide)
- [测试与验证](../design/test)
- [QEMU 部署](../manual/deploy/qemu)
