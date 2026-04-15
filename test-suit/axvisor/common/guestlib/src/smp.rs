use std::{
    os::arceos::{
        api::task::{AxCpuMask, ax_set_current_affinity},
        modules::ax_hal::percpu::this_cpu_id,
    },
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    vec::Vec,
};

pub fn cpu_count() -> usize {
    thread::available_parallelism().unwrap().get()
}

pub fn current_cpu_id() -> usize {
    this_cpu_id()
}

pub fn pin_current_to_cpu(cpu_id: usize) {
    assert!(
        ax_set_current_affinity(AxCpuMask::one_shot(cpu_id)).is_ok(),
        "failed to pin current task to CPU {cpu_id}"
    );

    // Affinity change and actual migration are not the same event. Wait until
    // the current task is really executing on the requested CPU.
    for _ in 0..256 {
        if this_cpu_id() == cpu_id {
            return;
        }
        thread::yield_now();
    }

    assert_eq!(
        this_cpu_id(),
        cpu_id,
        "task did not migrate to CPU {cpu_id}"
    );
}

pub fn wait_until(counter: &AtomicUsize, expected: usize) {
    while counter.load(Ordering::Acquire) != expected {
        thread::yield_now();
    }
}

// Run one probe per visible CPU and collect the per-CPU results in spawn order.
pub fn collect_from_each_cpu<T, F>(cpu_count: usize, probe: F) -> Vec<T>
where
    T: Send + 'static,
    F: Fn(usize) -> T + Send + Sync + 'static,
{
    let probe = Arc::new(probe);
    let mut handles = Vec::with_capacity(cpu_count);

    for cpu_id in 0..cpu_count {
        let probe = probe.clone();
        handles.push(thread::spawn(move || probe(cpu_id)));
    }

    handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect()
}

// Keep secondary CPUs periodically vmexit-ing after the main CPU has emitted
// the final result. This is a test-environment teardown helper, not a generic
// guest/hypervisor ABI primitive.
pub fn start_secondary_vmexit_keepers(cpu_count: usize) {
    if cpu_count <= 1 {
        return;
    }

    let ready = Arc::new(AtomicUsize::new(0));

    for cpu_id in 1..cpu_count {
        let ready = ready.clone();
        thread::spawn(move || {
            pin_current_to_cpu(cpu_id);
            ready.fetch_add(1, Ordering::Release);

            loop {
                trigger_vmexit_hint();
                thread::yield_now();
            }
        });
    }

    wait_until(&ready, cpu_count - 1);
}

#[cfg(target_arch = "aarch64")]
fn trigger_vmexit_hint() {
    use core::arch::asm;

    const PSCI_0_2_FN_VERSION: usize = 0x8400_0000;

    // A benign PSCI SMC is enough to force a trap back into the hypervisor.
    unsafe {
        asm!(
            "smc #0",
            inout("x0") PSCI_0_2_FN_VERSION => _,
            in("x1") 0usize,
            in("x2") 0usize,
            in("x3") 0usize,
        );
    }
}

#[cfg(not(target_arch = "aarch64"))]
fn trigger_vmexit_hint() {}
