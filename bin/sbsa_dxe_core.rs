//! DXE Core Sample AARCH64 Binary for QEMU SBSA
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

use core::{ffi::c_void, panic::PanicInfo};
use patina::{log::Format, serial::uart::UartPl011};
use patina_adv_logger::{component::AdvancedLoggerComponent, logger::AdvancedLogger};
use patina_dxe_core::{Core, GicBases};
use patina_stacktrace::StackTrace;

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

static LOGGER: AdvancedLogger<UartPl011> = AdvancedLogger::new(
    Format::Standard,
    &[
        ("goblin", log::LevelFilter::Off),
        ("gcd_measure", log::LevelFilter::Off),
        ("allocations", log::LevelFilter::Off),
        ("efi_memory_map", log::LevelFilter::Off),
    ],
    log::LevelFilter::Info,
    UartPl011::new(0x6000_0000),
);

#[cfg(feature = "enable_debugger")]
const _ENABLE_DEBUGGER: bool = true;
#[cfg(not(feature = "enable_debugger"))]
const _ENABLE_DEBUGGER: bool = false;

#[cfg(feature = "build_debugger")]
static DEBUGGER: patina_debugger::PatinaDebugger<UartPl011> =
    patina_debugger::PatinaDebugger::new(UartPl011::new(0x6000_0000)).with_force_enable(_ENABLE_DEBUGGER);

#[cfg_attr(target_os = "uefi", unsafe(export_name = "efi_main"))]
pub extern "efiapi" fn _start(physical_hob_list: *const c_void) -> ! {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Trace)).unwrap();
    let adv_logger_component = AdvancedLoggerComponent::<UartPl011>::new(&LOGGER);
    adv_logger_component.init_advanced_logger(physical_hob_list).unwrap();

    #[cfg(feature = "build_debugger")]
    patina_debugger::set_debugger(&DEBUGGER);

    log::info!("DXE Core Platform Binary v{}", env!("CARGO_PKG_VERSION"));

    Core::default()
        .init_memory(physical_hob_list) // We can make allocations now!
        .with_config(GicBases::new(0x40060000, 0x40080000)) // GIC bases for AArch64
        .with_service(patina_ffs_extractors::CompositeSectionExtractor::default())
        .with_component(adv_logger_component)
        .start()
        .unwrap();

    log::info!("Dead Loop Time");
    loop {}
}
