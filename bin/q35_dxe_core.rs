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
use patina_dxe_core::Core;
use patina_samples as sc;
use patina_smbios::component::{SmbiosConfiguration, SmbiosProviderManager};
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

#[cfg_attr(target_os = "uefi", unsafe(export_name = "efi_main"))]
pub extern "efiapi" fn _start(physical_hob_list: *const c_void) -> ! {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Trace)).unwrap();
    let adv_logger_component = AdvancedLoggerComponent::<Uart16550>::new(&LOGGER);
    adv_logger_component.init_advanced_logger(physical_hob_list).unwrap();

    #[cfg(feature = "build_debugger")]
    patina_debugger::set_debugger(&DEBUGGER);

    log::info!("DXE Core Platform Binary v{}", env!("CARGO_PKG_VERSION"));

    Core::default()
        .init_memory(physical_hob_list) // We can make allocations now!
        .with_service(patina_ffs_extractors::CompositeSectionExtractor::default())
        .with_component(adv_logger_component)
        .with_component(sc::HelloStruct("World")) // Example of a struct component
        .with_component(sc::GreetingsEnum::Hello("World")) // Example of a struct component (enum)
        .with_component(sc::GreetingsEnum::Goodbye("World")) // Example of a struct component (enum)0
        // Configure SMBIOS for version 3.9
        .with_config(SmbiosConfiguration { major_version: 3, minor_version: 9 })
        .with_component(SmbiosProviderManager::new())
        // Platform component that creates SMBIOS records and publishes the table
        .with_component(q35_services::smbios_platform::Q35SmbiosPlatform::new())
        .with_config(patina_mm::config::MmCommunicationConfiguration {
            acpi_base: patina_mm::config::AcpiBase::Mmio(0x0), // Actual ACPI base address will be set during boot
            cmd_port: patina_mm::config::MmiPort::Smi(0xB2),
            data_port: patina_mm::config::MmiPort::Smi(0xB3),
            comm_buffers: vec![],
        })
        .with_component(q35_services::mm_config_provider::MmConfigurationProvider)
        .with_component(q35_services::mm_control::QemuQ35PlatformMmControl::new())
        .with_component(patina_mm::component::sw_mmi_manager::SwMmiManager::new())
        // The Q35 firmware using the MM Supervisor. Additional support is needeed in the
        // supervisor to support MM communication outside of code that has direct access
        // to C internal state.
        //
        // Tracked in https://github.com/microsoft/mu_feature_mm_supv/issues/541
        //
        // .with_component(patina_mm::component::communicator::MmCommunicator::new())
        // .with_component(q35_services::mm_test::QemuQ35MmTest::new())
        .with_config(patina_performance::config::PerfConfig {
            enable_component: true,
            enabled_measurements: {
                patina::performance::Measurement::DriverBindingStart         // Adds driver binding start measurements.
               | patina::performance::Measurement::DriverBindingStop        // Adds driver binding stop measurements.
               | patina::performance::Measurement::DriverBindingSupport     // Adds driver binding support measurements.
               | patina::performance::Measurement::LoadImage                // Adds load image measurements.
               | patina::performance::Measurement::StartImage // Adds start image measurements.
            },
        })
        .with_component(patina_performance::component::performance_config_provider::PerformanceConfigurationProvider)
        .with_component(patina_performance::component::performance::Performance)
        .start()
        .unwrap();

    log::info!("Dead Loop Time");
    loop {}
}
