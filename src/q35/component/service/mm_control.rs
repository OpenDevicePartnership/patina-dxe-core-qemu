//! QEMU Q35 Platform Management Mode (MM) Control
//!
//! Provides platform-specific MM control functionality for the QEMU Q35 platform.
//!
//! ## License
//!
//! Copyright (C) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
#![cfg(all(target_os = "uefi", target_arch = "x86_64", feature = "x64"))]

use patina_mm::{config::MmCommunicationConfiguration, service::platform_mm_control::PlatformMmControl};

use crate::q35::registers as register;
use patina::component::{Storage, component, service::IntoService};

use x86_64::instructions::port::Port;

/// The QEMU Q35 platform-specific MM control component.
///
/// This component is responsible for initializing and controlling the MM environment on the QEMU Q35 platform. All
/// QEMU Q35-specific logic for initializing the hardware environment for MM should be contained within this component.
#[derive(IntoService, Default)]
#[service(dyn PlatformMmControl)]
pub struct QemuQ35PlatformMmControl {
    inner_config: MmCommunicationConfiguration,
}

#[component]
impl QemuQ35PlatformMmControl {
    /// Creates a new instance of the QEMU Q35 platform MM control component.
    ///
    /// This function initializes the component with default values. It is equivalent to calling
    /// `QemuQ35PlatformMmControl::default()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Entry point for the QEMU Q35 platform MM control component.
    ///
    /// Installs an instance of the `PlatformMmControl` service that can be invoked by other components that depend
    /// upon hardware initialization for MMI control.
    pub fn entry_point(mut self, storage: &mut Storage) -> patina::error::Result<()> {
        log::debug!("Platform MM Control Entry Point");

        self.inner_config = storage
            .get_config::<MmCommunicationConfiguration>()
            .expect("Failed to get MM Configuration Config from storage")
            .clone();
        log::debug!("PMBASE I/O Port (from config): {:?}", self.inner_config.acpi_base);

        storage.add_service(self);

        Ok(())
    }
}

impl PlatformMmControl for QemuQ35PlatformMmControl {
    /// Initializes QEMU Q35 for Management Mode (MM).
    ///
    /// After this function completes, the platform hardware enabling required to support MMIs is completed.
    fn init(&self) -> patina::error::Result<()> {
        log::debug!("Performing platform-specific MM init...");

        let mut smi_en_port =
            Port::new(self.inner_config.acpi_base.get_io_value() + register::ich9::PMBASE_OFS_SMI_EN as u16);
        let smi_enable_val: u32 = unsafe { smi_en_port.read() };

        // On Q35, the SMI_EN bit should be set already if Standalone MM was launched in PEI.
        if smi_enable_val & register::ich9::SMI_EN_APMC_EN != 0 {
            assert_eq!(
                smi_enable_val & register::ich9::SMI_EN_GBL_SMI_EN,
                register::ich9::SMI_EN_GBL_SMI_EN,
                "SMI Enable bit not set"
            );
        }

        // In any case, set the SMI_EN bit to enable SMI generation.
        let smi_enable_val = smi_enable_val | register::ich9::SMI_EN_APMC_EN | register::ich9::SMI_EN_GBL_SMI_EN;
        unsafe { smi_en_port.write(smi_enable_val) };

        // Set the SMI Lock bit in the PM1A_CNT register to lock the SMI_EN bits
        let pm1a_cnt: *mut u16 = (register::PCI_EXPRESS_BASE_ADDRESS as usize
            + patina::pci_address!(0, 0x1F, 0, register::ich9::GEN_PMCON_1) as usize)
            as *mut u16;
        let mut pm1a_cnt_val: u16 = unsafe { core::ptr::read_volatile(pm1a_cnt) };
        pm1a_cnt_val |= register::ich9::GEN_PMCON_1_SMI_LOCK;
        unsafe { core::ptr::write_volatile(pm1a_cnt, pm1a_cnt_val) };

        Ok(())
    }
}
