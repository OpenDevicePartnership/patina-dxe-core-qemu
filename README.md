# QEMU DXE Core Binaries

## Overview

The main purpose of this repository is to integrate the Rust components and dependencies necessary to build a sample
Rust DXE Core binary that can be used in QEMU UEFI firmware build.

Currently, two QEMU platforms are supported, Q35 for x64 architecture and SBSA for aarch64 architecture.

To build an executable, this repo uses the same compiler setup steps that are used in the patina project
[readme.md file build section](https://github.com/OpenDevicePartnership/patina#Build).  Once the compiler is installed,
executing cargo make will output a DXE core .EFI file that is a direct replacement for the dxe core driver in the
[patina-qemu](https://github.com/OpenDevicePartnership/patina-qemu) UEFI build.

   - Q35 (x64) debug

      ```
      Compile Command:  'cargo make q35'
      Output File:      'target/x86_64-unknown-uefi/debug/qemu_q35_dxe_core.efi'
      ```
      
   - Q35 (x64) release

      ```
      Compile Command:  'cargo make q35-release'
      Output File:      'target/x86_64-unknown-uefi/release/qemu_q35_dxe_core.efi'
      ```

   - SBSA (aarch64) debug

      ```
      Compile Command:  'cargo make sbsa'
      Output File:      'target/aarch64-unknown-uefi/debug/qemu_sbsa_dxe_core.efi'
      ```

   - SBSA (aarch64) release

      ```
      Compile Command:  'cargo make sbsa-release'
      Output File:      'target/aarch64-unknown-uefi/release/qemu_sbsa_dxe_core.efi'
      ```

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
