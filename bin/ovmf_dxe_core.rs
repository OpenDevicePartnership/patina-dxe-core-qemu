//! DXE Core Sample X64 Binary for the Open Virtual Machine Firmware (OVMF) platform.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
#![cfg(all(target_os = "uefi", feature = "x64"))]
#![no_std]
#![no_main]

use core::{ffi::c_void, panic::PanicInfo};
use patina::{
    log::{Format, SerialLogger},
    serial::uart::Uart16550,
};
use patina_dxe_core::*;
use patina_ffs_extractors::CompositeSectionExtractor;
use patina_stacktrace::StackTrace;
use qemu_resources::q35::timer;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::error!("{}", info);

    if let Err(err) = unsafe { StackTrace::dump() } {
        log::error!("StackTrace: {}", err);
    }

    patina_debugger::breakpoint();

    loop {}
}

static LOGGER: SerialLogger<Uart16550> = SerialLogger::new(
    Format::Standard,
    &[
        ("allocations", log::LevelFilter::Off),
        ("efi_memory_map", log::LevelFilter::Off),
        ("gcd_measure", log::LevelFilter::Off),
        ("goblin", log::LevelFilter::Off),
    ],
    log::LevelFilter::Info,
    Uart16550::Io { base: 0x402 },
);

const PM_TIMER_PORT: u16 = 0x608;
const _ENABLE_DEBUGGER: bool = cfg!(feature = "enable_debugger");

#[cfg(feature = "build_debugger")]
static DEBUGGER: patina_debugger::PatinaDebugger<Uart16550> =
    patina_debugger::PatinaDebugger::new(Uart16550::Io { base: 0x3F8 })
        .with_force_enable(_ENABLE_DEBUGGER)
        .with_log_policy(patina_debugger::DebuggerLoggingPolicy::FullLogging);

struct OVMF;

// Default `MemoryInfo` implementation is sufficient for OVMF.
impl MemoryInfo for OVMF {}

// OVMF should use TSC frequency calibrated from ACPI PM Timer.
impl CpuInfo for OVMF {
    fn perf_timer_frequency() -> Option<u64> {
        // SAFETY: Reading from the PM Timer I/O port is safe as long as the port is valid.
        // On OVMF, the PM Timer is always available at the specified port address.
        Some(unsafe { timer::calibrate_tsc_frequency(PM_TIMER_PORT) })
    }
}

impl ComponentInfo for OVMF {
    fn configs(_add: Add<Config>) {
        // Add components and configs later
    }

    fn components(_add: Add<Component>) {
        // Add components and configs later
    }
}

impl PlatformInfo for OVMF {
    type CpuInfo = Self;
    type MemoryInfo = Self;
    type ComponentInfo = Self;
    type Extractor = CompositeSectionExtractor;
}

static CORE: Core<OVMF> = Core::new(CompositeSectionExtractor::new());

#[cfg_attr(target_os = "uefi", unsafe(export_name = "efi_main"))]
pub extern "efiapi" fn _start(physical_hob_list: *const c_void) -> ! {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Trace)).unwrap();

    #[cfg(feature = "build_debugger")]
    patina_debugger::set_debugger(&DEBUGGER);

    log::info!("DXE Core Platform Binary v{}", env!("CARGO_PKG_VERSION"));
    CORE.entry_point(physical_hob_list)
}
