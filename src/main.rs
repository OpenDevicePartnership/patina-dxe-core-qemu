//! DXE Core Sample AARCH64 Binary
//!
//! ## License
//!
//! Copyright (C) Microsoft Corporation. All rights reserved.
//!
//! SPDX-License-Identifier: BSD-2-Clause-Patent
//!
#![no_std]
#![no_main]

use core::{ffi::c_void, panic::PanicInfo};
use patina_adv_logger::{component::AdvancedLoggerComponent, logger::AdvancedLogger};
use patina_dxe_core::{Core, GicBases};
mod uart_debug;

///
///  Platform specific configuration
/// 

// MMIO base address for the UART
const UART_BASE: usize = 0x16A00000;

// GIC distributor base set to the same value as gArmTokenSpaceGuid.PcdGicDistributorBase
const GIC_DISTRIBUTOR_BASE: u64 = 0x06800000;

// GIC redistributors base set to the same value as gArmTokenSpaceGuid.PcdGicRedistributorsBase
const GIC_REDISTRIBUTORS_BASE: u64 = 0x06880000;

///
/// Rust panic handler
/// 

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::error!("{}", info);
    loop {}
}

///
///  Logger setup
/// 

static LOGGER: AdvancedLogger<uart_debug::Uart> = AdvancedLogger::new(
    patina_sdk::log::Format::Standard,
    &[
        ("goblin", log::LevelFilter::Off),
        ("gcd_measure", log::LevelFilter::Off),
        ("allocations", log::LevelFilter::Off),
        ("efi_memory_map", log::LevelFilter::Off),
    ],
    log::LevelFilter::Trace,
    uart_debug::Uart::new(UART_BASE),
);

static DEBUGGER: patina_debugger::UefiDebugger<uart_debug::Uart> =
    patina_debugger::UefiDebugger::new(uart_debug::Uart::new(UART_BASE))
    .with_default_config(false, true, 0);

///
///  Primary entry point for the DXE Core
/// 

#[cfg_attr(target_os = "uefi", unsafe(export_name = "efi_main"))]
pub extern "efiapi" fn _start(physical_hob_list: *const c_void) -> ! {

    // Implement logger
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Trace))
        .unwrap();
    let adv_logger_component = AdvancedLoggerComponent::<uart_debug::Uart>::new(&LOGGER);
    adv_logger_component
        .init_advanced_logger(physical_hob_list)
        .unwrap();

    patina_debugger::set_debugger(&DEBUGGER);

    log::info!("Rust DXE Core entry ...");

    // Call the DXE core
    Core::default()
        .with_section_extractor(patina_section_extractor::CompositeSectionExtractor::default())
        .init_memory(physical_hob_list) // We can make allocations now!
        .with_config(GicBases::new(GIC_DISTRIBUTOR_BASE, GIC_REDISTRIBUTORS_BASE))
        .with_component(adv_logger_component)
        .start()
        .unwrap();

    // DXE core should never return
    log::info!("DXE core returned unexpectedly");
    loop {}
}

