//! Management Mode (MM) Configuration Provider
//!
//! Produces MM configuration for QEMU Q35 that can be consumed by other components.
//!
//! ## License
//!
//! Copyright (C) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use patina::component::{
    component,
    hob::{FromHob, Hob},
    params::ConfigMut,
};
use patina_mm::config::{CommunicateBuffer, MmCommunicationConfiguration};

use crate::q35::registers as register;

extern crate alloc;

/// Responsible for providing MM configuration information to other components. All other MM related components
/// should be abstracted from MM details by the configuration produced by this component.
pub struct MmConfigurationProvider;

/// Represents a MM Communication Region.
///
/// Most platforms allocate a single or small number of MM Communication Regions. Each region is a set of pages
/// that are used for MM communication. The communication region here is the region outside of MMRAM that is directly
/// written to and read from by DXE components. Care should always be taken to validate the contents of MM Communication
/// especially in MM code. Platforms may also impose restrictions on the accessibility of the MM Communication Region
/// from within MM. For example, only mapping designated MM Communication Region pages to the MM address space. Consult
/// platform requirements and documentation for more information.
#[derive(FromHob, Default, Clone, Copy, zerocopy::FromBytes)]
#[hob = "d4ffc718-fb82-4274-9afc-aa8b1eef5293"]
#[repr(C)]
pub struct MmCommRegionHob {
    buffer_type: u64,
    address: u64,
    pages: u64,
}

#[component]
impl MmConfigurationProvider {
    /// Entry point for the MM Configuration Provider.
    ///
    /// Depends on at least one instance of the MM Communicate Region HOB to be present in the HOB list. This component
    /// will not be dispatched if no MM Communicate Region HOBs are present in the HOB list.
    ///
    /// Depends on a mutable `patina::component::config::mm::MmCommunicationConfiguration` instance to be in storage. This
    /// component will populate the given configuration instance with runtime information about the MM configuration
    /// and lock the configuration to prevent further modifications and to allow components with immutable dependencies
    /// on the configuration to be dispatched.
    ///
    /// ## Parameters
    ///
    /// - `mm_comm_region_hob`: The MM Communicate Region HOB(s) to be used for MM communication.
    /// - `config_mut`: A mutable reference to the MM Configuration Config instance to be populated with runtime
    ///   information.
    ///
    /// ## Returns
    ///
    /// - `Ok(())` if the entry point was successful.
    /// - `Err(patina::error::Result)` if the entry point failed.
    ///
    pub fn entry_point(
        self,
        mm_comm_region_hob: Hob<MmCommRegionHob>,
        mut config_mut: ConfigMut<MmCommunicationConfiguration>,
    ) -> patina::error::Result<()> {
        log::debug!("MM Configuration Provider Entry Point");

        log::debug!("Incoming MM Configuration: {config_mut:?}");

        let pm_base: *const u16 = (register::PCI_EXPRESS_BASE_ADDRESS as usize
            + patina::pci_address!(0, 0x1F, 0, register::ich9::PMBASE) as usize)
            as *const u16;
        let pm_base_value: u16 = unsafe { core::ptr::read_volatile(pm_base) } & register::ich9::PMBASE_MASK;

        log::info!("ACPI I/O Port Address: {:#X}", pm_base as usize);
        log::info!("ACPI (PMBASE) I/O Port: {pm_base_value:#X}");

        config_mut.acpi_base = pm_base_value.into();

        log::info!("Found {} MM Communicate Region HOBs", mm_comm_region_hob.iter().count());

        for hob in mm_comm_region_hob.iter() {
            log::debug!("HOB Address: {:#X}", hob.address);
            log::debug!("HOB Pages: {:#X}", hob.pages);
            log::debug!("HOB Buffer Type: {:#X}", hob.buffer_type);

            let buffer = unsafe {
                CommunicateBuffer::from_raw_parts(
                    hob.address as usize as *mut u8,
                    hob.pages as usize * patina::base::UEFI_PAGE_SIZE,
                    hob.buffer_type as u8,
                )
            };

            match buffer {
                Ok(buffer) => {
                    config_mut.comm_buffers.push(buffer);
                }
                Err(e) => {
                    log::error!("Failed to create MM Communicate Buffer from HOB: {e:?}");
                    debug_assert!(false, "Failed to create MM Communicate Buffer from HOB");
                }
            };
        }

        config_mut.lock();

        Ok(())
    }
}
