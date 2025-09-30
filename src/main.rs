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
use patina_adv_logger::logger::AdvancedLogger;
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
        ("goblin", log::LevelFilter::Info),
        ("gcd_measure", log::LevelFilter::Info),
        ("allocations", log::LevelFilter::Info),
        ("efi_memory_map", log::LevelFilter::Info),
    ],
    log::LevelFilter::Info,
    uart_debug::Uart::new(UART_BASE),
);


///
///  Primary entry point for the DXE Core
/// 

#[cfg_attr(target_os = "uefi", unsafe(export_name = "efi_main"))]
pub extern "efiapi" fn _start(physical_hob_list: *const c_void) -> ! {

    // Implement logger
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Trace))
        .unwrap();

    log::info!("Rust DXE Core entry ...");

    // Call the DXE core
    Core::default()
        .init_memory(physical_hob_list) // We can make allocations now!
        .with_service(patina_ffs_extractors::CompositeSectionExtractor::default())
        .with_config(GicBases::new(GIC_DISTRIBUTOR_BASE, GIC_REDISTRIBUTORS_BASE))
        .start()
        .unwrap();

    // DXE core should never return
    log::info!("DXE core returned unexpectedly");
    loop {}
}

