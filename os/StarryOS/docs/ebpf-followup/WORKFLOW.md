# Workflow: 完善 tgoskits eBPF 子系统 (tracepoint / perf-ringbuf / demo)

> 面向后续 AI 协作者或本人后续 session 的工作流文档.
> 基线: `feat/starry-ebpf-userspace` @ `2f2533968`
> (PR-A eBPF runtime + PR-D 用户态程序的并集).
> 配套审计: [tracepoint-audit.md](tracepoint-audit.md),
> [perf-ringbuf-audit.md](perf-ringbuf-audit.md);
> 配套设计: [demo-stack-design.md](demo-stack-design.md).
> 工作日志: [journal.md](journal.md).
>
> 本工作流借鉴 `~/Downloads/busybox-fix-workflow.md` 的"先复现 →
> 给出可证伪 claim → 修复 → 反 fallback 自检"五步.

---

## 1. 背景与三任务范围

### 1.1 目标
让本仓库 (`rcore-os/tgoskits`, `os/StarryOS/`) 的 eBPF 子系统从"端到端能编译"
跨到"端到端能跑通可观察 demo". 不接 BTF, 不接 PMU.

### 1.2 三个任务 (互相松耦合, 可独立 PR)

| ID | 内容 | 主要改动面 | 依赖 |
|---|---|---|---|
| **T1: tracepoint 完善** | 补 P0 缺的 tracepoint hook (`sched:sched_switch`, `syscalls:sys_clone`, 关键 `sys_enter_*`); `trace_pipe` fd poll; `tracing_on` 全局开关 | `kernel/src/tracepoint/`, `os/arceos/modules/axtask/` (crate_interface hook), `kernel/src/syscall/` 中相关 entry | [tracepoint-audit.md §3 P0/P1] |
| **T2: perf / ring buffer 完善** | 接通 `mmap(perf_fd, ...)` → `BpfPerfEvent::do_mmap`; 实现 `Drop` 时 `frame_dealloc`; sample_type 校验; (可选) RINGBUF map mmap | `kernel/src/perf/{mod,bpf}.rs`, `kernel/src/file/mod.rs`, `kernel/src/syscall/mm/mmap.rs` | [perf-ringbuf-audit.md §3] |
| **T3: 稳定 demo 栈** | D1 syscall_count, D3 profile_kprobe, D2 sched_trace 三个 demo + 测试脚本; rootfs 安装路径 | `user/ebpf/`, `test-suit/starryos/ebpf/`, `scripts/axbuild/src/starry/` | T1 + T2 |

依赖图:
```
T1 ───┐
       ├──► T3 (D2 sched_trace 同时需要 T1 + T2; D1/D3 不需要)
T2 ───┘
```

### 1.3 触发判定 (什么样的 PR 走本工作流)

满足下列任一:
- 修改 `os/StarryOS/kernel/src/{tracepoint,perf,ebpf,kprobe}/`
- 修改 `os/arceos/modules/axtask/src/run_queue.rs` 中 switch / resched 路径并涉及 tracing hook
- 新增 `user/ebpf/` 下的 BPF demo crate
- 新增 `test-suit/starryos/ebpf/` 下的测试脚本
- 改动 `bpf(2)` / `perf_event_open(2)` syscall 的 dispatch 逻辑

否则按 `CONTRIBUTING.md` / `WORKFLOW_LINUX_SEMANTICS.md` / `WORKFLOW_EBPF_LKM_MIGRATION.md` 走.

---

## 2. 构建与测试环境

### 2.1 必须用 docker (host macOS 不可)

理由: macOS 无 `riscv64-linux-musl-cc` / `aarch64-linux-musl-cc`, host build 不可能过. 本工作流统一走 `docker.cnb.cool/starry-os/arceos-build:latest`.

辅助脚本 (在 `busybox_fix` 历史中存在过, 但本分支未跟踪): 在工作开始时把它写回 `os/StarryOS/.docker-run.sh` (`.gitignore` 已忽略 dot-prefix), 内容:

```bash
#!/usr/bin/env bash
set -euo pipefail
IMAGE=${STARRY_BUILD_IMAGE:-docker.cnb.cool/starry-os/arceos-build:latest}
LOCAL_IMAGE=${STARRY_BUILD_IMAGE_LOCAL:-starryos-dev-local:latest}
HOST_CACHE=${STARRY_DOCKER_CACHE:-$HOME/.cache/cargo-docker}
WORKSPACE_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
if ! docker image inspect "$LOCAL_IMAGE" >/dev/null 2>&1; then
    docker build --platform linux/amd64 -t "$LOCAL_IMAGE" -<<EOF >&2
FROM $IMAGE
RUN apt-get update && apt-get install -y --no-install-recommends libudev-dev pkg-config e2fsprogs && rm -rf /var/lib/apt/lists/*
EOF
fi
IMAGE="$LOCAL_IMAGE"
mkdir -p "$HOST_CACHE"/{registry,git,target-tgoskits,rustup}
if [ -z "$(ls -A "$HOST_CACHE/rustup" 2>/dev/null)" ]; then
    docker run --rm --platform linux/amd64 -v "$HOST_CACHE/rustup":/tmp/host-rustup "$IMAGE" bash -c 'cp -a /root/.rustup/. /tmp/host-rustup/' >&2 || true
fi
TTY=()
[ -t 0 ] && [ -t 1 ] && TTY=(-it)
exec docker run --rm ${TTY[@]+"${TTY[@]}"} --platform linux/amd64 \
    -v "$WORKSPACE_ROOT":/workspace \
    -v "$HOST_CACHE/registry":/root/.cargo/registry \
    -v "$HOST_CACHE/git":/root/.cargo/git \
    -v "$HOST_CACHE/rustup":/root/.rustup \
    -v "$HOST_CACHE/target-tgoskits":/cargo-target \
    -e CARGO_TARGET_DIR=/cargo-target \
    -e CARGO_HTTP_MULTIPLEXING=false \
    -w /workspace \
    "$IMAGE" "$@"
```

注意 `-w /workspace` (不是 `/workspace/os/StarryOS`), 因为 `cargo xtask` 要从仓库根调用.

### 2.2 常用命令

```bash
# 0. 一次性: 触发镜像 + 工具链 prime (3–8 min)
./os/StarryOS/.docker-run.sh bash -c 'rustc --version && cargo --version'

# 1. 编译 starry-kernel (最快验证编译错误, x86_64 通常 1m)
./os/StarryOS/.docker-run.sh cargo xtask starry build --arch x86_64

# 2. 跑 qemu (基线)
./os/StarryOS/.docker-run.sh cargo xtask starry rootfs --arch x86_64
./os/StarryOS/.docker-run.sh cargo xtask starry qemu --arch x86_64

# 3. fmt + clippy (按 AGENTS.md)
./os/StarryOS/.docker-run.sh cargo fmt --all -- --check
./os/StarryOS/.docker-run.sh cargo xtask clippy --package starry-kernel

# 4. user-space eBPF 程序构建 (PR-D xtask 子命令)
./os/StarryOS/.docker-run.sh cargo xtask starry user-ebpf list
./os/StarryOS/.docker-run.sh cargo xtask starry user-ebpf build --program syscall_ebpf --arch x86_64

# 5. eBPF demo 测试 (假设新建 group)
./os/StarryOS/.docker-run.sh cargo xtask starry test qemu --arch x86_64 -c ebpf
```

### 2.3 已知环境陷阱 (与 busybox 工作流共享)

| 现象 | 原因 | 处理 |
|---|---|---|
| `pkg-config exited with status code 1` | 镜像缺 libudev-dev | docker-run.sh 头部已 layer 上去 |
| `failed to read /etc/apk/repositories` | rootfs alpine 镜像源问题 | rootfs cmd 跑过一次成功后 cache 起 |
| `printf-compat 0.3 fails to build` | `core::ffi::VaList` API 在 nightly-2026-04-27 改名 | 仓库根已有 `[patch.crates-io] printf-compat = "0.4"` (commit `87f081516`) |
| `lwprintf-rs build.rs: gcc -print-sysroot fails` | 镜像 cross-toolchain 路径 | 仅影响 LKM (PR-B), 与本工作流无关 |

---

## 3. 仓库结构要点 (按本工作流的视角)

```
os/StarryOS/
├── kernel/src/
│   ├── ebpf/
│   │   ├── mod.rs       # sys_bpf dispatch
│   │   ├── map.rs       # BpfMap (FileLike) + PollSetWrapper
│   │   ├── prog.rs      # BpfProg (FileLike) + load_prog
│   │   └── transform.rs # EbpfKernelAuxiliary + PerCpuImpl + perf_event_output trampoline
│   ├── perf/
│   │   ├── mod.rs       # PerfEvent (FileLike) + perf_event_open dispatch + PERF_FILE table
│   │   ├── bpf.rs       # BpfPerfEventWrapper + OwnedEbpfVm  ← T2 主战场
│   │   ├── kprobe.rs    # ProbePerfEvent + KprobePerfCallBack
│   │   ├── tracepoint.rs
│   │   ├── raw_tracepoint.rs
│   │   └── uprobe.rs    # 全 Unsupported
│   ├── tracepoint/
│   │   ├── mod.rs       # TRACE_STATE + tracepoint_init + init_tracing_dir  ← T1 主战场
│   │   ├── control.rs   # EventEnableObj / EventFilterObj
│   │   ├── trace.rs     # TraceFile / TraceCmdLineFile
│   │   └── trace_pipe.rs # TracePipeFile (blocking)
│   ├── kprobe.rs        # PR-805 kprobe wrapper
│   ├── kallsyms.rs      # PR-805 nm-at-build-time
│   ├── entry.rs         # init: kallsyms → tracepoint → ebpf → perf
│   ├── lib.rs           # mod declarations
│   └── syscall/
│       ├── fs/{ctl,fd_ops}.rs # 已有 2 个 define_event_trace!
│       └── mod.rs       # bpf / perf_event_open 分派
├── user/ebpf/
│   ├── README.md
│   ├── async_test/      # smoke, 无 aya
│   ├── kret/            # kretprobe + aya_log
│   ├── mytrace/         # tracepoint + aya_log
│   ├── rawtp/           # raw tp (sys_clone 当前不可 attach)
│   ├── syscall_ebpf/    # kprobe + HashMap   ← T3 D1 基础
│   ├── upb/, upb2/      # uprobe, 内核侧 Unsupported
│   └── .cargo/config.toml
├── docs/ebpf-followup/  # 本工作流 + 三个审计 + journal
└── starryos/build.rs    # kallsyms nm 抽取

os/arceos/modules/axtask/src/run_queue.rs:559  # switch_to, T1 sched hook 注入点

scripts/axbuild/src/starry/
├── kmod.rs          # PR-B kmod xtask
├── user_ebpf.rs     # PR-D user-ebpf xtask
└── ...              # build / qemu / rootfs / test / clippy 等
```

---

## 4. 每个任务的标准 8 步流程

> **核心原则** (与 busybox 工作流相同):
> > 不允许"先把测试改通过、再回头编故事".
> > 必须先复现现状, 给出可证伪的失败原因 claim, 再决定改内核 / 改测试.
> > 反向自检 (§5.6) 任一项不过 → 回 §4.2, 不要绕过.

### 4.1 准备
1. 切分支 (一个 PR 一个 task, 不要叠):
   ```bash
   git fetch origin
   git checkout -b feat/ebpf-tp-<short> origin/feat/starry-ebpf-userspace
   ```
2. 读 audit 文档对应章节, 在 `docs/ebpf-followup/journal.md` 顶部追加一段:
   ```markdown
   ## YYYY-MM-DD — T<n> 开工: <one-line summary>
   - author: <name>
   - base: feat/starry-ebpf-userspace @ <sha>
   - 目标残缺项: TP-P0-1 (sched_switch hook)
   - 预期完成判据: §4 demo-stack-design D2 验证脚本
   ```

### 4.2 复现现状 (强制, 不允许跳过)

**对每一项要改的残缺项**, 在改代码之前先证明它真的是残缺. 写法:

```bash
# 例: TP-P0-1 (sched_switch 缺失) 的复现命令
./os/StarryOS/.docker-run.sh cargo xtask starry qemu --arch x86_64
# 在 qemu 内:
ls /sys/kernel/debug/tracing/events/sched/ 2>&1
# 期望看到 "No such file or directory" 或空目录 → 即"sched 子系统下无任何事件"
```

把观察归到 A/B/C/D 四类之一:
- **A. 内核 panic** — 必须改内核.
- **B. 业务异常退出** (`ENOSYS` / `EINVAL` 等) — syscall / pseudofs / kbpf-basic API 缺.
- **C. 静默成功** (脚本看不到错误, 但用户态拿不到任何数据) — 多数是测试设计问题, 但**必须证据级证明**.
- **D. 其实通了** — issue / audit 过期了, 直接补测试.

证据形式 (≥ 1 种, 可复现):
- 容器内某 syscall 的真实 rc + stdout/stderr.
- BPF prog 在内核内的 `error!` 日志 (perf/bpf.rs:97 即有这类日志).
- 内核 `dmesg` 等价 (qemu serial console).

### 4.3 给出可证伪 claim

写一句**带具体 syscall / 文件 / 行号**的失败原因, 并写明如何反向验证. 例子:

> claim: `bpf(BPF_RAW_TRACEPOINT_OPEN, name="sched_switch")` 在
> `os/StarryOS/kernel/src/perf/raw_tracepoint.rs:115` 调
> `find_ext_tracepoint_by_name("sched_switch")` 返 `None`, 因为本仓库
> 内没有任何 `ktracepoint::define_event_trace!(sched_switch, ...)`.
> 反向验证: 临时加一行 `define_event_trace!(sched_switch, …)`, 不接 hook,
> 仅看 `find_*` 是否能返 Some(_). 通过 → 修复方向确认是"加 hook";
> 仍 None → 说明问题在 ktracepoint 0.6 注册路径, 不在缺定义.

claim 不能含 "可能"/"应该"/"感觉" — 必须被新观察推翻或确认.

### 4.4 修复 (照 claim 决定路径)

**改内核** (T1 / T2 多数情况):
- AGENTS.md 硬规: 不允许 `#[allow]` 压 clippy, 不要写非必要注释, 不要写"伪 stub" (例如 `if not_supported { return 0 }`).
- 注释规则: 默认无注释; 写 WHY 不写 WHAT; 不要 reference "PR-A" / 当前 task 这种短命信息.
- **不允许把 `Unsupported` 路径默默改成 `Ok(())`**. 要么真实做事, 要么真实返错.

**改测试** (T3 验证脚本):
- 模板见 demo-stack-design.md §4.
- **不允许**降低验证强度 (`grep -q invoke` 改成 `[ -n "$_t" ]`).
- **不允许**绕路 (用 `dmesg` 代替 ringbuf, 用 `-h` 代替真实路径).
- 加强必须能反向证伪 claim: 未来内核如果回归到 claim 描述的状态, 测试要挂.

### 4.5 跑测试

按改动面分级:

| 改动面 | 必跑 |
|---|---|
| 仅改 `kernel/src/tracepoint/` 或 `perf/` | (a) fmt-check; (b) build x86_64; (c) qemu boot 不 panic; (d) 对应 demo 脚本 |
| 跨 crate (axtask + starry-kernel) | (a) + 全 4 架构 build + (d); 还要单独跑 `cargo xtask clippy --package starry-kernel --package axtask` |
| user/ebpf 新 crate | (a) `cargo metadata` per workspace + `cargo xtask starry user-ebpf build --program <new> --arch x86_64` + qemu 内跑测试脚本 |

```bash
./os/StarryOS/.docker-run.sh cargo fmt --all -- --check
./os/StarryOS/.docker-run.sh cargo xtask clippy --package starry-kernel
./os/StarryOS/.docker-run.sh cargo xtask starry build --arch x86_64
./os/StarryOS/.docker-run.sh cargo xtask starry build --arch riscv64
./os/StarryOS/.docker-run.sh cargo xtask starry build --arch aarch64
# loongarch64 受 PR-A 的 patch 限制, 单独验证
./os/StarryOS/.docker-run.sh cargo xtask starry test qemu --arch x86_64 -c ebpf 2>&1 | tee /tmp/ebpf-test.log
```

### 4.6 反向自检 (§5.6 模板)

提 PR **之前**逐条自答:

1. **这次"通过"是不是绕开了真正的代码路径?**
   - `dmesg | grep` 代替 ringbuf? `bpf_trace_printk` 代替 `bpf_perf_event_output`? 把 `Unsupported` 改成 `Ok(())`?
   - 答"是" → 必须在 PR body 明示"故意不覆盖 X, 理由 Y", 否则回 §4.4.
2. **是不是依赖了仍然有 bug 的子系统?**
   - 例如声称 sched_trace 工作, 但 ringbuf 仍不通 — 说明本 PR 没覆盖真实 P0 链.
3. **Linux 语义对齐?**
   - 在 host Linux + 同样 aya/libbpf API 跑一次, 与 qemu 输出形态对比 (行数、字段结构).
4. **新引入的 hook 有没有被覆盖到?**
   - 必须能写一条 sh 命令, 今天能触发本 PR 新增的 Rust 行.

PR body 要把这 4 条的答案写出来.

### 4.7 提交

按 user memory: **不要加 `Co-Authored-By` trailer**.

```bash
./os/StarryOS/.docker-run.sh cargo fmt --all -- --check
git add <精确路径>     # 不要 git add -A
git commit -m "$(cat <<'EOF'
feat(starry-kernel): add sched:sched_switch tracepoint hook

Audit: docs/ebpf-followup/tracepoint-audit.md TP-P0-1.
Claim: bpf(BPF_RAW_TRACEPOINT_OPEN, "sched_switch") 当前因 lookup 失败
返 EINVAL; 在 axtask 的 switch_to 上加 crate_interface hook + starry
侧 define_event_trace! 后, find_ext_tracepoint_by_name 命中.
反向自检: ringbuf 仍未通时 attach 成功 + trace_pipe 有记录, 但 user
程序在 ringbuf 修通前不会有输出; 与 PERF-P0-1 协同后才形成完整 demo.
EOF
)"
git push origin feat/ebpf-tp-<short>:feat/ebpf-tp-<short>   # 显式 refspec
```

### 4.8 PR description (中文 body, 英文 title)

按 AGENTS.md.

```markdown
type(scope): content

Audit refs:
- docs/ebpf-followup/tracepoint-audit.md §3 TP-P0-1
- docs/ebpf-followup/demo-stack-design.md §3.1

## 背景
<one paragraph>

## Claim (§4.3)
<可证伪 claim>

## 改动
1. <文件>: <动作>
2. ...

## 反向自检 (§4.6)
- 绕路: <是/否 + 证据>
- 依赖未通子系统: <是/否 + 影响范围>
- Linux 语义对齐: <reference 命令 + 行数对比>
- 新增代码可被触发: <一条命令>

## 验证
- [x] cargo fmt --all -- --check (clean)
- [x] cargo xtask clippy --package starry-kernel (clean)
- [x] cargo xtask starry build --arch {x86_64,riscv64,aarch64} (ok)
- [ ] cargo xtask starry build --arch loongarch64 (blocked on <link>)
- [x] qemu x86_64 + sched_trace demo: <output 行数>
- [x] qemu riscv64 同上

## 关联
- 依赖: PR #<id> (PERF-P0 ringbuf mmap), PR #<id> (PR-D user/ebpf)
- 阻塞: PR #<id> (D2 demo PR)
```

---

## 5. 反 fallback 黑名单 (一眼自检)

任意 PR 触发以下任一 → reviewer 直接驳回:

- ❌ 把 `Err(AxError::Unsupported)` 改成 `Ok(())` 让测试过.
- ❌ BPF prog 内只用 `aya_log_ebpf::info!` 当成"看见了" (ringbuf 未通时这条永远静默).
- ❌ 测试只断言 `[ -n "$_t" ]` 或 `grep -q invoke` 这类弱条件.
- ❌ `dmesg | grep sched_switch` 代替 ringbuf 验证.
- ❌ 把 tracepoint hook 加到错的位置 (例如 timer tick 代替 sched switch) 让记录数变大.
- ❌ PR body 缺 §4.6 反向自检.
- ❌ 用 `#[allow(clippy::...)]` 压 warning.
- ❌ 一个 PR 同时改 tracepoint + perf-ringbuf + demo 三件套.
- ❌ commit 加 `Co-Authored-By: Claude ...` (违反 user memory).
- ❌ 引入 `[patch.crates-io]` 指 personal fork (违反 WORKFLOW_EBPF_LKM_MIGRATION §4.1).
- ❌ 不在 `journal.md` 留记录就开 PR.

---

## 6. 验证清单 (pre-push)

每个 PR 推之前在 PR body "## 验证" 节贴最新输出.

### 6.1 格式
```
cargo fmt --all -- --check     ← docker
git diff --check
```

### 6.2 受影响 crate clippy
```
cargo xtask clippy --package starry-kernel    ← docker, 11 features
# 若改了 axtask, 再加:
cargo xtask clippy --package axtask
```

### 6.3 4 架构 build
```
for a in x86_64 riscv64 aarch64 loongarch64; do
    cargo xtask starry build --arch $a
done
```
loongarch64 在 PR-A 后仍可能挂 (printf-compat / lwprintf-rs 历史问题), 显式标 blocked.

### 6.4 sync-lint
```
cargo xtask sync-lint --since origin/dev
```

eBPF / perf / kprobe 大量原子操作, sync-lint 容易踩.

### 6.5 demo 烟测
```
cargo xtask starry rootfs --arch x86_64
cargo xtask starry test qemu --arch x86_64 -c ebpf
# 期望: PASS: N FAIL: 0
```

### 6.6 PR body ↔ 代码一致性
最后一次 push 后**重读 body 全文**:
- 每个声称的行为 → 找到对应代码位置.
- 每个验证结果 → 重新跑一遍贴最新输出.
- 早期 TODO / 已知问题段 → 删除或更新.

---

## 7. 进度勾选

每完成一项把对应行改为 `[x]` 并填上 PR 链接. test-only / docs-only PR 必须在 8.2 登记 follow-up.

### 7.1 主线

#### T1 — tracepoint
- [ ] **TP-P0-1**: `sched:sched_switch` raw tracepoint hook (axtask switch_to + starry kernel/tracepoint/sched.rs + crate_interface)
- [ ] **TP-P0-2**: `sched:sched_process_fork` / `_process_exit` hook
- [ ] **TP-P0-3**: `syscalls:sys_clone` raw tracepoint
- [ ] **TP-P0-4**: 批量补 `sys_enter_{read,write,execve,clone,exit_group}`
- [ ] **TP-P1-1**: `trace_pipe` fd-level poll
- [ ] **TP-P1-2**: `tracing_on` 全局开关
- [ ] **TP-P1-3**: `events/<sys>/enable` 组开关
- [ ] **TP-P1-4**: filter DSL 对齐审计

#### T2 — perf / ring buffer
- [ ] **PERF-P0-1/2/3**: PerfEvent::device_mmap → BpfPerfEvent::do_mmap → Drop dealloc
- [ ] **PERF-P1-1**: BPF_MAP_TYPE_RINGBUF map mmap
- [ ] **PERF-P1-2**: sample_type release-build 校验
- [ ] **PERF-P1-3**: read(perf_fd) counter (即使常 0)
- [ ] **PERF-P1-4**: poll 在 mmap 之前 EAGAIN 自检

#### T3 — demo
- [ ] **D1 syscall_count**: aya_log 注释 + 用户态读 map + 验证脚本 (依赖现状即可)
- [ ] **D3 profile_kprobe**: 新 user crate + 验证脚本 (依赖现状)
- [ ] **D2 sched_trace**: 新 user crate + 验证脚本 (依赖 TP-P0-1 + PERF-P0-1)

### 7.2 follow-up (test-only / 弱断言的欠账)

- (待补) 若 D1 因 aya_log 路径未通而把 BPF 内 log 注释掉, 这是临时方案. PERF-P0-1 接通后必须把 log 恢复, 并验证 user 侧能看到.

### 7.3 最终交付

T1 / T2 / T3 全过后, 在工作日志 (journal.md) 写一段"daemonize / fs-mutation / kprobe / tracepoint / ringbuf 在 StarryOS 上的可用矩阵" (借鉴 busybox §8.2 的回访矩阵), 让后续 reviewer 一眼看到边界.

---

## 8. 快速命令清单

```bash
# 0. docker prime
./os/StarryOS/.docker-run.sh bash -c 'rustc --version'

# 1. 编译内核
./os/StarryOS/.docker-run.sh cargo xtask starry build --arch x86_64

# 2. rootfs + qemu
./os/StarryOS/.docker-run.sh cargo xtask starry rootfs --arch x86_64
./os/StarryOS/.docker-run.sh cargo xtask starry qemu --arch x86_64

# 3. user-space eBPF 构建
./os/StarryOS/.docker-run.sh cargo xtask starry user-ebpf build --program syscall_ebpf --arch x86_64

# 4. fmt + clippy
./os/StarryOS/.docker-run.sh cargo fmt --all -- --check
./os/StarryOS/.docker-run.sh cargo xtask clippy --package starry-kernel

# 5. demo 测试 (T3 落地后)
./os/StarryOS/.docker-run.sh cargo xtask starry test qemu --arch x86_64 -c ebpf

# 6. 推 PR (按 §4.7 模板)
git push origin <branch>:<branch>
gh pr create -R rcore-os/tgoskits -B dev -H LorenzLorentz:<branch> \
    -t "feat(starry-kernel): ..." -F /tmp/pr-body.md
```

---

## 9. 参考链接

- 配套审计: [tracepoint-audit.md](tracepoint-audit.md), [perf-ringbuf-audit.md](perf-ringbuf-audit.md)
- 配套设计: [demo-stack-design.md](demo-stack-design.md)
- 工作日志: [journal.md](journal.md)
- 上一阶段流程: `os/StarryOS/docs/WORKFLOW_EBPF_LKM_MIGRATION.md` (PR-A/B/C/D 迁移)
- 风格参考: `~/Downloads/busybox-fix-workflow.md` (反 fallback 文化)
- 顶层规范: `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING.md`
