//! DXE Core Sample X64 Binary for QEMU Q35
//!
//! ## License
//!
//! Copyright (C) Microsoft Corporation. All rights reserved.
//!
//! SPDX-License-Identifier: BSD-2-Clause-Patent
//!
#![cfg(all(target_os = "uefi", feature = "x64"))]
#![no_std]
#![no_main]

use core::{ffi::c_void, panic::PanicInfo};
use patina_adv_logger::{component::AdvancedLoggerComponent, logger::AdvancedLogger};
use patina_dxe_core::Core;
use patina_samples as sc;
use patina_sdk::component::config as uefi_sdk_configs;
use patina_sdk::component::service as uefi_sdk_services;
use patina_sdk::{log::Format, serial::uart::Uart16550};
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

static DEBUGGER: patina_debugger::UefiDebugger<Uart16550> =
    patina_debugger::UefiDebugger::new(Uart16550::Io { base: 0x3F8 })
        .with_default_config(false, true, 0)
        .with_debugger_logging();

#[cfg_attr(target_os = "uefi", unsafe(export_name = "efi_main"))]
pub extern "efiapi" fn _start(physical_hob_list: *const c_void) -> ! {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Trace)).unwrap();
    let adv_logger_component = AdvancedLoggerComponent::<Uart16550>::new(&LOGGER);
    adv_logger_component.init_advanced_logger(physical_hob_list).unwrap();

    patina_debugger::set_debugger(&DEBUGGER);

    log::info!("DXE Core Platform Binary v{}", env!("CARGO_PKG_VERSION"));

    Core::default()
        .with_section_extractor(patina_section_extractor::CompositeSectionExtractor::default())
        .init_memory(physical_hob_list) // We can make allocations now!
        .with_config(sc::Name("World")) // Config knob for sc::log_hello
        .with_component(adv_logger_component)
        .with_component(sc::log_hello) // Example of a function component
        .with_component(sc::HelloStruct("World")) // Example of a struct component
        .with_component(sc::GreetingsEnum::Hello("World")) // Example of a struct component (enum)
        .with_component(sc::GreetingsEnum::Goodbye("World")) // Example of a struct component (enum)
        .with_config(uefi_sdk_configs::mm::MmCommunicationConfiguration {
            acpi_base: uefi_sdk_configs::mm::AcpiBase::Mmio(0x0), // Actual ACPI base address will be set during boot
            cmd_port: uefi_sdk_configs::mm::MmiPort::Smi(0xB2),
            data_port: uefi_sdk_configs::mm::MmiPort::Smi(0xB3),
            comm_buffers: vec![],
        })
        .with_component(q35_services::mm_config_provider::MmConfigurationProvider)
        .with_component(q35_services::mm_control::QemuQ35PlatformMmControl::new())
        .with_component(uefi_sdk_services::sw_mmi_manager::SwMmiManager::new())
        .with_component(uefi_sdk_services::mm_communicator::MmCommunicator::new())
        .with_component(q35_services::mm_test::QemuQ35MmTest::new())
        .start()
        .unwrap();

    log::info!("Dead Loop Time");
    loop {}
}
