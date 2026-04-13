<h1 align="center">axplat-cargo</h1>

<p align="center">Manages hardware platform packages using `axplat`</p>

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/axplat-cargo.svg)](https://crates.io/crates/axplat-cargo)
[![Docs.rs](https://docs.rs/axplat-cargo/badge.svg)](https://docs.rs/axplat-cargo)
[![Rust](https://img.shields.io/badge/edition-2024-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](./LICENSE)

</div>

English | [中文](README_CN.md)

# Introduction

`axplat-cargo` provides Manages hardware platform packages using `axplat`. It is maintained as part of the TGOSKits component set and is intended for Rust projects that integrate with ArceOS, AxVisor, or related low-level systems software. The installed binary remains `cargo-axplat`, so `cargo axplat` continues to work.

## Quick Start

### Installation

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
axplat-cargo = "0.4.5"
```

### Run Check and Test

```bash
# Enter the crate directory
cd components/axplat_crates/cargo-axplat

# Format code
cargo fmt --all

# Run clippy
cargo clippy --all-targets --all-features

# Run tests
cargo test --all-features

# Build documentation
cargo doc --no-deps
```

## Integration

### Example

```rust
use cargo_axplat as _;

fn main() {
    // Integrate `axplat-cargo` into your project here.
}
```

### Documentation

Generate and view API documentation:

```bash
cargo doc --no-deps --open
```

Online documentation: [docs.rs/axplat-cargo](https://docs.rs/axplat-cargo)

# Contributing

1. Fork the repository and create a branch
2. Run local format and checks
3. Run local tests relevant to this crate
4. Submit a PR and ensure CI passes

# License

Licensed under the Apache License, Version 2.0. See [LICENSE](./LICENSE) for details.
