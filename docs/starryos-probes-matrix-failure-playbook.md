# SMP2 / guest 矩阵失败 — 可行动项清单

当 **`run-smp2-guest-matrix.sh`** 或 CI **`starryos-probes-smp2-matrix`** 报错时，按下面顺序缩小范围。矩阵会在 **`$LOGDIR/MATRIX_FAILURES.md`**（默认 **`$TMPDIR/starry-smp2-matrix/`**）追加结构化摘要，便于贴进 issue。

## 1. 区分失败阶段

| 现象 | 含义 | 优先动作 |
|------|------|----------|
| `FAILED: QEMU starry test` | 内核/镜像/超时/启动未跑完探针 | 打开对应 **`smp2-<probe>.log`** 尾部，查 panic、timeout、`starryos-test` 退出码 |
| `FAILED: guest CASE vs oracle` | 探针已跑，串口 **`CASE …`** 与 **`expected/*.line`**（或 **`.cases`**）不一致 | 对比 Linux oracle 与 guest（见下） |

## 2. QEMU 阶段可行动项

1. 确认基准盘： **`ensure-starry-base-rootfs.sh`** 是否成功（网络、磁盘、**`cargo xtask starry rootfs`**）。
2. 单探针复现：  
   `test-suit/starryos/scripts/run-starry-probe-qemu-smp2.sh <probe>`  
   必要时改 **`--timeout`**（在 **`run-starry-probe-qemu-smp2.sh`** 末尾追加 xtask 参数）。
3. 注入镜像： **`prepare-rootfs-with-probe.sh <probe>`** 是否报 **`debugfs`** / 交叉编译错误。
4. 将 **`smp2-<probe>.log`**（或关键片段）附到 issue，并注明 **`-smp 2`** 与 **`qemu-riscv64-smp2.toml`**。

## 3. Oracle 不一致可行动项

1. **Linux 锚点**（user-mode）：  
   `CC=… test-suit/starryos/scripts/build-probes.sh`  
   `test-suit/starryos/scripts/run-diff-probes.sh verify-oracle <probe>`  
   确认 **`expected/`** 仍与 **`qemu-riscv64`** 一致。
2. **抽取 guest 首行或多行 `CASE`**：  
   `test-suit/starryos/scripts/extract-case-line.sh smp2-<probe>.log`  
   若使用 **`.cases`**： **`extract-case-lines.sh`**。
3. **判定**：
   - StarryOS **应** 与 Linux 一致 → **内核 bug**，开 issue / 修 **`impl_path`**（见 catalog）。
   - 经评审接受差异 → 在 **`docs/starryos-syscall-compat-matrix.yaml`** 将 **`parity`** 标为 **`divergent`**，**`notes`** 写原因；必要时增加 **guest 专用期望**（见 **`docs/starryos-syscall-testing-method.md`**）。
4. 多 **`CASE`** 场景：使用 **`expected/<probe>.cases`**（**排序后的集合**比较，与日志中 **`CASE`** 行顺序无关）：
   - 抽取：`test-suit/starryos/scripts/extract-case-lines.sh <log>`
   - 比对：`test-suit/starryos/scripts/diff-guest-cases.sh <probe> <log>` 或 **`verify-guest-log-oracle.sh`**（自动识别 **`.cases`**）
   - Linux oracle：`test-suit/starryos/scripts/run-diff-probes.sh verify-oracle <probe>`
   - **勿**与 **`.line`** 并存同一探针名。

## 4. 开 issue 时建议标题格式

`[starry-probe] SMP2 guest oracle mismatch: <probe_basename>`

正文附上：**探针名**、**失败阶段**、**want / got**（或 **`MATRIX_FAILURES.md`** 片段）、**相关 log 路径**、是否已复现 **`verify-oracle`**。
