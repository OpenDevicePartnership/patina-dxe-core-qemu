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
    smbios_record::{
        Type0PlatformFirmwareInformation, Type1SystemInformation, Type2BaseboardInformation, Type3SystemEnclosure,
    },
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

        let enclosure_info = Type3SystemEnclosure {
            header: SmbiosTableHeader::new(3, 0, SMBIOS_HANDLE_PI_RESERVED),
            manufacturer: 1,
            enclosure_type: 0x03, // Desktop
            version: 2,
            serial_number: 3,
            asset_tag_number: 4,
            bootup_state: 0x03,
            power_supply_state: 0x03,
            thermal_state: 0x03,
            security_status: 0x02,
            oem_defined: 0x00000000,
            height: 0x00,
            number_of_power_cords: 0x01,
            contained_element_count: 0x00,
            contained_element_record_length: 0x00,
            string_pool: vec![
                String::from("Example Corporation"),
                String::from("Example Chassis v1.0"),
                String::from("CHASSIS-99999"),
                String::from("ASSET-CHASSIS-001"),
            ],
        };

        let mut type3_handle = 0x0009;
        match smbios.add_record(None, &enclosure_info) {
            Ok(handle) => {
                log::trace!("  Type 3 (System Enclosure) - Handle 0x{:04X}", handle);
                type3_handle = handle;
            }
            Err(e) => log::warn!("  Failed to add Type 3: {:?}", e),
        }

        let baseboard_info = Type2BaseboardInformation {
            header: SmbiosTableHeader::new(2, 0, SMBIOS_HANDLE_PI_RESERVED),
            manufacturer: 1,
            product: 2,
            version: 3,
            serial_number: 4,
            asset_tag: 5,
            feature_flags: 0x01, // Board is a hosting board
            location_in_chassis: 6,
            chassis_handle: type3_handle,
            board_type: 0x0A, // Motherboard
            contained_object_handles: 0,
            string_pool: vec![
                String::from("Example Corporation"),
                String::from("Example Baseboard"),
                String::from("1.0"),
                String::from("MB-67890"),
                String::from("ASSET-MB-001"),
                String::from("Main Board Slot"),
            ],
        };

        match smbios.add_record(None, &baseboard_info) {
            Ok(handle) => log::trace!("  Type 2 (Base Board Info) - Handle 0x{:04X}", handle),
            Err(e) => log::warn!("  Failed to add Type 2: {:?}", e),
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
