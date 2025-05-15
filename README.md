# QEMU DXE Core Binaries

## Overview

The main purpose of this repository is to integrate the Rust components and dependencies necessary to build a Rust
DXE Core binary that will be used in QEMU firmware.

Currently, two QEMU platforms are supported. The build commands for each are given below.

Set the `RUSTC_BOOTSTRAP` environment variable to `1` in the terminal used for the build.

- Linux: `export RUSTC_BOOTSTRAP=1`
- Windows (cmd): `set RUSTC_BOOTSTRAP=1`
- Windows (powershell): `$env:RUSTC_BOOTSTRAP=1`

---

- **QEMU Q35**: `cargo make q35`
  - Release build: `cargo make q35-release`
- **QEMU SBSA**: `cargo make sbsa`
  - Release build: `cargo make sbsa-release`

The binaries are produced in the `target` directory.

- **QEMU Q35**: `target/x86_64-unknown-uefi`
- **QEMU SBSA**: `target/aarch64-unknown-uefi`

## Working with Local Dependencies

In your development workflow, you should test your firmware changes on QEMU. You can replace the dependencies in this
repo with your local repo for each dependency to build and test that code.

To do that, follow the [Overriding Dependencies](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html)
section in the Cargo Book. Notice that although the `crates-io` registry is replaced with the `UefiRust` in our repo
in `.cargo/config.toml`, the `crates-io` registry is still patched here similar to the examples in the Cargo Book.

```toml
adv_logger = { version = "7" }
dxe_core = { version = "7" }
log = { version = "^0.4", default-features = false, features = [
    "release_max_level_warn",
] }
sample_components = { version = "7" }
section_extractor = { version = "9" }
uefi_cpu = { version = "9" }
uefi_debugger = { version = "9" }
uefi_sdk = { version = "1" }
```

To produce the following temporary contents in the `Cargo.toml` file:

```toml
adv_logger = { version = "7" }
dxe_core = { version = "7" }
log = { version = "^0.4", default-features = false, features = [
    "release_max_level_warn",
] }
sample_components = { version = "7" }
section_extractor = { version = "9" }
uefi_cpu = { version = "9" }
uefi_debugger = { version = "9" }
uefi_sdk = { version = "1" }

[patch.crates-io]
dxe_core = { path = "../uefi-dxe-core/dxe_core" }
```

## NuGet Publishing Instructions

The NuGet package is currently published to the public [Patina QEMU DXE Core](https://dev.azure.com/patina-fw/artifacts/_artifacts/feed/qemu-dxe-core)
feed where it is consumed in the [Patina QEMU](https://github.com/OpenDevicePartnership/patina-qemu) repository.

The NuGet is built and published using a GitHub workflow in [Patina QEMU DXE Core Actions](https://github.com/OpenDevicePartnership/patina-dxe-core-qemu/actions).
