#![cfg_attr(feature = "ax-std", no_std)]
#![cfg_attr(feature = "ax-std", no_main)]

use core::fmt::Write;

#[cfg(feature = "ax-std")]
#[macro_use]
extern crate ax_std as std;

use std::{println, string::String, thread, vec::Vec};

use axvisor_guestlib::{emit_json_result, power_off_or_hang, smp};

const CASE_ID: &str = "cpu.aarch64.mpidr";
const SAMPLES_PER_CPU: usize = 3;

#[derive(Clone, Copy)]
struct CpuRecord {
    logical_cpu_id: usize,
    observed_cpu_id: usize,
    samples: [u64; SAMPLES_PER_CPU],
}

#[cfg(target_arch = "aarch64")]
fn read_mpidr_el1() -> u64 {
    use core::arch::asm;

    let raw: u64;
    unsafe {
        asm!("mrs {value}, MPIDR_EL1", value = out(reg) raw);
    }
    raw
}

#[cfg(not(target_arch = "aarch64"))]
fn read_mpidr_el1() -> u64 {
    0
}

fn read_samples() -> [u64; SAMPLES_PER_CPU] {
    let mut samples = [0u64; SAMPLES_PER_CPU];
    for sample in &mut samples {
        // Read the same register several times on the same CPU to catch
        // instability inside one vCPU before comparing across vCPUs.
        *sample = read_mpidr_el1();
        thread::yield_now();
    }
    samples
}

#[cfg(feature = "ax-std")]
fn probe_cpu(cpu_id: usize) -> CpuRecord {
    smp::pin_current_to_cpu(cpu_id);
    CpuRecord {
        logical_cpu_id: cpu_id,
        observed_cpu_id: smp::current_cpu_id(),
        samples: read_samples(),
    }
}

fn decode_record(record: &CpuRecord) -> String {
    let raw = record.samples[0];
    let aff0 = raw & 0xff;
    let aff1 = (raw >> 8) & 0xff;
    let aff2 = (raw >> 16) & 0xff;
    let aff3 = (raw >> 32) & 0xff;
    let mt = ((raw >> 24) & 0x1) != 0;
    let u = ((raw >> 30) & 0x1) != 0;
    let all_equal = record.samples.iter().all(|sample| *sample == raw);

    format!(
        "{{\"logical_cpu_id\":{},\"observed_cpu_id\":{},\"samples\":[{},{},{}],\"all_equal\":{},\"\
         raw\":{},\"aff0\":{},\"aff1\":{},\"aff2\":{},\"aff3\":{},\"mt\":{},\"u\":{}}}",
        record.logical_cpu_id,
        record.observed_cpu_id,
        record.samples[0],
        record.samples[1],
        record.samples[2],
        all_equal,
        raw,
        aff0,
        aff1,
        aff2,
        aff3,
        mt,
        u
    )
}

#[cfg_attr(feature = "ax-std", unsafe(no_mangle))]
fn main() -> ! {
    println!("Running {}", CASE_ID);

    let cpu_count = smp::cpu_count();
    // Probe each logical CPU independently so the diff can check both the
    // per-CPU MPIDR value and whether the guest actually ran where pinned.
    let mut records = smp::collect_from_each_cpu(cpu_count, probe_cpu);
    records.sort_by_key(|record| record.logical_cpu_id);

    let mut records_json = String::from("[");
    for (index, record) in records.iter().enumerate() {
        if index != 0 {
            records_json.push(',');
        }
        records_json.push_str(&decode_record(record));
    }
    records_json.push(']');

    let unique_raw_count = {
        let mut unique = Vec::new();
        for record in &records {
            let raw = record.samples[0];
            if !unique.contains(&raw) {
                unique.push(raw);
            }
        }
        unique.len()
    };

    let mut diff = String::new();
    write!(
        &mut diff,
        "{{\"cpu_count\":{},\"unique_raw_count\":{},\"records\":{}}}",
        cpu_count, unique_raw_count, records_json
    )
    .unwrap();

    // Emit the result before entering the shutdown path so the runner can
    // capture a stable output even if VM teardown needs extra host cooperation.
    emit_json_result(CASE_ID, "ok", &diff);

    smp::pin_current_to_cpu(0);
    smp::start_secondary_vmexit_keepers(cpu_count);

    power_off_or_hang();
}
