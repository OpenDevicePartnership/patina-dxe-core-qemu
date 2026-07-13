//! DXE Core X64 Binary for MsvmPkg
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
use patina::{log::Format, serial::uart::Uart16550};
use patina_adv_logger::{
    component::AdvancedLoggerComponent,
    logger::{AdvancedLogger, TargetFilter},
};
use patina_dxe_core::*;
use patina_ffs_extractors::CompositeSectionExtractor;
use patina_stacktrace::StackTrace;
extern crate alloc;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::error!("{}", info);

    if let Err(err) = unsafe { StackTrace::dump() } {
        log::error!("StackTrace: {}", err);
    }

    patina_debugger::breakpoint();

    loop {}
}

static LOGGER: AdvancedLogger<Uart16550> = AdvancedLogger::new(
    Format::Standard,
    &[
        TargetFilter { target: "goblin", log_level: log::LevelFilter::Off, hw_filter_override: None },
        TargetFilter { target: "gcd_measure", log_level: log::LevelFilter::Off, hw_filter_override: None },
        TargetFilter { target: "allocations", log_level: log::LevelFilter::Off, hw_filter_override: None },
        TargetFilter { target: "efi_memory_map", log_level: log::LevelFilter::Off, hw_filter_override: None },
    ],
    log::LevelFilter::Info,
    Uart16550::Io { base: 0x2F8 },
);

#[cfg(feature = "enable_debugger")]
const _ENABLE_DEBUGGER: bool = true;
#[cfg(not(feature = "enable_debugger"))]
const _ENABLE_DEBUGGER: bool = false;

#[cfg(feature = "build_debugger")]
static DEBUGGER: patina_debugger::PatinaDebugger<Uart16550> =
    patina_debugger::PatinaDebugger::new(Uart16550::Io { base: 0x3F8 })
        .with_force_enable(_ENABLE_DEBUGGER)
        .with_log_policy(patina_debugger::DebuggerLoggingPolicy::FullLogging)
        .with_transport_init();

struct Msvm;

// Default `MemoryInfo` implementation is sufficient for Msvm.
impl MemoryInfo for Msvm {}

impl CpuInfo for Msvm {
    fn perf_timer_frequency() -> Option<u64> {
        None
    }
}

impl ComponentInfo for Msvm {
    fn configs(_add: Add<Config>) {}

    fn components(mut add: Add<Component>) {
        add.component(AdvancedLoggerComponent::<Uart16550>::new(&LOGGER));
    }
}

impl PlatformInfo for Msvm {
    type CpuInfo = Self;
    type MemoryInfo = Self;
    type ComponentInfo = Self;
    type Extractor = CompositeSectionExtractor;
}

static CORE: Core<Msvm> = Core::new(CompositeSectionExtractor::new());

#[cfg_attr(target_os = "uefi", unsafe(export_name = "efi_main"))]
/// # Safety
/// We must take on faith that the physical_hob_list pointer is valid.
pub unsafe extern "efiapi" fn _start(physical_hob_list: *const c_void) -> ! {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Trace)).unwrap();
    // SAFETY: The physical_hob_list pointer is considered valid at this point as it's provided by the previous
    // FW stage.
    unsafe {
        LOGGER.init(physical_hob_list).unwrap();
    }

    #[cfg(feature = "build_debugger")]
    patina_debugger::set_debugger(&DEBUGGER);

    log::info!("DXE Core Platform Binary v{}", env!("CARGO_PKG_VERSION"));
    CORE.entry_point(physical_hob_list)
}
