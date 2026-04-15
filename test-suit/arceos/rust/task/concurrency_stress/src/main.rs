#![cfg_attr(feature = "ax-std", no_std)]
#![cfg_attr(feature = "ax-std", no_main)]

#[macro_use]
#[cfg(feature = "ax-std")]
extern crate ax_std as std;

#[cfg(feature = "ax-std")]
use std::os::arceos::{
    api::task::{AxCpuMask, ax_set_current_affinity},
    modules::ax_task::WaitQueue,
};
#[cfg(feature = "ax-std")]
use std::{
    string::String,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
    vec::Vec,
};

#[cfg(feature = "ax-std")]
const START_TIMEOUT_MS: u64 = 180;
#[cfg(feature = "ax-std")]
const RELEASE_TIMEOUT_MS: u64 = 180;
#[cfg(feature = "ax-std")]
const ROUND_TIMEOUT_MS: u64 = 2500;
#[cfg(feature = "ax-std")]
const WATCHDOG_SLEEP_MS: u64 = 50;
#[cfg(feature = "ax-std")]
const WATCHDOG_STALL_TICKS: usize = 40;
#[cfg(feature = "ax-std")]
const STAGE_SHIFT: usize = 16;
#[cfg(feature = "ax-std")]
const DETAIL_SHIFT: usize = 8;

#[cfg(feature = "ax-std")]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum WorkerStage {
    Init           = 0,
    Armed          = 1,
    WaitStart      = 2,
    FirstCritical  = 3,
    Midway         = 4,
    WaitRelease    = 5,
    SecondCritical = 6,
    Finished       = 7,
}

#[cfg(feature = "ax-std")]
impl WorkerStage {
    fn from_usize(value: usize) -> Self {
        match value {
            0 => Self::Init,
            1 => Self::Armed,
            2 => Self::WaitStart,
            3 => Self::FirstCritical,
            4 => Self::Midway,
            5 => Self::WaitRelease,
            6 => Self::SecondCritical,
            7 => Self::Finished,
            _ => Self::Init,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Armed => "armed",
            Self::WaitStart => "wait_start",
            Self::FirstCritical => "first_critical",
            Self::Midway => "midway",
            Self::WaitRelease => "wait_release",
            Self::SecondCritical => "second_critical",
            Self::Finished => "finished",
        }
    }
}

#[cfg(feature = "ax-std")]
struct Shared {
    round_hits: usize,
    critical_sum: usize,
}

#[cfg(feature = "ax-std")]
struct StressContext {
    start_round: AtomicUsize,
    release_round: AtomicUsize,
    armed_workers: AtomicUsize,
    midway_workers: AtomicUsize,
    finished_workers: AtomicUsize,
    progress: AtomicUsize,
    stop_watchdog: AtomicBool,
    start_wq: WaitQueue,
    release_wq: WaitQueue,
    finished_wq: WaitQueue,
    arm_wq: WaitQueue,
    shared: Mutex<Shared>,
    worker_state: Vec<AtomicUsize>,
}

#[cfg(feature = "ax-std")]
impl StressContext {
    fn new(worker_count: usize) -> Self {
        let worker_state = (0..worker_count).map(|_| AtomicUsize::new(0)).collect();
        Self {
            start_round: AtomicUsize::new(0),
            release_round: AtomicUsize::new(0),
            armed_workers: AtomicUsize::new(0),
            midway_workers: AtomicUsize::new(0),
            finished_workers: AtomicUsize::new(0),
            progress: AtomicUsize::new(0),
            stop_watchdog: AtomicBool::new(false),
            start_wq: WaitQueue::new(),
            release_wq: WaitQueue::new(),
            finished_wq: WaitQueue::new(),
            arm_wq: WaitQueue::new(),
            shared: Mutex::new(Shared {
                round_hits: 0,
                critical_sum: 0,
            }),
            worker_state,
        }
    }

    fn bump_progress(&self) {
        self.progress.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(feature = "ax-std")]
fn encode_worker_state(round: usize, stage: WorkerStage, detail: usize) -> usize {
    (round << STAGE_SHIFT) | (detail << DETAIL_SHIFT) | stage as usize
}

#[cfg(feature = "ax-std")]
fn decode_worker_state(state: usize) -> (usize, WorkerStage, usize) {
    let round = state >> STAGE_SHIFT;
    let detail = (state >> DETAIL_SHIFT) & 0xff;
    let stage = WorkerStage::from_usize(state & 0xff);
    (round, stage, detail)
}

#[cfg(feature = "ax-std")]
fn record_worker_state(
    ctx: &StressContext,
    worker: usize,
    round: usize,
    stage: WorkerStage,
    detail: usize,
) {
    ctx.worker_state[worker].store(
        encode_worker_state(round, stage, detail.min(0xff)),
        Ordering::Release,
    );
}

#[cfg(feature = "ax-std")]
fn dump_stuck_workers(
    ctx: &StressContext,
    round: usize,
    expected_stage: WorkerStage,
    worker_count: usize,
    reason: &str,
) {
    println!(
        "stuck dump: reason={reason}, round={round}, expected_stage={}",
        expected_stage.as_str()
    );
    for worker in 0..worker_count {
        let state = ctx.worker_state[worker].load(Ordering::Acquire);
        let (worker_round, stage, detail) = decode_worker_state(state);
        if worker_round < round
            || (worker_round == round && (stage as usize) < (expected_stage as usize))
        {
            println!(
                "  worker {worker:02}: round={worker_round}, stage={}, detail={detail}",
                stage.as_str()
            );
        }
    }
}

#[cfg(feature = "ax-std")]
fn cpu_mask_all(cpu_num: usize) -> AxCpuMask {
    let mut mask = AxCpuMask::new();
    for cpu in 0..cpu_num {
        mask.set(cpu, true);
    }
    mask
}

#[cfg(feature = "ax-std")]
fn cpu_mask_excluding(cpu_num: usize, exclude: usize) -> AxCpuMask {
    let mut mask = AxCpuMask::new();
    for cpu in 0..cpu_num {
        if cpu != exclude {
            mask.set(cpu, true);
        }
    }
    if mask.is_empty() {
        mask.set(exclude, true);
    }
    mask
}

#[cfg(feature = "ax-std")]
fn set_affinity_for_round(worker: usize, round: usize, cpu_num: usize) {
    let home_cpu = worker % cpu_num;
    let mask = match round % 3 {
        0 => AxCpuMask::one_shot(home_cpu),
        1 => cpu_mask_excluding(cpu_num, home_cpu),
        _ => cpu_mask_all(cpu_num),
    };
    assert!(
        ax_set_current_affinity(mask).is_ok(),
        "worker {worker} failed to update affinity at round {round}",
    );
}

#[cfg(feature = "ax-std")]
fn wait_for_all_armed(ctx: &StressContext, worker_count: usize, round: usize) {
    let expected = worker_count * (round + 1);
    let timeout = ctx
        .arm_wq
        .wait_timeout_until(Duration::from_millis(ROUND_TIMEOUT_MS), || {
            ctx.armed_workers.load(Ordering::Acquire) >= expected
        });
    if timeout {
        dump_stuck_workers(
            ctx,
            round,
            WorkerStage::Armed,
            worker_count,
            "armed timeout",
        );
    }
    assert!(
        !timeout,
        "round {round}: timed out waiting workers to arm, armed={}",
        ctx.armed_workers.load(Ordering::Relaxed),
    );
}

#[cfg(feature = "ax-std")]
fn wait_for_all_finished(ctx: &StressContext, worker_count: usize, round: usize) {
    let expected = worker_count * (round + 1);
    let timeout = ctx
        .finished_wq
        .wait_timeout_until(Duration::from_millis(ROUND_TIMEOUT_MS), || {
            ctx.finished_workers.load(Ordering::Acquire) >= expected
        });
    if timeout {
        dump_stuck_workers(
            ctx,
            round,
            WorkerStage::Finished,
            worker_count,
            "finished timeout",
        );
    }
    assert!(
        !timeout,
        "round {round}: timed out waiting workers to finish, finished={}",
        ctx.finished_workers.load(Ordering::Relaxed),
    );
}

#[cfg(feature = "ax-std")]
fn wait_for_all_midway(ctx: &StressContext, worker_count: usize, round: usize) {
    let expected = worker_count * (round + 1);
    let timeout = ctx
        .arm_wq
        .wait_timeout_until(Duration::from_millis(ROUND_TIMEOUT_MS), || {
            ctx.midway_workers.load(Ordering::Acquire) >= expected
        });
    if timeout {
        dump_stuck_workers(
            ctx,
            round,
            WorkerStage::Midway,
            worker_count,
            "midway timeout",
        );
    }
    assert!(
        !timeout,
        "round {round}: timed out waiting workers to reach midway, midway={}",
        ctx.midway_workers.load(Ordering::Relaxed),
    );
}

#[cfg(feature = "ax-std")]
fn spawn_watchdog(ctx: Arc<StressContext>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut last_progress = ctx.progress.load(Ordering::Acquire);
        let mut stall_ticks = 0usize;
        while !ctx.stop_watchdog.load(Ordering::Acquire) {
            thread::sleep(Duration::from_millis(WATCHDOG_SLEEP_MS));
            let progress = ctx.progress.load(Ordering::Acquire);
            if progress == last_progress {
                stall_ticks += 1;
                assert!(
                    stall_ticks < WATCHDOG_STALL_TICKS,
                    "watchdog detected no progress: start_round={}, armed={}, finished={}",
                    ctx.start_round.load(Ordering::Relaxed),
                    ctx.armed_workers.load(Ordering::Relaxed),
                    ctx.finished_workers.load(Ordering::Relaxed),
                );
                if stall_ticks + 1 == WATCHDOG_STALL_TICKS {
                    let approx_round =
                        ctx.finished_workers.load(Ordering::Relaxed) / ctx.worker_state.len();
                    dump_stuck_workers(
                        &ctx,
                        approx_round,
                        WorkerStage::Finished,
                        ctx.worker_state.len(),
                        "watchdog stall",
                    );
                }
            } else {
                last_progress = progress;
                stall_ticks = 0;
            }
        }
    })
}

#[cfg(feature = "ax-std")]
fn spawn_workers(
    ctx: &Arc<StressContext>,
    worker_count: usize,
    rounds: usize,
    cpu_num: usize,
) -> Vec<thread::JoinHandle<usize>> {
    (0..worker_count)
        .map(|worker| {
            let ctx = Arc::clone(ctx);
            thread::spawn(move || {
                let mut local_score = 0usize;
                for round in 0..rounds {
                    record_worker_state(&ctx, worker, round, WorkerStage::Armed, 0);
                    set_affinity_for_round(worker, round, cpu_num);
                    ctx.armed_workers.fetch_add(1, Ordering::Release);
                    ctx.arm_wq.notify_one(true);
                    ctx.bump_progress();

                    while ctx.start_round.load(Ordering::Acquire) <= round {
                        record_worker_state(&ctx, worker, round, WorkerStage::WaitStart, 0);
                        let _timed_out = ctx
                            .start_wq
                            .wait_timeout_until(Duration::from_millis(START_TIMEOUT_MS), || {
                                ctx.start_round.load(Ordering::Acquire) > round
                            });
                        if ctx.start_round.load(Ordering::Acquire) <= round {
                            thread::yield_now();
                            ctx.bump_progress();
                        }
                    }

                    let first_sections = 2 + (worker + round) % 3;
                    for section in 0..first_sections {
                        record_worker_state(
                            &ctx,
                            worker,
                            round,
                            WorkerStage::FirstCritical,
                            section,
                        );
                        let mut guard = ctx.shared.lock();
                        guard.round_hits += 1;
                        guard.critical_sum += worker ^ round ^ section;
                        local_score = local_score.wrapping_add(guard.critical_sum);
                        if (worker + round + section) % 2 == 0 {
                            thread::yield_now();
                        }
                        if (worker + round + section) % 4 == 0 {
                            for _ in 0..128 {
                                core::hint::spin_loop();
                            }
                        }
                        drop(guard);

                        if (worker + round + section) % 3 == 0 {
                            thread::sleep(Duration::from_millis(1));
                        } else {
                            thread::yield_now();
                        }
                    }

                    record_worker_state(&ctx, worker, round, WorkerStage::Midway, first_sections);
                    ctx.midway_workers.fetch_add(1, Ordering::Release);
                    ctx.arm_wq.notify_one(true);
                    ctx.bump_progress();

                    while ctx.release_round.load(Ordering::Acquire) <= round {
                        record_worker_state(&ctx, worker, round, WorkerStage::WaitRelease, 0);
                        let _timed_out = ctx
                            .release_wq
                            .wait_timeout_until(Duration::from_millis(RELEASE_TIMEOUT_MS), || {
                                ctx.release_round.load(Ordering::Acquire) > round
                            });
                        if ctx.release_round.load(Ordering::Acquire) <= round {
                            thread::yield_now();
                            ctx.bump_progress();
                        }
                    }

                    let second_sections = 3 + (worker + round) % 4;
                    for section in 0..second_sections {
                        record_worker_state(
                            &ctx,
                            worker,
                            round,
                            WorkerStage::SecondCritical,
                            section,
                        );
                        set_affinity_for_round(worker + section + round, round + section, cpu_num);
                        let mut guard = ctx.shared.lock();
                        guard.round_hits += 1;
                        guard.critical_sum += (worker + section) ^ (round << 1);
                        local_score = local_score.wrapping_add(guard.critical_sum);
                        if (worker + round + section) % 2 == 1 {
                            thread::yield_now();
                        }
                        if (worker + round + section) % 5 == 0 {
                            for _ in 0..256 {
                                core::hint::spin_loop();
                            }
                        }
                        drop(guard);

                        if (worker + round + section) % 2 == 0 {
                            thread::sleep(Duration::from_millis(1));
                        } else {
                            thread::yield_now();
                        }
                    }

                    record_worker_state(
                        &ctx,
                        worker,
                        round,
                        WorkerStage::Finished,
                        second_sections,
                    );
                    ctx.finished_workers.fetch_add(1, Ordering::Release);
                    ctx.finished_wq.notify_one(true);
                    ctx.bump_progress();
                }
                local_score
            })
        })
        .collect()
}

#[cfg(feature = "ax-std")]
fn run_stress() {
    let cpu_num = thread::available_parallelism().unwrap().get();
    if cpu_num <= 1 {
        println!("skip concurrency stress: single CPU");
        return;
    }

    let worker_count = cpu_num * 4;
    let rounds = 64;
    println!("concurrency_stress: cpu_num={cpu_num}, worker_count={worker_count}, rounds={rounds}");

    let ctx = Arc::new(StressContext::new(worker_count));
    let watchdog = spawn_watchdog(Arc::clone(&ctx));
    let workers = spawn_workers(&ctx, worker_count, rounds, cpu_num);

    for round in 0..rounds {
        wait_for_all_armed(&ctx, worker_count, round);

        let pre_notify_delay = match round % 4 {
            0 => Duration::from_millis(0),
            1 => Duration::from_millis(START_TIMEOUT_MS / 4),
            2 => Duration::from_millis(START_TIMEOUT_MS.saturating_sub(2)),
            _ => Duration::from_millis(START_TIMEOUT_MS + 1),
        };
        if pre_notify_delay.as_millis() > 0 {
            thread::sleep(pre_notify_delay);
        } else {
            thread::yield_now();
        }

        ctx.start_round.store(round + 1, Ordering::Release);
        ctx.start_wq.notify_all(true);
        ctx.bump_progress();

        wait_for_all_midway(&ctx, worker_count, round);

        let release_delay = match round % 5 {
            0 => Duration::from_millis(0),
            1 => Duration::from_millis(RELEASE_TIMEOUT_MS / 3),
            2 => Duration::from_millis(RELEASE_TIMEOUT_MS.saturating_sub(2)),
            3 => Duration::from_millis(RELEASE_TIMEOUT_MS + 1),
            _ => Duration::from_millis(RELEASE_TIMEOUT_MS / 2),
        };
        if release_delay.as_millis() > 0 {
            thread::sleep(release_delay);
        } else {
            thread::yield_now();
        }

        ctx.release_round.store(round + 1, Ordering::Release);
        ctx.release_wq.notify_all(true);
        ctx.bump_progress();
        wait_for_all_finished(&ctx, worker_count, round);

        let shared = ctx.shared.lock();
        let expected_min = worker_count * (round + 1) * 5;
        assert!(
            shared.round_hits >= expected_min,
            "round {round}: insufficient critical sections, hits={}, expected_min={expected_min}",
            shared.round_hits,
        );
        drop(shared);

        if round % 5 == 0 {
            println!(
                "round {round:02}: armed={}, midway={}, finished={}, start_round={}, \
                 release_round={}",
                ctx.armed_workers.load(Ordering::Relaxed),
                ctx.midway_workers.load(Ordering::Relaxed),
                ctx.finished_workers.load(Ordering::Relaxed),
                ctx.start_round.load(Ordering::Relaxed),
                ctx.release_round.load(Ordering::Relaxed),
            );
        }
    }

    let mut total_score = 0usize;
    for handle in workers {
        total_score = total_score.wrapping_add(handle.join().unwrap());
    }

    ctx.stop_watchdog.store(true, Ordering::Release);
    watchdog.join().unwrap();

    let shared = ctx.shared.lock();
    let expected_round_hits_min = worker_count * rounds * 5;
    assert!(
        shared.round_hits >= expected_round_hits_min,
        "final round hits too small: {} < {}",
        shared.round_hits,
        expected_round_hits_min,
    );
    println!(
        "concurrency_stress: round_hits={}, critical_sum={}, total_score={}",
        shared.round_hits, shared.critical_sum, total_score
    );
}

#[cfg_attr(feature = "ax-std", unsafe(no_mangle))]
fn main() {
    #[cfg(feature = "ax-std")]
    {
        let mut banner = String::from("Hello, concurrency stress test");
        banner.push('!');
        println!("{banner}");
        run_stress();
    }

    println!("All tests passed!");
}
