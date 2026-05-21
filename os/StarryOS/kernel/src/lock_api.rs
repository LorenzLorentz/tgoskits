//! A kernel wrapper around `kspin::SpinNoPreempt` exposing the `lock_api::RawMutex`
//! API used by the `kprobe` / `ktracepoint` ecosystem (e.g. `Kretprobe`,
//! `Uprobe`, `KretprobeBuilder`).
//!
//! tgoskits' `#805` already defines [`crate::kprobe::KernelRawMutex`] for
//! kprobe internals; `KSpinNoPreempt<T>` here is the generic, value-holding
//! counterpart that matches the type the upstream `kbpf-basic` / `kprobe`
//! crates expect when they want a `lock_api::RawMutex` parameterized over
//! `()` (e.g. `KretprobeBuilder<KSpinNoPreempt<()>>`).
use ax_kspin::{SpinNoPreempt, SpinNoPreemptGuard};
use kernel_guard::{BaseGuard, NoPreempt};

/// A `SpinNoPreempt`-backed lock that exposes the `lock_api::RawMutex`
/// interface used by `kprobe` / `ktracepoint` generic types.
pub struct KSpinNoPreempt<T>(SpinNoPreempt<T>);

impl<T> KSpinNoPreempt<T> {
    /// Creates a new `KSpinNoPreempt`.
    pub const fn new(data: T) -> Self {
        KSpinNoPreempt(SpinNoPreempt::new(data))
    }

    /// Locks the spinlock and returns a guard.
    pub fn lock(&self) -> SpinNoPreemptGuard<'_, T> {
        self.0.lock()
    }

    /// Attempts to lock without blocking.
    pub fn try_lock(&self) -> Option<SpinNoPreemptGuard<'_, T>> {
        self.0.try_lock()
    }

    /// Whether the lock is currently held.
    pub fn is_locked(&self) -> bool {
        self.0.is_locked()
    }
}

// SAFETY: `SpinNoPreempt<()>` upholds `RawMutex`'s exclusion guarantees; we
// disable preemption on lock and re-enable it on unlock so callers cannot be
// preempted while holding the lock.
unsafe impl lock_api::RawMutex for KSpinNoPreempt<()> {
    type GuardMarker = lock_api::GuardSend;

    const INIT: Self = KSpinNoPreempt(SpinNoPreempt::new(()));

    fn lock(&self) {
        core::mem::forget(self.0.lock());
    }

    fn try_lock(&self) -> bool {
        self.0.try_lock().map(core::mem::forget).is_some()
    }

    unsafe fn unlock(&self) {
        unsafe { self.0.force_unlock() };
        NoPreempt::release(());
    }

    fn is_locked(&self) -> bool {
        self.0.is_locked()
    }
}
