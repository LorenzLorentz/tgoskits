fn main() {
    println!("cargo:rerun-if-env-changed=AXVISOR_SMP");
    println!("cargo:rerun-if-changed=linker.lds.S");

    let mut smp = 1;
    if let Ok(s) = std::env::var("AXVISOR_SMP") {
        smp = s.parse::<usize>().unwrap_or(1);
    }

    let ld_content = include_str!("linker.lds.S");
    let ld_content = ld_content.replace("%ARCH%", "riscv");
    let ld_content = ld_content.replace(
        "%KERNEL_BASE%",
        &format!("{:#x}", 0xffff_ffc0_8020_0000usize),
    );
    let ld_content = ld_content.replace("%SMP%", &format!("{smp}",));

    // target/<target_triple>/<mode>/build/axvisor-xxxx/out
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_dir = std::path::Path::new(&out_dir);
    let linker_name = "linker.x";
    let out_path = out_dir.join(linker_name);
    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rustc-link-arg=-T{linker_name}");
    std::fs::write(&out_path, &ld_content).unwrap();

    // Keep a stable copy under target/<target_triple>/<mode>/ for callers that
    // still expect the linker script outside the build-script OUT_DIR.
    let target_dir = out_dir.join("../../..");
    std::fs::write(target_dir.join(linker_name), ld_content).unwrap();
}
