# StarryOS Syscall 与 SMP（S0-6）

## 已落地：`-smp 2` QEMU 模板

- **`test-suit/starryos/qemu-riscv64-smp2.toml`**：在默认 `qemu-riscv64.toml` 基础上增加 **`-smp` `2`**。
- **`cargo xtask starry test qemu --qemu-config test-suit/starryos/qemu-riscv64-smp2.toml`**：使用该模板生成临时测试配置（仍配合 **`--test-disk-image`** 等参数）。
- 便捷脚本：**`test-suit/starryos/scripts/run-starry-probe-qemu-smp2.sh <probe>`**（等价于单核版 **`run-starry-probe-qemu.sh`** + SMP TOML）。

日常命令摘要见 **`docs/starryos-probes-daily.md`**。

## 把「已有机理」用满：全 contract 的 SMP2 + guest oracle

对 **每个手写 contract**（见 **`list-contract-probes.sh`**），建议在 **双核 QEMU** 下再跑一遍 **`starryos-test`**，并用 **`verify-guest-log-oracle.sh`** 核对串口中的 **`CASE …`** 与 **`expected/*.line`**（与 Linux oracle 对齐）。

**一键矩阵**（耗时与探针数量成正比，默认日志在 **`$TMPDIR/starry-smp2-matrix/`**）：

- 若 **`target/riscv64gc-unknown-none-elf/rootfs-riscv64.img`** 不存在，脚本会先执行 **`cargo xtask starry rootfs --arch riscv64`**（首次可能下载，需网络）。
- 强制刷新基准盘：**`STARRY_REFRESH_ROOTFS=1`**。离线且已放好镜像：**`SKIP_STARRY_ROOTFS_FETCH=1`**（缺盘则失败，不跑 cargo）。

```sh
test-suit/starryos/scripts/run-smp2-guest-matrix.sh
```

单探针调试：

```sh
test-suit/starryos/scripts/run-smp2-guest-matrix.sh write_stdout
```

## 建议用法

- 对 **errno / 零长度 IO / EFAULT** 等确定性探针，在单核 + **`verify-oracle-all`** 通过后，再跑 **SMP2 矩阵** 确认无回归。
- **`futex` / `ppoll`** 等多核语义或竞态相关项：勿单独依赖固定 `expected/*.line`；需单独设计用例与匹配策略。

## 与矩阵的关系

在 **`docs/starryos-syscall-compat-matrix.yaml`** 文件头注释已指向 **SMP2 + guest** 一键矩阵；同步原语类待专用矩阵后再填 `parity`。
