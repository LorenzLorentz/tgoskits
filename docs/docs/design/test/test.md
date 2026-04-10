---
sidebar_position: 1
sidebar_label: "验证策略"
---

# 验证策略

当前仓库最有效的验证方式是按**最近消费者优先**的方式逐步放大。

## 推荐顺序

```
最小消费者 -> 对应系统 test qemu -> cargo xtask test -> cargo xtask clippy
```

## 按改动位置选择

| 改动位置 | 先做什么 |
|----------|---------|
| `components/*` 基础 crate | `cargo test -p <crate>` 或最小 host 路径 |
| ArceOS 模块 | 最小 ArceOS `qemu` 路径 |
| StarryOS | rootfs + 最小 `qemu` 路径 |
| Axvisor | `setup_qemu.sh` + `cargo xtask axvisor qemu` |

详细说明：[组件开发指南](/docs/design/reference/components)
