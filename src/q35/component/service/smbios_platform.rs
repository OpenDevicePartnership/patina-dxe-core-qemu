//! Q35 SMBIOS Platform Component
//!
//! Platform component that populates and publishes SMBIOS tables:
//! 1. Uses the type-safe `add_record<T>()` API for adding SMBIOS records
//! 2. Publishes the table after all records are added
//! 3. Uses structured record types (Type0, Type1)
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

extern crate alloc;
use alloc::{string::String, vec};

use patina::{
    component::{component, service::Service},
    error::Result,
};
use patina_smbios::{
    service::{SMBIOS_HANDLE_PI_RESERVED, Smbios, SmbiosExt, SmbiosTableHeader},
    smbios_record::{Type0PlatformFirmwareInformation, Type1SystemInformation},
};

/// Q35 platform SMBIOS component that populates and publishes SMBIOS tables.
///
/// This component adds platform-specific SMBIOS records (Type 0 BIOS Information,
/// Type 1 System Information) and publishes the complete SMBIOS table to the
/// UEFI Configuration Table for OS consumption.
#[derive(Default)]
pub struct Q35SmbiosPlatform;

#[component]
impl Q35SmbiosPlatform {
    /// Creates a new Q35 SMBIOS platform component instance.
    pub fn new() -> Self {
        Self
    }

    fn entry_point(self, smbios: Service<dyn Smbios>) -> Result<()> {
        log::debug!("=== Q35 SMBIOS Platform Component ===");

        // Verify SMBIOS version
        let (major, minor) = smbios.version();
        log::trace!("SMBIOS Version: {}.{}", major, minor);

        // Add platform SMBIOS records using the type-safe API
        log::trace!("Creating platform SMBIOS records...");

        // Type 0: BIOS/Firmware Information
        // Uses add_record<T>() - the recommended type-safe API
        let bios_info = Type0PlatformFirmwareInformation {
            header: SmbiosTableHeader::new(0, 0, SMBIOS_HANDLE_PI_RESERVED),
            vendor: 1,
            firmware_version: 2,
            bios_starting_address_segment: 0xE800,
            firmware_release_date: 3,
            firmware_rom_size: 0xFF, // 16MB
            // BIOS Characteristics (SMBIOS spec 7.1.1)
            // Bit 3: BIOS Characteristics are supported
            characteristics: 0x08,
            // BIOS Characteristics Extension Byte 1 (SMBIOS spec 7.1.2.1)
            // Bit 0: ACPI supported, Bit 1: USB Legacy supported
            characteristics_ext1: 0x03,
            // BIOS Characteristics Extension Byte 2 (SMBIOS spec 7.1.2.2)
            // Bit 0: BIOS Boot Specification supported, Bit 1: Function key-initiated network boot supported
            characteristics_ext2: 0x03,
            system_bios_major_release: 1,
            system_bios_minor_release: 0,
            embedded_controller_major_release: 0xFF,
            embedded_controller_minor_release: 0xFF,
            extended_bios_rom_size: 0,
            string_pool: vec![
                String::from("Patina Firmware"),
                String::from(env!("CARGO_PKG_VERSION")),
                String::from(option_env!("BUILD_DATE").unwrap_or("01/01/1970")),
            ],
        };

        match smbios.add_record(None, &bios_info) {
            Ok(handle) => log::trace!("  Type 0 (BIOS Info) - Handle 0x{:04X}", handle),
            Err(e) => log::warn!("  Failed to add Type 0: {:?}", e),
        }

        // Type 1: System Information
        let system_info = Type1SystemInformation {
            header: SmbiosTableHeader::new(1, 0, SMBIOS_HANDLE_PI_RESERVED),
            manufacturer: 1,
            product_name: 2,
            version: 3,
            serial_number: 4,
            uuid: [0; 16],
            wake_up_type: 0x06, // Power Switch
            sku_number: 5,
            family: 6,
            string_pool: vec![
                String::from("QEMU"),
                String::from("Q35 Virtual Machine"),
                String::from("1.0"),
                String::from("VM-001"),
                String::from("Q35-STANDARD"),
                String::from("Virtual Machine Family"),
            ],
        };

        match smbios.add_record(None, &system_info) {
            Ok(handle) => log::trace!("  Type 1 (System Info) - Handle 0x{:04X}", handle),
            Err(e) => log::warn!("  Failed to add Type 1: {:?}", e),
        }

        // Type 127 End-of-Table marker is automatically added by the manager during initialization
        log::trace!("Platform SMBIOS records created successfully");

        // Publish the SMBIOS table
        // This makes the table available to the OS via UEFI Configuration Table
        log::debug!("Publishing SMBIOS table to Configuration Table...");
        match smbios.publish_table() {
            Ok((table_addr, entry_point_addr)) => {
                log::debug!("SMBIOS table published successfully");
                log::debug!("  Entry Point: 0x{:X}", entry_point_addr);
                log::debug!("  Table Data: 0x{:X}", table_addr);
                log::debug!("Use 'smbiosview' in UEFI Shell to view records");
            }
            Err(e) => {
                log::error!("Failed to publish SMBIOS table: {:?}", e);
                // Continue even if publication fails - this is not critical
            }
        }

        log::debug!("SMBIOS platform component initialized successfully");
        Ok(())
    }
}
