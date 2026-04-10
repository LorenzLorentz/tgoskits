---
sidebar_position: 3
sidebar_label: "运行与回归"
---

# 运行与回归

将测试链路理解为三层会更清楚：

| 层级 | 内容 | 命令 |
|------|------|------|
| 单 crate / host 测试 | 白名单 std 测试 | `cargo xtask test` |
| 单系统启动验证 | 最小 QEMU 路径 | 各系统 `qemu` 命令 |
| 统一回归入口 | 全量检查 | `cargo xtask clippy` |

## 提交前必跑命令组

```bash
cargo xtask test                    # Host std 测试
cargo xtask clippy                  # 静态检查
cargo xtask arceos test qemu --target riscv64
cargo xtask starry test qemu --target riscv64
cargo xtask axvisor test qemu --target aarch64
```
