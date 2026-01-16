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
use patina_dxe_core::*;
use patina_ffs_extractors::CompositeSectionExtractor;
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

struct Sbsa;

// Default `MemoryInfo` implementation is sufficient for SBSA.
impl MemoryInfo for Sbsa {}

impl CpuInfo for Sbsa {
    fn gic_bases() -> GicBases {
        // SAFETY: gicd and gicr bases correctly point to the register spaces.
        // SAFETY: Access to these registers is exclusive to this struct instance.
        unsafe { GicBases::new(0x40060000, 0x40080000) }
    }
}

impl ComponentInfo for Sbsa {
    fn components(mut add: Add<Component>) {
        add.component(AdvancedLoggerComponent::<UartPl011>::new(&LOGGER));
        add.component(patina::test::TestRunner::default().with_callback(|test_name, err_msg| {
            log::error!("Test {} failed: {}", test_name, err_msg);
        }));
    }
}

impl PlatformInfo for Sbsa {
    type CpuInfo = Self;
    type MemoryInfo = Self;
    type ComponentInfo = Self;
    type Extractor = CompositeSectionExtractor;
}

static CORE: Core<Sbsa> = Core::new(CompositeSectionExtractor::new());

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

/// Stack canary value generated at build time with entropy from compile-time constants.
/// This value is checked by __security_check_cookie to detect stack buffer overflows.
#[unsafe(no_mangle)]
static __security_cookie: usize = {
    const SEED1: u128 = include_bytes!("q35_dxe_core.rs").len() as u128;
    const SEED2: u128 = line!() as u128;
    const SEED3: u128 = column!() as u128;
    
    // Combine compile-time constants to create pseudo-random value
    const COOKIE: u128 = SEED1
        .wrapping_mul(0x517cc1b727220a95)
        .wrapping_add(SEED2.wrapping_mul(0x6c5895682394bad5))
        .wrapping_add(SEED3.wrapping_mul(0x9e3779b97f4a7c15))
        ^ 0xDEADBEEFCAFEBABE;
    
    COOKIE as usize
};

/// Security check function called by the compiler when stack protection is enabled.
/// This function validates that the stack canary has not been corrupted.
/// If corruption is detected, it will panic to prevent potential exploits.
///
/// # Safety
/// This function is called automatically by compiler-generated code.
/// It should never be called directly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __security_check_cookie(cookie: usize) {
    if cookie != __security_cookie {
        panic!("Stack corruption detected! Cookie: {:#x}, Expected: {:#x}", cookie, __security_cookie);
    }
}