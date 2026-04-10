---
sidebar_position: 1
sidebar_label: "ArceOS on QEMU"
title: "ArceOS 在 QEMU 上启动"
---

# ArceOS 在 QEMU 上启动

本文聚焦如何在 QEMU 环境中跑通 ArceOS 的最短路径，并给出当前仓库里更常用的平台选择。

## 前置条件

- 已完成基础环境准备
- 可在仓库根目录使用 `cargo xtask`
- 本地已安装 QEMU

相关文档：[环境与平台](../introduction/hardware) | [构建系统说明](../design/reference/build-system)

## 推荐平台：riscv64

这是当前仓库里最适合第一次验证 ArceOS 的路径。

```bash
cargo xtask arceos qemu --package ax-helloworld --target riscv64gc-unknown-none-elf
```

如果一切正常，QEMU 中会输出 `Hello, world!`。

### 适用场景

- 第一次确认工具链和工作区入口是否正常
- 调试最小示例应用
- 验证共享组件改动是否影响最短系统路径

## 可选平台：aarch64

如果你正在关注 AArch64 平台，也可以直接使用对应 target 运行：

```bash
cargo xtask arceos qemu --package ax-helloworld --target aarch64-unknown-none-softfloat
```

### 适用场景

- 平台适配集中在 AArch64
- 需要和 Axvisor 常用平台保持一致
- 准备继续调试板级或设备相关逻辑

## 常用变体

```bash
# 带网络或文件系统能力的应用，建议直接选择对应 package
cargo xtask arceos qemu --package ax-httpserver --target riscv64gc-unknown-none-elf

# 只做构建，不启动 QEMU
cargo xtask arceos build --package ax-helloworld --target riscv64gc-unknown-none-elf
```

## 下一步

- [ArceOS 开发指南](../design/systems/arceos-guide)
- [架构与组件层次](../design/architecture/arch)
- [组件开发指南](../design/reference/components)
