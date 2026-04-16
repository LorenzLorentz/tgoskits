#![cfg_attr(feature = "ax-std", no_std)]

#[cfg(feature = "ax-std")]
extern crate ax_std as std;

use std::println;

#[cfg(feature = "smp")]
pub mod smp;
#[cfg(feature = "sysreg-sctlr")]
pub mod sysreg;
#[cfg(feature = "user-probe")]
pub mod user_probe;

pub const RESULT_BEGIN_MARKER: &str = "AXTEST_RESULT_BEGIN";
pub const RESULT_END_MARKER: &str = "AXTEST_RESULT_END";

// Emit one structured result record delimited by stable markers so the runner
// can extract the payload from mixed guest console output.
pub fn emit_json_result(case_id: &str, status: &str, diff_json: &str) {
    println!("{RESULT_BEGIN_MARKER}");
    println!(
        "{{\"case_id\":\"{}\",\"status\":\"{}\",\"diff\":{}}}",
        case_id, status, diff_json
    );
    println!("{RESULT_END_MARKER}");
}

pub fn power_off_or_hang() -> ! {
    #[cfg(feature = "ax-std")]
    {
        use std::os::arceos::modules::ax_hal;
        // Under ax-std we request guest poweroff so the host can move on to
        // the next case. Bare-metal-style guests fall back to a local spin.
        ax_hal::power::system_off();
    }

    #[cfg(not(feature = "ax-std"))]
    loop {
        core::hint::spin_loop();
    }
}
