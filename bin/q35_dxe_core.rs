//! DXE Core Sample X64 Binary for QEMU Q35
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
use patina_adv_logger::{component::AdvancedLoggerComponent, logger::AdvancedLogger};
use patina_dxe_core::*;
use patina_ffs_extractors::CompositeSectionExtractor;
use patina_stacktrace::StackTrace;
use qemu_resources::q35::component::service as q35_services;
extern crate alloc;
use alloc::vec;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::error!("{}", info);

    if let Err(err) = unsafe { StackTrace::dump() } {
        log::error!("StackTrace: {}", err);
    }

    if patina_debugger::enabled() {
        patina_debugger::breakpoint();
    }

    loop {}
}

static LOGGER: AdvancedLogger<Uart16550> = AdvancedLogger::new(
    Format::Standard,
    &[
        ("goblin", log::LevelFilter::Off),
        ("gcd_measure", log::LevelFilter::Off),
        ("allocations", log::LevelFilter::Off),
        ("efi_memory_map", log::LevelFilter::Off),
        ("mm_comm", log::LevelFilter::Off),
        ("sw_mmi", log::LevelFilter::Off),
        ("patina_performance", log::LevelFilter::Off),
    ],
    log::LevelFilter::Info,
    Uart16550::Io { base: 0x402 },
);

#[cfg(feature = "enable_debugger")]
const _ENABLE_DEBUGGER: bool = true;
#[cfg(not(feature = "enable_debugger"))]
const _ENABLE_DEBUGGER: bool = false;

#[cfg(feature = "build_debugger")]
static DEBUGGER: patina_debugger::PatinaDebugger<Uart16550> =
    patina_debugger::PatinaDebugger::new(Uart16550::Io { base: 0x3F8 })
        .with_force_enable(_ENABLE_DEBUGGER)
        .with_log_policy(patina_debugger::DebuggerLoggingPolicy::FullLogging);

struct Q35;

// Default `MemoryInfo` implementation is sufficient for Q35.
impl MemoryInfo for Q35 {}

// Default `CpuInfo` implementation is sufficient for Q35.
impl CpuInfo for Q35 {}

impl ComponentInfo for Q35 {
    fn configs(mut add: Add<Config>) {
        add.config(patina_mm::config::MmCommunicationConfiguration {
            acpi_base: patina_mm::config::AcpiBase::Mmio(0x0), // Actual ACPI base address will be set during boot
            cmd_port: patina_mm::config::MmiPort::Smi(0xB2),
            data_port: patina_mm::config::MmiPort::Smi(0xB3),
            enable_comm_buffer_updates: false,
            updatable_buffer_id: None,
            comm_buffers: vec![],
        });
        add.config(patina_performance::config::PerfConfig {
            enable_component: true,
            enabled_measurements: {
                patina::performance::Measurement::DriverBindingStart         // Adds driver binding start measurements.
               | patina::performance::Measurement::DriverBindingStop        // Adds driver binding stop measurements.
               | patina::performance::Measurement::DriverBindingSupport     // Adds driver binding support measurements.
               | patina::performance::Measurement::LoadImage                // Adds load image measurements.
               | patina::performance::Measurement::StartImage // Adds start image measurements.
            },
        })
    }

    fn components(mut add: Add<Component>) {
        add.component(AdvancedLoggerComponent::<Uart16550>::new(&LOGGER));
        add.component(q35_services::mm_config_provider::MmConfigurationProvider);
        add.component(q35_services::mm_control::QemuQ35PlatformMmControl::new());
        add.component(patina_mm::component::sw_mmi_manager::SwMmiManager::new());
        add.component(patina_mm::component::communicator::MmCommunicator::new());
        add.component(q35_services::mm_test::QemuQ35MmTest::new());
        add.component(patina_performance::component::performance_config_provider::PerformanceConfigurationProvider);
        add.component(patina_performance::component::performance::Performance);
    }
}

impl PlatformInfo for Q35 {
    type CpuInfo = Self;
    type MemoryInfo = Self;
    type ComponentInfo = Self;
    type Extractor = CompositeSectionExtractor;
}

static CORE: Core<Q35> = Core::new(CompositeSectionExtractor::new());

#[cfg_attr(target_os = "uefi", unsafe(export_name = "efi_main"))]
pub extern "efiapi" fn _start(physical_hob_list: *const c_void) -> ! {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Trace)).unwrap();
    // SAFETY: The physical_hob_list pointer is considered valid at this point as it's provided by the core
    // to the entry point.
    unsafe {
        LOGGER.init(physical_hob_list).unwrap();
    }

    #[cfg(feature = "build_debugger")]
    patina_debugger::set_debugger(&DEBUGGER);

    log::info!("DXE Core Platform Binary v{}", env!("CARGO_PKG_VERSION"));
    CORE.entry_point(physical_hob_list)
}
