---
sidebar_position: 3
sidebar_label: "Axvisor on QEMU"
title: "Axvisor 在 QEMU 上启动"
---

# Axvisor 在 QEMU 上启动

Axvisor 的启动不只涉及编译结果，还依赖 Guest 镜像、板级配置和 VM 配置是否准备完整。因此它的快速上手应当按平台分开理解。

## 前置条件

- 已完成基础环境准备
- 可在仓库根目录使用 `cargo xtask` / `cargo axvisor`
- 本地已安装 QEMU

相关文档：[环境与平台](../introduction/hardware) | [QEMU 部署](../manual/deploy/qemu)

## 推荐平台：aarch64

这是当前最完整、最适合作为第一条 Axvisor QEMU 路径的配置。

```bash
# 1. 生成默认配置
cargo axvisor defconfig qemu-aarch64

# 2. 准备 Guest 镜像和 VM 配置
(cd os/axvisor && ./scripts/setup_qemu.sh arceos)

# 3. 启动 QEMU
cargo axvisor qemu --config os/axvisor/.build.toml
```

### 为什么推荐 aarch64

- 现有文档与示例更完整
- `setup_qemu.sh` 路径更成熟
- 更适合作为 Guest 配置和板级配置的入门样例

## 可选平台：x86_64

当前仓库中保留了 `x86_64` 的代码和平台实现，但它更适合平台适配或专项验证，而不是第一条上手路径。

```bash
# 建议先阅读构建系统和平台说明，再尝试 x86_64 路径
cargo axvisor build
```

### 何时关注 x86_64

- 正在做 `x86_64` 平台适配
- 需要核对某个 crate 的跨架构条件编译行为
- 准备补齐或完善 `x86_64` 的验证链路

## 下一步

- [Axvisor 开发指南](../design/systems/axvisor-guide)
- [Axvisor 内部机制](../design/architecture/axvisor-internals)
- [Guest 配置体系](../design/guest-config/config-overview)
