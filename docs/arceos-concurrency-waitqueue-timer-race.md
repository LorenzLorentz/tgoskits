# ArceOS 多核并发卡死问题定位全过程

## 1. 问题背景

在 ArceOS 的 SMP 场景下，我们之前遇到过概率性的多核并发卡死问题。

早期现象主要集中在：

- `wait_queue`
- `parallel`

这些任务相关测例上。

在前一轮修改之后，原有 `wait_queue` 和 `parallel` 测例已经不容易稳定复现问题，但这并不意味着问题已经消失，更可能只是：

- 原有测例的压力模式不再足以持续打中剩余的竞争窗口。

因此，本轮目标不是“看旧测例还会不会挂”，而是：

1. 系统性梳理当前 ArceOS 中仍然可能导致多核并发卡死的风险路径
2. 设计一个新的组合型压力测例
3. 借助该测例对问题做逐层定位，找到更根本的原因

---

## 2. 新测例的设计

为了把多个潜在风险同时放大，新建了组合型压力测例：

- `test-suit/arceos/rust/task/concurrency_stress`

它叠加了多种并发压力：

- `WaitQueue` 的等待/唤醒竞争
- 带 timeout 的等待
- `Mutex` 高竞争下的 owner handoff
- 高频 `yield_now()` / `sleep()`
- CPU affinity 变化与迁移

测例使用：

- `SMP=8`

并且在后续分析中分别测试了：

- 默认 FIFO 调度
- `sched-rr` Round-Robin 调度

### 2.1 测例具体在测什么

这个测例不是单点功能测试，而是把多种并发行为交织在一起，持续制造：

- 多个 worker 在每一轮开始前通过 `WaitQueue::wait_timeout_until()` 等待 `start_round`
- 进入第一段 critical section 前后频繁 `yield_now()`
- 在 critical section 中高频竞争同一个 `Mutex`
- 穿插短时间 `sleep(Duration::from_millis(1))`
- 在轮次和 section 之间不断变化 CPU affinity
- 在 `midway` 和 `finish` 两个阶段分别通过 `WaitQueue` 做 barrier 式推进

因此它同时覆盖了：

- `WaitQueue` 正常通知与 timeout 的竞争
- `Mutex` owner handoff 与 waiter 唤醒
- 远端 CPU 上的 run queue 插入与延迟调度
- 高频让出 CPU、短睡眠、迁移带来的调度抖动

从实现上看，worker 会在一轮内经历：

1. `WaitStart`
2. `FirstCritical`
3. `Midway`
4. `WaitRelease`
5. `SecondCritical`
6. `Finished`

所以当测例超时时，打印出来的 `first_critical`、`second_critical`、`midway` 等阶段信息，正好可以反映问题停在了哪一段推进链路上。

### 2.2 测例如何执行

本轮分析中主要使用的是 x86_64 QEMU 配置，对应命令为：

```bash
cargo xtask arceos qemu \
  --package arceos-concurrency-stress \
  --target x86_64-unknown-none \
  --qemu-config test-suit/arceos/rust/task/concurrency_stress/qemu-x86_64.toml
```

这个命令会读取：

- `test-suit/arceos/rust/task/concurrency_stress/.build-x86_64-unknown-none.toml`
- `test-suit/arceos/rust/task/concurrency_stress/qemu-x86_64.toml`

其中关键配置包括：

- `features = ["ax-std"]`
- `SMP=8`
- `qemu -smp 8`

而测例本身在 `Cargo.toml` 中显式启用了：

- `alloc`
- `multitask`
- `irq`
- `sched-rr`

因此这里实际验证的是：

- 开中断
- 开多线程
- 8 核 SMP
- RR 调度

---

## 3. 最初观察到的现象

在这个新测例中，修复前可以概率性复现两类超时：

- `timed out waiting workers to reach midway`
- `timed out waiting workers to finish`

随着测例压力增大，问题更容易出现。

从测例侧的 worker 状态快照看：

- 一些 worker 并不是卡在 testcase 自己定义的 barrier 上
- 很多时候它们停在：
  - `first_critical`
  - `second_critical`

这说明问题表面上和 mutex/调度更接近，而不只是单纯的 barrier 逻辑错误。

---

## 4. 第一轮怀疑方向

最开始被怀疑的路径包括：

- `axsync::RawMutex`
- `axtask::WaitQueue`
- `run_queue::unblock_task`
- timer wakeup 路径
- affinity 迁移路径

一开始的直觉是：

- 既然很多 worker 卡在 critical section 附近，可能是 `RawMutex` 有问题。

但后续定位发现，这个判断只抓到了表象，没有抓到真正的根。

---

## 5. 第一轮定位：WaitQueue 与 timer wakeup 是否重复处理同一个 waiter

为了避免“每次只在一个点加日志，然后反复很多轮”，第一轮定位时同时在多个位置加了日志：

- `axsync/src/mutex.rs`
- `axtask/src/wait_queue.rs`
- `axtask/src/run_queue.rs`
- `axtask/src/task.rs`
- `axtask/src/timers.rs`

重点关注：

1. 谁从 wait queue 中选中了 waiter
2. waiter 当前的 timer ticket 是什么
3. timer callback 是否还会命中它
4. `Blocked -> Ready` 是否被重复执行

### 第一轮关键发现

日志中出现了这种序列：

1. `notify_one` / `notify_one_with` 已经从 wait queue 中选中了某个 waiter
2. 该 waiter 已通过正常路径被唤醒
3. 随后旧 timer event 仍然到来
4. timer callback 又尝试对同一个 waiter 做一次唤醒
5. 后面出现：
   - `transition_state(expected=Blocked, target=Ready)` 失败
   - `run_queue unblock_task skipped`
   - 甚至出现 “picked non-blocked task” 这类现象

### 第一轮得到的中间结论

这说明问题并不是：

- “任务根本没有被唤醒”

而是：

- **同一个 waiter 先被正常 notify 路径处理了**
- **旧 timeout event 稍后又来参与了一次 wakeup**

也就是：

**出现了 duplicate wakeup race。**

---

## 6. 为什么“duplicate wakeup race”还不是最根原因

虽然表面上看到了 duplicate wakeup，但继续分析后发现：

- “A 被多唤醒了一次”本身并不是最根本的问题

真正更重要的是：

- **为什么旧 timer event 在正常唤醒已经发生之后，还会被当成有效事件处理？**

也就是：

- 为什么正常样本里旧 timer event 会被挡掉
- 而异常样本里这个旧 timer event 没被挡掉

这就把分析重点从“重复唤醒”推进到了：

**timer ticket 为什么没有及时失效。**

---

## 7. 第二轮定位：ticket 生命周期

第二轮定位不再只看 wakeup，而是专门打 ticket 生命周期：

- `set_alarm_wakeup()`
- `timer_ticket_expired()`
- `cancel_events(from_timer)`
- timer callback 中看到的 `observed_ticket`

### 关键问题

我们真正想确认的是：

> 一个 waiter 被 `notify` 选中之后，旧 ticket 是在什么时候失效的？

### 正常样本中的顺序

正常样本里，可以看到典型序列：

1. `waitq notify_one selected waiter: task=..., old_ticket=...`
2. `waitq cancel_events(from_timer): task=..., ticket_before_expire=...`
3. `task timer_ticket_expired: task=..., old_ticket=...`
4. 后续：
   `timer callback ignored stale ticket: task=..., event_ticket=..., observed_ticket=...`

这说明：

- 旧 timeout event 不是没到
- 而是它到的时候，ticket 已经失效，所以被忽略

### 异常样本中的顺序

异常样本里，出现的是另一种情况：

1. waiter 已经被 `notify` 选中
2. 但还没来得及执行 `cancel_events(from_timer)` / `timer_ticket_expired()`
3. 旧 timer event 先到了
4. timer callback 没有被挡掉
5. 后面才出现 duplicate wakeup race

### 第二轮中间结论

所以更直接的原因不是“重复唤醒本身”，而是：

**ticket 的失效时机太晚。**

更准确地说：

> ticket 的失效依赖 waiter 之后重新运行并执行 `cancel_events(from_timer)`，  
> 只要 waiter 还没来得及跑到这里，旧 timer event 就还有机会先命中。

---

## 8. 第一步修复及验证

基于上面的观察，我们先验证了一个最小修复：

- 在 `notify_one()` / `notify_one_with()` 选中 waiter 后
- 立刻执行 `task.timer_ticket_expired()`
- 然后再 `unblock_one_task(...)`

也就是把 ticket 失效从“waiter 之后自己跑回来时”提前到“notify 决定唤醒它的瞬间”。

### 为什么这个修复有效

因为它直接消掉了这个危险窗口：

- 正常唤醒已经发生
- 但旧 timer ticket 还没失效

### 验证结果

在本轮会话中，应用这个最小修复后：

- `cargo fmt --all`
- `cargo xtask clippy --package arceos-concurrency-stress`
- 多轮 `boot + test + exit`

都表现出明显更稳定的结果，至少确认了前 **5 轮**完整回环通过。

这说明：

- 这个最小修复确实打中了一个重要问题点

但这还不是问题的最终根因，只是把问题窗口提前封掉了。

---

## 9. 第三轮追问：为什么 waiter 来不及执行 `cancel_events(from_timer)`？

你提出了一个关键追问：

> 不是所有 waiter 都来不及，为什么有的 waiter 正常来得及，而有的来不及？

这一步把问题继续往下推到了调度层。

---

## 10. 第三轮定位：remote wakeup 之后的真实运行延迟

这一轮我们不再只看 ticket，而是进一步记录：

1. waiter 被唤醒时是本地唤醒还是跨 CPU 唤醒
2. remote wakeup 时是否真正触发了目标 CPU 重新调度
3. 从 `unblock_task` 成功把任务放回 ready queue，到它真正开始 `Running` 之间的延迟有多大

### 为此加入的日志

- `waitq notify_one selected waiter: ... current_cpu=..., target_cpu=..., local=...`
- `waitq notify_one_with selected waiter: ... current_cpu=..., target_cpu=..., local=...`
- `run_queue unblock_task remote wakeup: ... resched_ignored=true`
- `task running after unblock: ... remote=..., delay_ns=...`

---

## 11. 第三轮关键发现

### 发现 1：很多问题 waiter 确实是 remote wakeup

日志里反复出现：

- `local=false`
- `resched_ignored=true`

说明这些 waiter 是：

- 被放回了**远端 CPU** 的 ready queue
- 但远端 `resched` 并没有被主动触发

### 发现 2：remote waiter 真正开始运行的延迟差异很大

这轮抓到了大量 `delay_ns`：

#### 较短样本

- `9ms`
- `17ms`
- `19ms`
- `25ms`

这些 waiter 通常来得及执行：

- `cancel_events(from_timer)`
- `timer_ticket_expired()`

#### 很慢的样本

也抓到了很多明显偏大的 remote waiter 延迟：

- `94.8ms`
- `105.8ms`
- `107.4ms`
- `123.9ms`
- `125.6ms`
- `127.7ms`
- `129.4ms`
- `154.3ms`
- `163.4ms`
- 甚至更高

这说明：

**不是 remote wakeup 一定慢，而是它的运行时延非常不稳定。**

一旦这个延迟足够大，waiter 就来不及去清理旧 ticket。

---

## 12. 为什么“只有某些 remote waiter 来不及”

这里是一个非常关键的理解点。

不是所有 remote wakeup 都出问题，是因为：

- waiter 被放进远端 CPU 的 ready queue 后
- 是否很快运行，取决于目标 CPU 当时的调度情况

包括：

- 当前正在跑的任务是否很快 `yield`
- 是否很快阻塞
- 是否很快触发下一次真正有利于 waiter 的调度
- 目标 CPU 上是否又来了新的 runnable 任务

所以同样是 remote wakeup：

- 有的 waiter 运气好，十几毫秒就运行
- 有的 waiter 运气差，拖到上百毫秒

这就是为什么“有的来得及、有的来不及”。

---

## 13. 切换到 `sched-rr` 之后是否还会卡住？

这个问题也专门验证过。

### 先做了什么

为了避免“还是沿用了 FIFO 的旧缓存产物”，先执行了：

- `cargo clean`

然后确认 `concurrency_stress` 的 `Cargo.toml` 里已经显式启用了：

- `sched-rr`

### 结果

在清缓存并重新构建之后，`sched-rr` 版本的测例：

- **仍然会卡住**

例如出现过：

- `round 0: timed out waiting workers to finish, finished=21`

### 这说明什么

这说明问题并不是：

- “因为之前其实跑的是 FIFO 才有”

而是：

- **即使真的切到 RR，问题仍然存在**

所以根因不在“调度器类型是不是 RR/FIFO”，而是在：

**remote wakeup 后没有积极推动目标 CPU 尽快运行 waiter。**

---

## 14. RR 下时间片是多少？为什么问题还会超过一个时间片？

### RR 时间片

在 `axtask` 中：

- `MAX_TIME_SLICE = 5`

而当前配置中：

- `ticks-per-sec = 100`

所以：

- 1 tick = 10ms
- RR 时间片约为 **50ms**

### 但实际测到的运行延迟远大于 50ms

日志里明确观察到很多 remote waiter：

- 延迟超过 `100ms`
- 甚至达到 `150ms+`

这说明：

**“下一个时间片就会调到这个 waiter”这个推断并不成立。**

因为当前实现里：

- remote wakeup 只是把任务放进目标 CPU 的 ready queue
- 但不会主动强制那个 CPU 立即 resched

所以 waiter 是否在下一个时间片运行，并没有被保证。

---

## 15. 正常与异常时序图

### 15.1 正常时序

```text
Worker A            WaitQueue               Timer                 Target CPU
   |                    |                     |                        |
   |---- wait(timeout)->|                     |                        |
   |   state=Blocked    |---- set ticket ---->|                        |
   |                    |                     |                        |
   |                    |---- notify_one ---->|                        |
   |                    |  选中 A             |                        |
   |                    |                     |                        |
   |                    |------ unblock ----------------------------->  |
   |                    |                     |                        |
   |                    |                     |          A 很快被调度运行 |
   |                    |                     |<-----------------------|
   |<-------------------|                     |                        |
   | cancel_events()    |                     |                        |
   | ticket expired     |                     |                        |
   |                    |                     |                        |
   |                    |                     |--- old timer arrives -->|
   |                    |                     |   发现 ticket stale     |
   |                    |                     |   直接忽略              |
   |                    |                     |                        |
   |---------------- 系统继续正常推进 ---------------------------------|
```

### 15.2 异常时序

```text
Worker A            WaitQueue               Timer                 Target CPU
   |                    |                     |                        |
   |---- wait(timeout)->|                     |                        |
   |   state=Blocked    |---- set ticket ---->|                        |
   |                    |                     |                        |
   |                    |---- notify_one ---->|                        |
   |                    |  选中 A             |                        |
   |                    |                     |                        |
   |                    |------ unblock ----------------------------->  |
   |                    |                     |                        |
   |                    |                     |   A 被放进 ready queue   |
   |                    |                     |   但远端 resched 被忽略 |
   |                    |                     |   A 一直没真正跑起来    |
   |                    |                     |                        |
   |                    |                     |--- old timer arrives -->|
   |                    |                     |   ticket 还没失效      |
   |                    |                     |   timer callback 命中   |
   |                    |                     |                        |
   |                    |        duplicate wakeup / 状态竞争 / 队列污染 |
   |                    |                     |                        |
   |---------------------- 某些 waiter 最终永远推进不了 ----------------|
```

---

## 16. 最终结论：问题的完整因果链

到这一步，问题的完整链路已经比较清楚：

1. waiter 正常被 `notify` 选中
2. 它并不是立刻运行，而是被放回目标 CPU 的 ready queue
3. 如果这是 remote wakeup，当前实现不会主动推动目标 CPU 立即 resched
4. waiter 真正运行的延迟有时会很大，而且高度不可控
5. 在它真正运行并执行 `cancel_events(from_timer)` 之前，旧 timer ticket 仍有效
6. 如果这时 timeout event 先到，timer callback 就不会被挡掉
7. 之后才进入 duplicate wakeup race
8. 最终表现为 worker 推进失败、主线程超时，看起来像“卡死”

所以：

### 表面现象

- duplicate wakeup race

### 更直接原因

- waiter 来不及执行 `cancel_events(from_timer)`，旧 ticket 没及时失效

### 更深层调度原因

- remote wakeup 后 waiter 只是进入远端 ready queue
- 但远端 `resched` 被忽略
- waiter 的真实运行时延可能远大于一个时间片

### 更根本的设计问题

**当前协议把 timer ticket 的失效放在“waiter 重新获得 CPU 之后”才做，  
但在 SMP 下 remote wakeup 后的重新调度延迟并不受控。**

也就是说：

> timer ticket 的正确性，依赖了一个并不可靠的前提：  
> 被唤醒的 waiter 会很快重新运行。

这才是问题最根上的原因。

---

## 17. 为什么第一步修复有效

第一步修复做的是：

- `notify_one` / `notify_one_with` 选中 waiter 后
- 立即执行 `task.timer_ticket_expired()`

这等于把 ticket 失效从：

- waiter 之后自己跑回来时

提前到了：

- notify 决定由正常路径处理这个 waiter 的时刻

所以它有效，不是碰巧，而是因为：

**它绕开了“waiter 之后是否能及时重新运行”这个不可靠前提。**

---

## 18. 相关文件

- `os/arceos/modules/axtask/src/wait_queue.rs`
- `os/arceos/modules/axtask/src/timers.rs`
- `os/arceos/modules/axtask/src/run_queue.rs`
- `os/arceos/modules/axtask/src/task.rs`
- `os/arceos/modules/axsync/src/mutex.rs`
- `test-suit/arceos/rust/task/concurrency_stress/src/main.rs`

---

## 19. 最终一句话总结

**这次多核并发卡死问题的最根本原因，不是单纯的 duplicate wakeup，也不只是 timer race，而是：**

**ArceOS 当前把 timeout ticket 的失效放在 waiter 重新运行之后才做；  
而在 SMP 的 remote wakeup 场景下，waiter 被重新调度运行的时延并不受控。  
一旦这个时延过大，旧 timeout event 就会先命中，从而触发后续的一系列竞争并最终表现为卡死。**
