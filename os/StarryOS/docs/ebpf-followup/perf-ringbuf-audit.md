# perf / ring buffer 支持审计

> 审计时点: 2026-05-23, 分支 `feat/starry-ebpf-userspace` @ `2f2533968`.
> 重点回答: `bpf_perf_event_output` / aya_log / libbpf-style perf buffer /
> `BPF_MAP_TYPE_RINGBUF` 这条链, 哪一段可用、哪一段断了.

## 1. 数据通路 (one diagram, end-to-end)

```
USER (aya/libbpf)
  ├─ perf_event_open(PERF_TYPE_SOFTWARE, BPF_PERF_EVENT) → fd (1 per cpu)
  ├─ mmap(fd, 1 + 2^N pages, PROT_READ|PROT_WRITE)  ←—— ★ 这里挂掉
  ├─ ioctl(fd, PERF_EVENT_IOC_SET_BPF, prog_fd)
  └─ ioctl(fd, PERF_EVENT_IOC_ENABLE)
                            │
KERNEL                      ▼
  PerfEvent (FileLike)  ──► BpfPerfEventWrapper
       │                          │
       │  ioctl SET_BPF: 把 prog 绑到本 event
       │  ioctl ENABLE: BpfPerfEvent::enable() → data.enabled=true
       │
  BPF prog 在某个 hook (kprobe/tp/raw tp) 触发
       │
       └─► helper: bpf_perf_event_output(ctx, &perf_map, BPF_F_CURRENT_CPU, data, len)
                ├─ kbpf_basic 走到 transform.rs::perf_event_output
                ├─ → crate::perf::perf_event_output (perf/mod.rs:182)
                ├─ → 在 PERF_FILE map 找 fd → Weak<PerfEvent>
                ├─ → downcast 到 BpfPerfEventWrapper
                └─ → BpfPerfEventWrapper::write_event(data)
                          │
                          ├─ if phys_addr.is_none() → return Ok(())  ← ★ 静默丢失
                          └─ else → BpfPerfEvent::write_event → RingPage::write_event
                                   → 写 perf_event_mmap_page.data_head 推进
                                   → poll_ready.wake()

USER 端 poll(fd, POLLIN) / read mmap'd ringbuf
  ├─ 期望从 mmap 区域读到 PerfSample 头 + payload
  └─ ★ mmap 区域不存在 / 不映射 → libbpf 一次也读不到
```

★ 标记是当前的 3 个具体阻塞点.

## 2. 现状 (按 "支持/部分支持/不支持")

### 2.1 已实现

| 维度 | 现状 | 证据 |
|---|---|---|
| `perf_event_open(2)` dispatcher | `kprobe / tracepoint / software_bpf / uprobe` 四种 type 分派 | `kernel/src/perf/mod.rs:136-165` |
| `PerfEvent` FileLike | 注册到 fd 表; weak ref 入 `PERF_FILE` 表给 helper 用 | 同上 :156-163 |
| `PERF_EVENT_IOC_{ENABLE,DISABLE,SET_BPF}` | 走 `PerfEventIoc::try_from(cmd)` | 同上 :113-129 |
| `BpfPerfEventWrapper` 封装 | 含 `inner: BpfPerfEvent`, `poll_ready: PollSet`, `phys_addr: Option<(PhysAddr, usize)>` | `perf/bpf.rs:34-66` |
| `bpf_perf_event_output` helper 调用链 | `transform.rs:228` impl → `perf/mod.rs:182` 处理 | 经过 PERF_FILE 表反向找回 event |
| `OwnedEbpfVm` (rbpf + prog 绑定 + helper 表注册) | `perf/bpf.rs:141-198` | OK |
| `kbpf_basic::map::stream::RingBufMap` (BPF_MAP_TYPE_RINGBUF) | crates.io 内已实现; `map::bpf_map_create` 在 `BPF_MAP_TYPE_RINGBUF` 分支构造它 | `kbpf-basic-0.5.5/src/map/mod.rs:303-306` |
| `PollSetWrapper` (`Arc<dyn PollWaker>`) | 用于 ringbuf reservation / queue push 时唤醒读者 | `ebpf/map.rs:87-122` |
| `perf_event_attr` 各 type 的解析 | 由 `kbpf_basic::perf::PerfProbeArgs::try_from_perf_attr` 完成 | `perf/mod.rs:144-145` |

### 2.2 部分实现 / 桩

| 维度 | 现状 | 缺什么 |
|---|---|---|
| `BpfPerfEvent::do_mmap(start, len, offset)` | kbpf-basic 已实现 (`RingPage::new_init` 把 mmap_page header 写到第 0 页, data_offset/data_size 设好) | tgoskits 这边 **从未调用** |
| `BpfPerfEventWrapper.phys_addr` | 字段存在, Drop 处占位 (`perf/bpf.rs:94-101`) | **始终是 None**, 因为 mmap path 没接 |
| `BpfPerfEventWrapper::write_event` | 若 `phys_addr.is_none()` 直接 `Ok(())` 不写 | 等价于 silent-drop, libbpf 永远拿不到 sample (见 §3 修复) |
| ringbuf map (`BPF_MAP_TYPE_RINGBUF`) | kbpf-basic 内有实现 | tgoskits `BpfMap::file_mmap` 默认走 `Err(NoSuchDevice)`; userland `bpf_ringbuf_reserve` 用的 mmap 走不通 |
| `PerfEvent::poll(2)` | `Pollable::poll` 转给 inner; `BpfPerfEventWrapper::poll` 检查 `inner.readable()` | `inner.readable()` 在 `phys_addr=None` 时永远 false (mmap_page 没设, `data_head==data_tail==0`) — 与 §3 同根 |

### 2.3 未实现

| 维度 | 状态 | 注 |
|---|---|---|
| `PerfEvent::file_mmap()` / `device_mmap()` | 默认 `Err(NoSuchDevice)` (`file/mod.rs:167`) | **核心阻塞**: userland `mmap(perf_fd, ...)` 直接 ENODEV |
| `BpfMap::file_mmap()` for RINGBUF | 同上 | 同上, 影响 `BPF_MAP_TYPE_RINGBUF` 路径 |
| `PERF_TYPE_HARDWARE / HW_CACHE / RAW` | dispatcher 直接 `Unsupported` | PMU 类 profile 不可用 |
| `PERF_TYPE_UPROBE` | `perf/uprobe.rs:22` 返回 `Unsupported` | 与 perf 主链解耦, 不挡 demo |
| `BpfPerfEvent` 的 `enabled` flag | 已存 (`enable/disable` 切 `data.enabled`) | 但 `write_event` 在 wrapper 层只检查 `phys_addr.is_none()`, 没检查 `inner.enabled()`; 若 mmap 通了仍可能 attach-then-write 的窗口不对齐 |
| sample_type 非 `PERF_SAMPLE_RAW` 的支持 | `perf_event_open_bpf` 用 `debug_assert_eq!` (`perf/bpf.rs:123-127`) | release build 下断言被去掉 → 任意 sample_type 都走过, 但 `BpfPerfEvent` 内部固定写 `PerfSample` 结构 (PERF_RECORD_SAMPLE), 不一致时 libbpf 解析失败 |
| `mmap` 后的物理页生命周期管理 | `Drop` 占位 (perf/bpf.rs:94-101) | mmap 接通后必须真的释放, 当前是 `_ = phys_addr.take()` 即 leak |

## 3. 关键阻塞点详解 + 修复方案

### 3.1 `mmap(perf_fd, ...)` 通路 (本审计的核心)

**当前**: `sys_mmap` 收到 fd-backed mmap, 走 `FileLike::file_mmap` 或 `device_mmap`. `PerfEvent` 两个都没 override → 默认 `Err(NoSuchDevice)` → userland `mmap` 返 ENODEV.

**libbpf 的期望** (`tools/lib/bpf/libbpf.c:perf_buffer__open`):
```
fd = perf_event_open(...);
header = mmap(NULL, page_count * page_size, PROT_READ|PROT_WRITE, MAP_SHARED, fd, 0);
// header[0] 是 perf_event_mmap_page, header[1..] 是 data ring
```

**修复方向** (排序按 "改动量小 → 大"):

| 方案 | 描述 | 改动量 | 风险 |
|---|---|---|---|
| A. 走 `device_mmap` 路径 | 仿 `ion.rs:device_mmap`, 在 PerfEvent 内分配 `(2^N + 1) * 4K` 物理连续页, 返 `DeviceMmap { phys_addr, len, flags }`, 让 `sys_mmap` 在用户地址空间映射. 同时调 `BpfPerfEvent::do_mmap(virt_kernel_start, len, 0)` 让 kbpf-basic 接管 RingPage. | 中 (~150 行: 新 method + page alloc + drop 时 dealloc) | 物理连续 N 页在 ax_alloc 上要确认 alloc_pages 支持 |
| B. 走 `file_mmap` (file-backed) | 给 perf-event-fd 造一个"虚拟 file backend". 不自然, perf-event 没有 backing inode. | 高 | 不推荐 |
| C. 完全旁路 mmap, 让用户态用 `read(fd)` 拿 sample | 偏离 Linux ABI, libbpf/aya 必须重写 | 极高 | 不可行 (用户程序就是 libbpf 风格) |

→ **首选方案 A**.

最小可工作的代码骨架 (放 perf/bpf.rs):

```rust
impl PerfEventOps for BpfPerfEventWrapper {
    // 新增 — 由上层 PerfEvent::device_mmap 调
    fn device_mmap(&mut self, len: usize) -> AxResult<(PhysAddr, MappingFlags)> {
        let page_count = len / PAGE_SIZE_4K;
        // 物理连续 N 页, READ|WRITE, zeroed
        let paddr = frame_alloc_contiguous(page_count, true)?;
        // 写 perf_event_mmap_page 的头
        let kvirt = phys_to_virt(paddr);
        self.inner.do_mmap(kvirt.as_usize(), len, 0).map_err(...)?;
        self.phys_addr = Some((paddr, page_count));
        Ok((paddr, MappingFlags::READ | MappingFlags::WRITE))
    }
}

impl FileLike for PerfEvent {
    fn device_mmap(&self, _offset: u64) -> AxResult<DeviceMmap> {
        // 取 inner 的 phys_addr — 但要求先 mmap-prepare; 实际 libbpf 是先 mmap 后 ioctl,
        // 所以这里要在第一次 mmap 时按 len 现场分配
        // 见反 fallback §6
    }
}
```

**反 fallback 警告**:
- 不要分配后只填 `phys_addr` 不写 mmap_page header — 用户态首次读会拿到 garbage `data_offset`.
- 不要 leak: `Drop for BpfPerfEventWrapper` 必须 `frame_dealloc` (当前 `perf/bpf.rs:94-101` 已留位).
- mmap 的 `len` 必须是 `(2^N + 1) * 4K`, 与 libbpf 一致; 不是这个值就 `Err(InvalidInput)`.

### 3.2 ringbuf 映射的两条路径冲突

aya/libbpf 有两套 perf 数据传递:

1. **legacy perf buffer** (`BPF_PERF_EVENT_OUTPUT`) — 每 CPU 一个 perf-event-fd + mmap'd ringbuf. 我们走的就是这条 (见 §3.1).
2. **modern BPF ringbuf** (`BPF_MAP_TYPE_RINGBUF`) — 一个全局 map fd 直接 mmap. kbpf-basic 有实现 (`RingBufMap`), 但 `BpfMap::file_mmap` 也是默认 ENODEV.

→ aya_log 默认走方案 1 (`PerfEventArray<u8>` 在 `aya-log-ebpf`), 修通方案 1 即可解锁所有现有 user 程序的 log.
→ 方案 2 留给后续 demo 增强 (例如 sched_switch 高频事件用 ringbuf 更轻).

### 3.3 同步 / wake-up

`BpfPerfEventWrapper::write_event` 当前在写完后调 `poll_ready.wake()` (perf/bpf.rs:62). 这条 OK. 但:
- `Pollable::poll` 看的是 `inner.readable()` (perf/bpf.rs:106-110), 它读 `perf_event_mmap_page.data_head != data_tail`.
- 修通 §3.1 后, user 态写 `data_tail` 推进, 内核才会发现"还没 readable" → 这是 libbpf 期望的语义, 不用动. 关键是首次 `mmap` 把 mmap_page header 正确初始化.

### 3.4 `BPF_F_CURRENT_CPU` 与 PERF_EVENT_ARRAY map

aya 的 `PerfEventArray<u8>` 创建一个 `BPF_MAP_TYPE_PERF_EVENT_ARRAY`, 每个 CPU 一个 slot, 装一个 perf_event_fd. `bpf_perf_event_output` 收到的 ctx + map_ptr + flags=BPF_F_CURRENT_CPU 时, 解析为 `map[this_cpu_id]` → fd → 写该 fd 的 ringbuf.

- 当前 kbpf-basic 在 `transform.rs::perf_event_output` 的实现是直接 fd-based (上层已经把 map slot 解析成 fd 了); 这部分 PR-A 已经接通, 不需要再动.
- 验证: §6.A 的 perf buffer demo 跑通即等同验证.

## 4. 与 Linux 语义的差距 (摘要表)

| 维度 | Linux | tgoskits | 差距严重度 |
|---|---|---|---|
| `mmap(perf_fd, header_pages, ...)` 返物理页 | ✅ | ❌ ENODEV | **致命** |
| `mmap(ringbuf_map_fd, ...)` | ✅ | ❌ ENODEV | 中 (绕开走 perf buffer 即可) |
| `bpf_perf_event_output` helper | ✅ | ⚠️ 写, 但因 §3.1 实际丢失 | 等同 §3.1 |
| `PERF_RECORD_SAMPLE` 格式 | ✅ | ✅ kbpf-basic RingPage 已按 PerfSample 头写 | 0 |
| `PERF_RECORD_LOST` (lost-record) | ✅ | ✅ RingPage::write_lost 已实现 | 0 |
| `PERF_EVENT_IOC_PERIOD` (动态调采样周期) | ✅ | ❌ | 低 |
| `PERF_EVENT_IOC_REFRESH` (一次性触发 N 次) | ✅ | ❌ | 低 |
| `PERF_TYPE_HARDWARE` (PMU) | ✅ | ❌ | 中 — profile demo 需要 |
| `read(perf_fd)` 拿 counter 值 | ✅ | `Err(Unsupported)` (perf/mod.rs:97) | 中 (PMU profile 时需要) |
| `BPF_MAP_TYPE_RINGBUF` mmap | ✅ | ❌ | 中 |

## 5. 残缺清单 (按补完优先级)

### P0 — 任何 aya/libbpf-style demo 都要

| ID | 残缺项 | 影响 | 修复 §|
|---|---|---|---|
| PERF-P0-1 | `PerfEvent::device_mmap` 未实现 → userland mmap perf_fd 失败 | aya_log 在内核侧静默丢 sample, 所有 demo 看不到内核输出 | §3.1 方案 A |
| PERF-P0-2 | `BpfPerfEventWrapper::do_mmap` 没被调用 | 同上 | §3.1 同改 |
| PERF-P0-3 | `Drop` 不释放 phys_addr | 进程退出后泄漏页 | §3.1 同改 |

### P1 — 鲁棒性 / Linux 对齐

| ID | 残缺项 | 影响 |
|---|---|---|
| PERF-P1-1 | `BpfMap::file_mmap()` for RINGBUF map | aya `RingBuf` 用法不可用 |
| PERF-P1-2 | sample_type 校验 (perf/bpf.rs:124) `debug_assert_eq!` 改成 `if != raw return EINVAL` | release build 下错误的 sample_type 写出去会被 libbpf 拒, 但是是 ABI 层的, 应在 open 时就拒 |
| PERF-P1-3 | `read(perf_fd)` 返 counter (即使是固定 0) | bpftrace `perf_event_attr.read_format` 路径要这个 |
| PERF-P1-4 | poll fd readiness 在 mmap 之前应该返 EAGAIN / 无 IN | 当前 mmap=None 时 `inner.readable()` 总为 false, 行为正确, 但要在 §6 验证脚本里覆盖 |

### P2 — 远期

| ID | 残缺项 |
|---|---|
| PERF-P2-1 | `PERF_TYPE_HARDWARE` PMU counter (依赖架构 PMU 驱动) |
| PERF-P2-2 | `PERF_EVENT_IOC_PERIOD` / `_REFRESH` |
| PERF-P2-3 | per-CPU PerfEventArray 自动 fan-out (kbpf-basic 已支持, 但要核对 `BPF_F_CURRENT_CPU` 实际 flag 路径) |

## 6. 验证骨架 (反 fallback)

在 qemu 内 (内核需含本审计 P0 修复后):

### A. mmap 闭环 (最小可证)

```c
int fd = perf_event_open(&attr_software_bpf, 0/*pid*/, 0/*cpu*/, -1, 0);
assert(fd >= 0);
void *m = mmap(NULL, 9 * 4096, PROT_READ|PROT_WRITE, MAP_SHARED, fd, 0);
assert(m != MAP_FAILED);
struct perf_event_mmap_page *hdr = m;
assert(hdr->data_offset == 4096);
assert(hdr->data_size == 8 * 4096);
assert(hdr->data_head == 0 && hdr->data_tail == 0);
munmap(m, 9 * 4096);
close(fd);
```

不允许的 fallback:
- `mmap` 返 `MAP_FAILED` 但测试 `[ -n "$_t" ]` 类弱断言通过 → 错.
- `mmap` 返 `MAP_FAILED` 但 fallback 到 `read(fd)` → 错, libbpf 不走这条.

### B. perf_event_output 端到端

```c
// 内核: 加载一个最小 BPF 程序, 每次 sys_enter_openat 写 8 字节到 perf ringbuf
// 用户: mmap + 循环 sample
// 触发: open("/etc/hostname") × 100
// 断言: hdr->data_head 推进了至少 100 * sizeof(PerfSample 8B) 的距离, 容差 ± 20%
```

不允许的 fallback:
- BPF prog 里 `bpf_trace_printk` (打 trace_pipe) 看到了就当成"通了" → 错, 那条路径与 perf 无关.
- 把测试改成 `cat /sys/kernel/debug/tracing/trace | grep openat` → 错, 这绕过了 perf 通路.

### C. ringbuf 不溢出

```c
// 连开 10k 次, 监控 hdr->data_head 推进、用户态 data_tail 追上, lost_count == 0
```

### D. Drop 释放页

```c
// 内核侧: 在 frame_alloc / frame_dealloc 加 RAII counter (debug only)
// 用户: open perf_fd, mmap, close → 验证 counter 回到 baseline
```

## 7. 与 tracepoint 审计的耦合关系

- tracepoint 修复 (TP-P0-*) 只能让 attach 与触发链路通; 触发后 BPF prog 想把数据带回用户态, **必须先解决 PERF-P0-1**. 任何 sched_switch demo 没有 PERF-P0-1 就是 "在内核里看到调度, 在用户什么都看不到".
- syscall_ebpf demo (HashMap 计数 + 用户态 `BPF_MAP_LOOKUP_ELEM`) 不依赖 perf ringbuf, 是 PERF-P0 阻塞期间唯一稳定的 demo (见 demo-stack-design.md §2.1).

## 8. 建议 PR 拆分 (落到 git 上)

| PR | 范围 | 完成判据 |
|---|---|---|
| **PR-RB-A** | PERF-P0-1/2/3 + 单元测试 (验证 §6.A mmap 闭环) | qemu x86_64 跑 §6.A C 程序返 0; 任意 aya 程序的 `EbpfLogger::init` 不报错 |
| **PR-RB-B** | PERF-P1-1 (RINGBUF map mmap) | `bpf_ringbuf_reserve` 闭环 |
| **PR-RB-C** | PERF-P1-2/3/4 鲁棒性 | 反向测试 (错的 sample_type 拿 EINVAL) |
| **PR-RB-D** | PMU profile (PERF-P2-*) | 远期, 不挡 demo |
