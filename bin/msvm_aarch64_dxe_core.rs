//! DXE Core AArch64 Binary for MsvmPkg
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
#![cfg(all(target_os = "uefi", feature = "aarch64"))]
#![no_std]
#![no_main]

use core::{ffi::c_void, panic::PanicInfo, sync::atomic::AtomicU64};
use patina::{log::Format, serial::uart::UartPl011};
use patina_adv_logger::{
    component::AdvancedLoggerComponent,
    logger::{AdvancedLogger, TargetFilter},
};
use patina_dxe_core::*;
use patina_ffs_extractors::CompositeSectionExtractor;
use patina_stacktrace::StackTrace;
use qemu_resources::config::MsvmPatinaConfig;
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

static LOGGER: AdvancedLogger<UartPl011> = AdvancedLogger::new(
    Format::Standard,
    &[
        TargetFilter { target: "goblin", log_level: log::LevelFilter::Off, hw_filter_override: None },
        TargetFilter { target: "gcd_measure", log_level: log::LevelFilter::Off, hw_filter_override: None },
        TargetFilter { target: "allocations", log_level: log::LevelFilter::Off, hw_filter_override: None },
        TargetFilter { target: "efi_memory_map", log_level: log::LevelFilter::Off, hw_filter_override: None },
    ],
    log::LevelFilter::Info,
    UartPl011::new(0xEFFEB000),
);

#[cfg(feature = "enable_debugger")]
const _ENABLE_DEBUGGER: bool = true;
#[cfg(not(feature = "enable_debugger"))]
const _ENABLE_DEBUGGER: bool = false;

#[cfg(feature = "build_debugger")]
static DEBUGGER: patina_debugger::PatinaDebugger<UartPl011> =
    patina_debugger::PatinaDebugger::new(UartPl011::new(0xEFFEC000))
        .with_force_enable(_ENABLE_DEBUGGER)
        .with_log_policy(patina_debugger::DebuggerLoggingPolicy::FullLogging)
        .with_transport_init();

// Default to legacy Hyper-V GIC bases
static GICD_BASE: AtomicU64 = AtomicU64::new(0xFFFF0000);
static GICR_BASE: AtomicU64 = AtomicU64::new(0xEFFEE000);
struct Msvm;

// Default `MemoryInfo` implementation is sufficient for Msvm.
impl MemoryInfo for Msvm {}

impl CpuInfo for Msvm {
    fn perf_timer_frequency() -> Option<u64> {
        None
    }

    fn gic_bases() -> GicBases {
        // SAFETY: gicd and gicr bases correctly point to the register spaces.
        // SAFETY: Access to these registers is exclusive to this struct instance.
        unsafe {
            GicBases::new(
                GICD_BASE.load(core::sync::atomic::Ordering::Acquire),
                GICR_BASE.load(core::sync::atomic::Ordering::Acquire),
            )
        }
    }
}

impl ComponentInfo for Msvm {
    fn configs(_add: Add<Config>) {}

    fn components(mut add: Add<Component>) {
        add.component(AdvancedLoggerComponent::<UartPl011>::new(&LOGGER));
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

    // SAFETY: The physical_hob_list pointer is considered valid at this point as it's provided by the previous
    // FW stage.
    if let Ok(config) = unsafe { MsvmPatinaConfig::from_hob_list(physical_hob_list) } {
        GICD_BASE.store(config.gic_distributor_base, core::sync::atomic::Ordering::Release);
        GICR_BASE.store(config.gic_redistributor_base, core::sync::atomic::Ordering::Release);
    }

    log::info!("DXE Core Platform Binary v{}", env!("CARGO_PKG_VERSION"));
    CORE.entry_point(physical_hob_list)
}
