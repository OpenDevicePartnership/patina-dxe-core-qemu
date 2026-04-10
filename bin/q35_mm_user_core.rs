//! Example MM User Core Binary
//!
//! This is an example platform binary that demonstrates how to build a PE/COFF
//! MM User Core using the `patina_mm_user_core` crate. It follows the same pattern
//! as `q35_dxe_core.rs` for the DXE Core.
//!
//! ## Building
//!
//! Build with cargo for the UEFI target:
//! ```bash
//! cargo build --release --target x86_64-unknown-uefi --bin example_mm_user
//! ```
//!
//! ## Entry Point
//!
//! The MM User Core is invoked by the MM Supervisor Core via `invoke_demoted_routine`
//! after being loaded into MMRAM. The supervisor passes three arguments:
//! - `arg1`: Command type (StartUserCore, UserRequest, UserApProcedure)
//! - `arg2`: Command-specific data pointer (HOB list for init, buffer for requests)
//! - `arg3`: Command-specific auxiliary data
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
#![cfg(all(target_os = "uefi", target_arch = "x86_64"))]
#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::sync::atomic::AtomicBool;
use core::ffi::c_void;
use patina::{log::Format, serial::uart::Uart16550, management_mode::supervisor::UserCommandType};
use patina_adv_logger::logger::{ AdvancedLogger, TargetFilter};
use patina_mm_user_core::MmUserCore;

/// Flag indicating that advanced logger initialization is complete.
static ADV_LOGGER_INIT_COMPLETE: AtomicBool = AtomicBool::new(false);

/// The static MM User Core instance.
static USER_CORE: MmUserCore = MmUserCore::new();

static LOGGER: AdvancedLogger<Uart16550> = AdvancedLogger::new(
    Format::Standard,
    &[
        TargetFilter { target: "goblin", log_level: log::LevelFilter::Off, hw_filter_override: None },
        TargetFilter { target: "allocations", log_level: log::LevelFilter::Off, hw_filter_override: None },
        TargetFilter { target: "efi_memory_map", log_level: log::LevelFilter::Off, hw_filter_override: None },
        TargetFilter { target: "mm_comm", log_level: log::LevelFilter::Off, hw_filter_override: None },
        TargetFilter { target: "sw_mmi", log_level: log::LevelFilter::Off, hw_filter_override: None },
        TargetFilter { target: "patina_performance", log_level: log::LevelFilter::Off, hw_filter_override: None },
    ],
    log::LevelFilter::Info,
    Uart16550::Io { base: 0x402 },
);

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

/// The entry point for the MM User Core binary.
///
/// Called by the MM Supervisor via `invoke_demoted_routine` with three arguments:
/// - `arg1`: Command type (0 = StartUserCore, 1 = UserRequest, 2 = UserApProcedure)
/// - `arg2`: Command-specific data (HOB list pointer for init, buffer pointer for requests)
/// - `arg3`: Command-specific auxiliary data (0 for init, context size for requests)
///
/// Returns 0 (`EFI_SUCCESS`) on success, or a non-zero EFI status code on failure.
#[cfg_attr(target_os = "uefi", unsafe(export_name = "user_core_main"))]
pub extern "efiapi" fn mm_user_main(op_code: u64, arg1: u64, arg2: u64) -> u64 {

    // Initialize the advanced logger on the first CPU to arrive (BSP)
    if !ADV_LOGGER_INIT_COMPLETE.swap(true, core::sync::atomic::Ordering::SeqCst) {
        // If this is our first time here, it better be that the op_code being MmUserRequestTypeInit
        if op_code != UserCommandType::StartUserCore as u64 {
            // This means the BSP didn't send the expected init command first, which is a problem.
            // Log an error and return failure.
            panic!("MM User Core received non-init command before initialization: op_code = {}", op_code);
        }

        log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Trace)).unwrap();
        // SAFETY: The physical_hob_list pointer is considered valid at this point as it's provided by the core
        // to the entry point.
        unsafe {
            LOGGER.init(arg1 as *const c_void).unwrap();
        }
    }

    USER_CORE.entry_point_worker(op_code, arg1, arg2)
}
