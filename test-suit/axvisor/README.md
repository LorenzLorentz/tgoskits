# AxVisor 差分测试集

## 如何测试

命令入口：

```bash
cd tgoskits
cargo axvisor test diff ...
```

常见用法：

```bash
# 运行单个测例；如果不显式指定 --arch，则默认使用 aarch64。
cargo axvisor test diff \
  --case test-suit/axvisor/cpu-state/aarch64-currentel

# 运行一个测例集。
cargo axvisor test diff \
  --suite test-suit/axvisor/suites/smoke.toml

# 显式指定架构。
cargo axvisor test diff \
  --arch aarch64 \
  --suite test-suit/axvisor/suites/smoke.toml

# 控制是否将 guest 串口输出同时打印到终端。
cargo axvisor test diff \
  --case test-suit/axvisor/cpu-state/aarch64-currentel \
  --guest-log false
```

说明：

- `--case` 与 `--suite` 互斥，且必须二选一。
- `--arch` 当前默认值为 `aarch64`。
- `--guest-log` 为可选参数。
- 单测例运行时，`guest_log` 默认值为 `true`。
- 测例集运行时，`guest_log` 默认值为 `false`。
- 对于所选架构，每个测例都需要提供 `case.toml`、guest 构建配置、VM 模板以及基准 QEMU 配置。
- runner 的输出会写入 `os/axvisor/tmp/diff/<run-id>/`，其中包括原始日志、解析后的 guest 结果以及 `summary.json`。

## 已实现测例列表

当前已有测例集：

- `test-suit/axvisor/suites/smoke.toml`

状态说明：

- `PASS`：测试验证通过。
- `FAIL`：测试验证失败。
- `-`：该测例尚未在该架构下实现，或该架构不支持此测例。

| 测例 ID | 目录 | 目标接口 | 比较模式 | x86_64 | aarch64 | riscv64 |
| --- | --- | --- | --- | --- | --- | --- |
| `cpu.aarch64.currentel` | `cpu-state/aarch64-currentel` | `CurrentEL` | `strong` | - | PASS | - |
| `cpu.aarch64.mpidr` | `cpu-state/aarch64-mpidr` | `MPIDR_EL1` | `strong` | - | PASS | - |
