//! SBSA SMBIOS Platform Component
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

extern crate alloc;
use alloc::{format, string::String, vec};

use patina::{
    component::{component, service::Service},
    error::Result,
};
use patina_smbios::{
    service::{SMBIOS_HANDLE_PI_RESERVED, Smbios, SmbiosExt, SmbiosTableHeader},
    smbios_record::{
        Type0PlatformFirmwareInformation, Type1SystemInformation, Type2BaseboardInformation, Type3SystemEnclosure,
        Type4ProcessorInformation, Type7CacheInformation,
    },
};

/// Add processor and cache SMBIOS records (Type 4 + Type 7).
fn add_processor_records(smbios: &Service<dyn Smbios>) {
    // Cache Configuration bits: [2:0] = level - 1, [7] = enabled, [9:8] = operation mode
    const CACHE_ENABLED: u16 = 1 << 7;

    let l1i = Type7CacheInformation {
        header: SmbiosTableHeader::new(7, 0, SMBIOS_HANDLE_PI_RESERVED),
        socket_designation: 1,
        cache_configuration: (3 << 8) | CACHE_ENABLED | 0, // L1, unknown mode
        maximum_cache_size: 64,
        installed_size: 64,
        supported_sram_type: 0x0002,
        current_sram_type: 0x0002,
        cache_speed: 0,
        error_correction_type: 0x02,
        system_cache_type: 0x03, // Instruction
        associativity: 0x05,     // 4-way
        maximum_cache_size2: 64,
        installed_cache_size2: 64,
        string_pool: vec![String::from("L1 Instruction Cache")],
    };
    match smbios.add_record(None, &l1i) {
        Ok(h) => log::trace!("  Type 7 (L1I Cache) - Handle 0x{:04X}", h),
        Err(e) => {
            log::warn!("  Failed to add Type 7 (L1I Cache): {:?}", e);
            return;
        }
    };

    let l1d = Type7CacheInformation {
        header: SmbiosTableHeader::new(7, 0, SMBIOS_HANDLE_PI_RESERVED),
        socket_designation: 1,
        cache_configuration: (1 << 8) | CACHE_ENABLED | 0, // L1, write-back
        maximum_cache_size: 64,
        installed_size: 64,
        supported_sram_type: 0x0002,
        current_sram_type: 0x0002,
        cache_speed: 0,
        error_correction_type: 0x02,
        system_cache_type: 0x04, // Data
        associativity: 0x05,     // 4-way
        maximum_cache_size2: 64,
        installed_cache_size2: 64,
        string_pool: vec![String::from("L1 Data Cache")],
    };
    let l1d_handle = match smbios.add_record(None, &l1d) {
        Ok(h) => {
            log::trace!("  Type 7 (L1D Cache) - Handle 0x{:04X}", h);
            h
        }
        Err(e) => {
            log::warn!("  Failed to add Type 7 (L1D Cache): {:?}", e);
            return;
        }
    };

    let l2 = Type7CacheInformation {
        header: SmbiosTableHeader::new(7, 0, SMBIOS_HANDLE_PI_RESERVED),
        socket_designation: 1,
        cache_configuration: (1 << 8) | CACHE_ENABLED | 1, // L2, write-back
        maximum_cache_size: 1024,
        installed_size: 1024,
        supported_sram_type: 0x0002,
        current_sram_type: 0x0002,
        cache_speed: 0,
        error_correction_type: 0x02,
        system_cache_type: 0x05, // Unified
        associativity: 0x08,     // 16-way
        maximum_cache_size2: 1024,
        installed_cache_size2: 1024,
        string_pool: vec![String::from("L2 Cache")],
    };
    let l2_handle = match smbios.add_record(None, &l2) {
        Ok(h) => {
            log::trace!("  Type 7 (L2 Cache) - Handle 0x{:04X}", h);
            h
        }
        Err(e) => {
            log::warn!("  Failed to add Type 7 (L2 Cache): {:?}", e);
            return;
        }
    };

    let processor = Type4ProcessorInformation {
        header: SmbiosTableHeader::new(4, 0, SMBIOS_HANDLE_PI_RESERVED),
        socket_designation: 1,
        processor_type: 0x03,   // Central Processor
        processor_family: 0xFE, // Use Family2
        processor_manufacturer: 2,
        processor_id: [0u8; 8],
        processor_version: 3,
        voltage: 0,
        external_clock: 62,      // MHz
        max_speed: 2000,         // MHz
        current_speed: 2000,     // MHz
        status: 0x41,            // CPU Enabled + Socket Populated
        processor_upgrade: 0x02, // Unknown
        l1_cache_handle: l1d_handle,
        l2_cache_handle: l2_handle,
        l3_cache_handle: 0xFFFF, // Not provided
        serial_number: 0,
        asset_tag: 0,
        part_number: 0,
        core_count: 1,
        core_enabled: 1,
        thread_count: 1,
        processor_characteristics: (1 << 2) | (1 << 5) | (1 << 9), // 64-bit, XN, ARM64 SoC ID
        processor_family2: 0x0101,                                 // ARMv8
        core_count2: 1,
        core_enabled2: 1,
        thread_count2: 1,
        string_pool: vec![format!("CPU{:02}", 1), String::from("QEMU"), String::from("ARMv8")],
    };
    match smbios.add_record(None, &processor) {
        Ok(h) => log::trace!("  Type 4 (Processor Info) - Handle 0x{:04X}", h),
        Err(e) => log::warn!("  Failed to add Type 4 (Processor Info): {:?}", e),
    }
}

/// SBSA platform SMBIOS record provider.
#[derive(Default)]
pub struct SbsaSmbiosPlatform;

#[component]
impl SbsaSmbiosPlatform {
    /// Creates a new instance.
    pub fn new() -> Self {
        Self
    }

    fn entry_point(self, smbios: Service<dyn Smbios>) -> Result<()> {
        log::debug!("=== SBSA SMBIOS Platform Component ===");

        let (major, minor) = smbios.version();
        log::trace!("SMBIOS Version: {}.{}", major, minor);

        let bios_info = Type0PlatformFirmwareInformation {
            header: SmbiosTableHeader::new(0, 0, SMBIOS_HANDLE_PI_RESERVED),
            vendor: 1,
            firmware_version: 2,
            bios_starting_address_segment: 0xE800,
            firmware_release_date: 3,
            firmware_rom_size: 0xFF,
            characteristics: 0x08,
            characteristics_ext1: 0x03,
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

        // Type 0 and Type 1 are required per SMBIOS spec Section 6.2. Propagate errors
        // to avoid publishing an incompliant table.
        let type0_handle = smbios.add_record(None, &bios_info).map_err(|e| {
            log::error!("Failed to add required Type 0 (BIOS Info): {:?}", e);
            e
        })?;
        log::trace!("  Type 0 (BIOS Info) - Handle 0x{:04X}", type0_handle);

        let system_info = Type1SystemInformation {
            header: SmbiosTableHeader::new(1, 0, SMBIOS_HANDLE_PI_RESERVED),
            manufacturer: 1,
            product_name: 2,
            version: 3,
            serial_number: 4,
            uuid: [0; 16],
            wake_up_type: 0x06,
            sku_number: 5,
            family: 6,
            string_pool: vec![
                String::from("QEMU"),
                String::from("SBSA Virtual Machine"),
                String::from("1.0"),
                String::from("VM-001"),
                String::from("SBSA-STANDARD"),
                String::from("Virtual Machine Family"),
            ],
        };

        let type1_handle = smbios.add_record(None, &system_info).map_err(|e| {
            log::error!("Failed to add required Type 1 (System Info): {:?}", e);
            e
        })?;
        log::trace!("  Type 1 (System Info) - Handle 0x{:04X}", type1_handle);

        let enclosure_info = Type3SystemEnclosure {
            header: SmbiosTableHeader::new(3, 0, SMBIOS_HANDLE_PI_RESERVED),
            manufacturer: 1,
            enclosure_type: 0x03,
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

        let mut type3_handle = 0xFFFF;
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
            feature_flags: 0x01,
            location_in_chassis: 6,
            chassis_handle: type3_handle,
            board_type: 0x0A,
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

        // Type 4 (Processor) + Type 7 (Cache) - replaces the C ProcessorSubClassDxe driver.
        add_processor_records(&smbios);

        log::debug!("Publishing SMBIOS table...");
        let (table_addr, entry_point_addr) = smbios.publish_table().map_err(|e| {
            log::error!("Failed to publish SMBIOS table: {:?}", e);
            e
        })?;
        log::debug!("SMBIOS table published successfully");
        log::debug!("  Entry Point: 0x{:X}", entry_point_addr);
        log::debug!("  Table Data: 0x{:X}", table_addr);

        Ok(())
    }
}
