//! `cargo xtask starry user-ebpf` driver for the eBPF userspace programs under
//! `os/StarryOS/user/ebpf/`.
//!
//! Replaces the source-tree `user/musl/Makefile` from `Starry-OS/StarryOS:ebpf-kmod`
//! per workflow §5.3 (no new Makefile; build entrypoint is an xtask subcommand).
//! Each program is its own Cargo workspace with an aya `*-ebpf` sub-crate built
//! via build.rs, so the xtask just shells out to `cargo build --release --target
//! <musl-target>` from each program's directory and propagates the status.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, bail};
use clap::{Args, Subcommand};

use super::Starry;

/// All programs under `os/StarryOS/user/ebpf/`. Order is intentional: the
/// fastest-to-build (`async_test`, single crate) goes first so failures surface
/// early; the heavy aya workspaces follow.
const PROGRAMS: &[&str] = &[
    "async_test",
    "kret",
    "rawtp",
    "mytrace",
    "syscall_ebpf",
    "upb",
    "upb2",
];

#[derive(Args, Debug, Clone)]
pub struct ArgsUserEbpf {
    #[command(subcommand)]
    pub command: UserEbpfCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum UserEbpfCommand {
    /// Cross-compile eBPF userspace programs against a musl target.
    Build(ArgsUserEbpfBuild),
    /// List the known programs (for scripting / CI matrix).
    List,
}

#[derive(Args, Debug, Clone)]
pub struct ArgsUserEbpfBuild {
    /// Build exactly one program (e.g. `kret`). Mutually exclusive with `--all`.
    #[arg(long, value_name = "NAME", conflicts_with = "all")]
    pub program: Option<String>,

    /// Build every program in [`PROGRAMS`].
    #[arg(long, conflicts_with = "program")]
    pub all: bool,

    /// Target architecture (`x86_64` / `aarch64` / `riscv64` / `loongarch64`).
    /// Mapped to the matching `*-unknown-linux-musl` Rust target.
    #[arg(long, default_value = "x86_64")]
    pub arch: String,
}

pub(super) async fn run(starry: &Starry, args: ArgsUserEbpf) -> anyhow::Result<()> {
    match args.command {
        UserEbpfCommand::List => {
            for prog in PROGRAMS {
                println!("{prog}");
            }
            Ok(())
        }
        UserEbpfCommand::Build(build) => build_programs(starry.app.workspace_root(), &build),
    }
}

fn build_programs(workspace_root: &Path, args: &ArgsUserEbpfBuild) -> anyhow::Result<()> {
    let target = musl_target_for_arch(&args.arch)?;
    let programs = resolve_program_list(args)?;
    let user_dir = workspace_root.join("os/StarryOS/user/ebpf");

    for program in &programs {
        let program_dir = user_dir.join(program);
        if !program_dir.is_dir() {
            bail!(
                "missing program directory `{}` (expected per workspace §1.3 mapping)",
                program_dir.display()
            );
        }
        run_cargo_build(&program_dir, target)?;
    }

    Ok(())
}

fn resolve_program_list(args: &ArgsUserEbpfBuild) -> anyhow::Result<Vec<String>> {
    if args.all {
        return Ok(PROGRAMS.iter().map(|p| (*p).to_string()).collect());
    }
    match args.program.as_deref() {
        Some(name) => {
            if !PROGRAMS.contains(&name) {
                bail!(
                    "unknown program `{name}`; expected one of: {}",
                    PROGRAMS.join(", ")
                );
            }
            Ok(vec![name.to_string()])
        }
        None => bail!("either `--program <NAME>` or `--all` must be provided"),
    }
}

fn musl_target_for_arch(arch: &str) -> anyhow::Result<&'static str> {
    Ok(match arch {
        "x86_64" => "x86_64-unknown-linux-musl",
        "aarch64" => "aarch64-unknown-linux-musl",
        "riscv64" | "riscv64gc" => "riscv64gc-unknown-linux-musl",
        "loongarch64" => "loongarch64-unknown-linux-musl",
        other => bail!(
            "unsupported arch `{other}`; supported: x86_64, aarch64, riscv64, loongarch64"
        ),
    })
}

fn run_cargo_build(program_dir: &Path, target: &str) -> anyhow::Result<()> {
    println!(
        "==> cargo build --release --target {} (in {})",
        target,
        program_dir.display()
    );

    // Each program is its own Cargo workspace (members live under
    // <program>/, <program>-common/, <program>-ebpf/). Per their root
    // Cargo.toml, `default-members = ["<program>", "<program>-common"]`, so
    // a plain `cargo build` from the program dir is sufficient — aya-build
    // pulls in `<program>-ebpf` through build.rs.
    let status = Command::new(cargo_bin())
        .arg("build")
        .arg("--release")
        .arg("--target")
        .arg(target)
        .current_dir(program_dir)
        .status()
        .with_context(|| format!("failed to spawn cargo in {}", program_dir.display()))?;

    if !status.success() {
        bail!(
            "cargo build failed for {} (target {target}): {status}",
            program_dir.display()
        );
    }
    Ok(())
}

fn cargo_bin() -> PathBuf {
    std::env::var_os("CARGO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("cargo"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programs_list_matches_user_ebpf_dirs() {
        // Keep the const in sync with the on-disk layout. If anyone adds a new
        // program, this list must be updated explicitly (we don't auto-discover
        // because the directory structure may also contain shared `.cargo/`,
        // README, etc.).
        let expected = [
            "async_test",
            "kret",
            "rawtp",
            "mytrace",
            "syscall_ebpf",
            "upb",
            "upb2",
        ];
        assert_eq!(PROGRAMS, expected);
    }

    #[test]
    fn arch_mapping_known() {
        assert_eq!(
            musl_target_for_arch("x86_64").unwrap(),
            "x86_64-unknown-linux-musl"
        );
        assert_eq!(
            musl_target_for_arch("aarch64").unwrap(),
            "aarch64-unknown-linux-musl"
        );
        assert_eq!(
            musl_target_for_arch("riscv64").unwrap(),
            "riscv64gc-unknown-linux-musl"
        );
        assert_eq!(
            musl_target_for_arch("riscv64gc").unwrap(),
            "riscv64gc-unknown-linux-musl"
        );
        assert_eq!(
            musl_target_for_arch("loongarch64").unwrap(),
            "loongarch64-unknown-linux-musl"
        );
        assert!(musl_target_for_arch("ppc64").is_err());
    }

    #[test]
    fn resolve_requires_program_or_all() {
        let args = ArgsUserEbpfBuild {
            program: None,
            all: false,
            arch: "x86_64".to_string(),
        };
        assert!(resolve_program_list(&args).is_err());
    }

    #[test]
    fn resolve_rejects_unknown_program() {
        let args = ArgsUserEbpfBuild {
            program: Some("not-a-program".to_string()),
            all: false,
            arch: "x86_64".to_string(),
        };
        assert!(resolve_program_list(&args).is_err());
    }

    #[test]
    fn resolve_all_returns_all_programs() {
        let args = ArgsUserEbpfBuild {
            program: None,
            all: true,
            arch: "x86_64".to_string(),
        };
        let list = resolve_program_list(&args).unwrap();
        assert_eq!(list.len(), PROGRAMS.len());
    }
}
