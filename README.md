# QEMU DXE Core Binaries

## Overview

The main purpose of this repository is to integrate the Rust components and dependencies necessary to build a sample
Rust DXE Core binary that can be used in a QEMU UEFI firmware build.

Currently, two QEMU platforms are supported, Q35 for x64 architecture and SBSA for aarch64 architecture.

## Documentation

The documentation in this repo can be generated with the following commands:

- `cargo make doc` - This will build the documentation for all packages in the workspace.
- `cargo make doc-open` - This will build the documentation for all packages in the workspace and open it in a
  web browser.

## Building

To build an executable, this repo uses the same compiler setup steps that are used in the patina project
[readme.md file build section](https://github.com/OpenDevicePartnership/patina#Build).  Once the compiler is installed,
executing cargo make will create a DXE core .EFI file that can be used as a replacement for the Patina DXE Core EFI
binary in the [patina-qemu](https://github.com/OpenDevicePartnership/patina-qemu) UEFI build.

- Q35 (x64) debug

   ```shell
   Compile Command:  'cargo make q35'
   Output File:      'target/x86_64-unknown-uefi/debug/qemu_q35_dxe_core.efi'
   ```

- Q35 (x64) release

   ```shell
   Compile Command:  'cargo make q35-release'
   Output File:      'target/x86_64-unknown-uefi/release/qemu_q35_dxe_core.efi'
   ```

- SBSA (aarch64) debug

   ```shell
   Compile Command:  'cargo make sbsa'
   Output File:      'target/aarch64-unknown-uefi/debug/qemu_sbsa_dxe_core.efi'
   ```

- SBSA (aarch64) release

   ```shell
   Compile Command:  'cargo make sbsa-release'
   Output File:      'target/aarch64-unknown-uefi/release/qemu_sbsa_dxe_core.efi'
   ```

The [patina_debugger](https://github.com/OpenDevicePartnership/patina/blob/main/docs/src/dxe_core/debugging.md) is
enabled by default on debug builds, but not release builds. It can be enabled on release builds by passing the
`enable_debugger` feature to the build, e.g. `cargo make q35-release --features enable_debugger`. The initial breakpoint
is disabled by default, passing the `initial_breakpoint` feature to the build will enable it.

## Size Comparison

The code in both the C and Rust modules is always changing and depending on the compression algorithm used, size comparisons
can be difficult.  But to give a general idea where current development stands, this repository Q35 build was compiled
both as debug and release, then compared to a Q35 build that contains the normal C based DXE core.

The Patina DXE Core does include support for performance tracing and features that are normally provided by the CpuDxe and
RuntimeDxe drivers.  So the Tiano DXE Core size entries below include compiling with performance tracing enabled and
include the size of the CpuDxe and RuntimeDxe drivers.

### Cargo Bloat

A size breakdown, whether by function or crate, can be analyzed by using `cargo make bloat-q35` or
`cargo make bloat-sbsa`. Optionally, additional arguments can be passed, e.g. to see a crate breakdown:
`cargo make bloat-q35 --crates -n 40`.

### Release Builds

| Compression | Tiano DXE Core Size | Patina DXE Core Size | Difference |
|:---:|:---:|:---:|:---:|
| None | 423,424 | 784,384 | 352.5 KB (185.2%) |
| Tiano | 133,847 | 365,086 | 225.8 KB (272.7%) |
| Lzma | 118,807 | 314,775 | 191.4 KB (264.9%) |
| Brotli | 120,721 | 301,069 | 176.1 KB (249.4%) |

### Debug Builds

| Compression | Tiano DXE Core Size | Patina DXE Core Size | Difference |
|:---:|:---:|:---:|:---:|
| None | 368,128 | 1,619,456 | 1222.0 KB (439.92%) |
| Tiano | 187,076 | 603,400 | 406.6 KB (322.54%) |
| Lzma | 164,637 | 496,764 | 324.3 KB (301.73%) |
| Brotli | 166,570 | 487,425 | 313.3 KB (292.6%) |

## NuGet Publishing

This repository has a GitHub action to build and publish the output .EFI files to a public NuGet package feed
[qemu-dxe-core](https://dev.azure.com/patina-fw/artifacts/_artifacts/feed/qemu-dxe-core).  That feed is then consumed
by the [patina-qemu](https://github.com/OpenDevicePartnership/patina-qemu) repository to demonstrate a UEFI
build that uses the Patina DXE core driver.
