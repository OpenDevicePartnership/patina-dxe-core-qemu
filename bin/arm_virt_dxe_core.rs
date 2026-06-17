//! DXE Core Sample AARCH64 Binary for QEMU Arm Virt
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
use patina::{
    log::Format,
    serial::{uart::UartPl011, virtio::VirtioSerial},
};
use patina_adv_logger::{
    component::AdvancedLoggerComponent,
    logger::{AdvancedLogger, TargetFilter},
};
use patina_dxe_core::*;
use patina_ffs_extractors::CompositeSectionExtractor;
use patina_stacktrace::StackTrace;
#[cfg(feature = "exit_on_patina_test_failure")]
use qemu_exit::QEMUExit;
use qemu_resources::armvirt::component::service as armvirt_services;
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
    UartPl011::new(PL011_UART_BASE),
);

#[cfg(feature = "enable_debugger")]
const _ENABLE_DEBUGGER: bool = true;
#[cfg(not(feature = "enable_debugger"))]
const _ENABLE_DEBUGGER: bool = false;

/// Base address of the PL011 UART on the QEMU Arm Virt machine.
const PL011_UART_BASE: usize = 0x0900_0000;

/// Base address of the GIC distributor on the QEMU Arm Virt machine.
const GICD_BASE: u64 = 0x0800_0000;

/// Base address of the GIC redistributor on the QEMU Arm Virt machine.
const GICR_BASE: u64 = 0x080A_0000;

/// The virtio-serial-mmio base address used. This is the only device
/// so it's the last of 32 0x200 byte blocks start at 0xA000000.
const VIRTIO_SERIAL_MMIO: usize = 0x0A00_0000 + (31 * 0x200);

#[cfg(feature = "build_debugger")]
// SAFETY: VIRTIO_SERIAL_MMIO is a valid address for the virtio-mmio device.
static DEBUGGER: patina_debugger::PatinaDebugger<VirtioSerial> =
    patina_debugger::PatinaDebugger::new(unsafe { VirtioSerial::new(VIRTIO_SERIAL_MMIO) })
        .with_transport_init() // Transport init required for virtio
        .with_force_enable(_ENABLE_DEBUGGER);

struct ArmVirt;

// Default `MemoryInfo` implementation is sufficient for Arm Virt.
impl MemoryInfo for ArmVirt {}

impl CpuInfo for ArmVirt {
    fn gic_bases() -> GicBases {
        // SAFETY: gicd and gicr bases correctly point to the register spaces.
        // SAFETY: Access to these registers is exclusive to this struct instance.
        unsafe { GicBases::new(GICD_BASE, GICR_BASE) }
    }
}

impl ComponentInfo for ArmVirt {
    fn components(mut add: Add<Component>) {
        add.component(AdvancedLoggerComponent::<UartPl011>::new(&LOGGER));
        add.component(patina_smbios::component::SmbiosProvider::new(3, 9));
        add.component(armvirt_services::smbios_platform::ArmVirtSmbiosPlatform::new());
        add.component(patina_test::component::TestRunner::default().with_callback(|test_name, err_msg| {
            log::error!("Test {} failed: {}", test_name, err_msg);
            #[cfg(feature = "exit_on_patina_test_failure")]
            qemu_exit::AArch64::new().exit_failure();
        }));
        add.component(patina_performance::component::Performance::new().with_measurements(
            patina::performance::Measurement::DriverBindingStart     // Adds driver binding start measurements.
               | patina::performance::Measurement::DriverBindingStop // Adds driver binding stop measurements.
               | patina::performance::Measurement::LoadImage         // Adds load image measurements.
               | patina::performance::Measurement::StartImage, // Adds start image measurements.
        ));
        add.component(patina_acpi::component::AcpiComponent::default());
    }

    fn configs(_add: Add<Config>) {}
}

impl PlatformInfo for ArmVirt {
    type CpuInfo = Self;
    type MemoryInfo = Self;
    type ComponentInfo = Self;
    type Extractor = CompositeSectionExtractor;
}

static CORE: Core<ArmVirt> = Core::new(CompositeSectionExtractor::new());

#[cfg_attr(target_os = "uefi", unsafe(export_name = "efi_main"))]
/// # Safety
/// We must take on faith that the physical_hob_list pointer is valid.
pub unsafe extern "efiapi" fn _start(physical_hob_list: *const c_void) -> ! {
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
