//! SBSA SMBIOS Platform Component
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
        Type4ProcessorInformation, Type7CacheInformation, Type16PhysicalMemoryArray, Type17MemoryDevice,
        Type19MemoryArrayMappedAddress,
    },
};

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

        // Type 7: Cache Information - L1 and L2 caches for the virtual processor
        let l1_cache = Type7CacheInformation {
            header: SmbiosTableHeader::new(7, 0, SMBIOS_HANDLE_PI_RESERVED),
            socket_designation: 1,
            cache_configuration: 0x0180, // L1, enabled, write-back
            maximum_cache_size: 64,      // 64 KB
            installed_size: 64,
            supported_sram_type: 0x0002, // Unknown
            current_sram_type: 0x0002,
            cache_speed: 0,
            error_correction_type: 0x05, // Single-bit ECC
            system_cache_type: 0x05,     // Unified
            associativity: 0x06,         // 8-way
            maximum_cache_size2: 64,
            installed_cache_size2: 64,
            string_pool: vec![String::from("L1 Cache")],
        };

        let mut l1_cache_handle = 0xFFFF;
        match smbios.add_record(None, &l1_cache) {
            Ok(handle) => {
                log::trace!("  Type 7 (L1 Cache) - Handle 0x{:04X}", handle);
                l1_cache_handle = handle;
            }
            Err(e) => log::warn!("  Failed to add Type 7 (L1 Cache): {:?}", e),
        }

        let l2_cache = Type7CacheInformation {
            header: SmbiosTableHeader::new(7, 0, SMBIOS_HANDLE_PI_RESERVED),
            socket_designation: 1,
            cache_configuration: 0x0181, // L2, enabled, write-back
            maximum_cache_size: 256,     // 256 KB
            installed_size: 256,
            supported_sram_type: 0x0002,
            current_sram_type: 0x0002,
            cache_speed: 0,
            error_correction_type: 0x05, // Single-bit ECC
            system_cache_type: 0x05,     // Unified
            associativity: 0x06,         // 8-way
            maximum_cache_size2: 256,
            installed_cache_size2: 256,
            string_pool: vec![String::from("L2 Cache")],
        };

        let mut l2_cache_handle = 0xFFFF;
        match smbios.add_record(None, &l2_cache) {
            Ok(handle) => {
                log::trace!("  Type 7 (L2 Cache) - Handle 0x{:04X}", handle);
                l2_cache_handle = handle;
            }
            Err(e) => log::warn!("  Failed to add Type 7 (L2 Cache): {:?}", e),
        }

        // Type 4: Processor Information
        let processor_info = Type4ProcessorInformation {
            header: SmbiosTableHeader::new(4, 0, SMBIOS_HANDLE_PI_RESERVED),
            socket_designation: 1,
            processor_type: 0x03,   // Central Processor
            processor_family: 0xFE, // Use processor_family2
            processor_manufacturer: 2,
            processor_id: [0u8; 8],
            processor_version: 3,
            voltage: 0x80,     // Legacy mode, voltage unknown
            external_clock: 0, // Unknown
            max_speed: 2000,
            current_speed: 2000,
            status: 0x41,            // CPU Enabled, Populated
            processor_upgrade: 0x06, // None
            l1_cache_handle,
            l2_cache_handle,
            l3_cache_handle: 0xFFFF, // Not provided
            serial_number: 4,
            asset_tag: 5,
            part_number: 6,
            core_count: 1,
            core_enabled: 1,
            thread_count: 1,
            processor_characteristics: 0x04, // 64-bit capable
            processor_family2: 0x0100,       // ARMv8
            core_count2: 1,
            core_enabled2: 1,
            thread_count2: 1,
            string_pool: vec![
                String::from("CPU0"),
                String::from("QEMU"),
                String::from("ARMv8 Virtual Processor"),
                String::from("SN-CPU-001"),
                String::from("ASSET-CPU-001"),
                String::from("PN-CPU-001"),
            ],
        };

        match smbios.add_record(None, &processor_info) {
            Ok(handle) => log::trace!("  Type 4 (Processor Info) - Handle 0x{:04X}", handle),
            Err(e) => log::warn!("  Failed to add Type 4: {:?}", e),
        }

        // Type 16: Physical Memory Array
        let memory_array = Type16PhysicalMemoryArray {
            header: SmbiosTableHeader::new(16, 0, SMBIOS_HANDLE_PI_RESERVED),
            location: 0x03,                          // System board
            use_field: 0x03,                         // System memory
            memory_error_correction: 0x03,           // None
            maximum_capacity: 0x00100000,            // 1 GB in KB
            memory_error_information_handle: 0xFFFE, // Not provided
            number_of_memory_devices: 1,
            extended_maximum_capacity: 0,
            string_pool: vec![],
        };

        let mut type16_handle = 0xFFFF;
        match smbios.add_record(None, &memory_array) {
            Ok(handle) => {
                log::trace!("  Type 16 (Physical Memory Array) - Handle 0x{:04X}", handle);
                type16_handle = handle;
            }
            Err(e) => log::warn!("  Failed to add Type 16: {:?}", e),
        }

        // Type 17: Memory Device
        let memory_device = Type17MemoryDevice {
            header: SmbiosTableHeader::new(17, 0, SMBIOS_HANDLE_PI_RESERVED),
            physical_memory_array_handle: type16_handle,
            memory_error_information_handle: 0xFFFE, // Not provided
            total_width: 64,
            data_width: 64,
            size: 0x0400,      // 1024 MB
            form_factor: 0x09, // DIMM
            device_set: 0,
            device_locator: 1,
            bank_locator: 2,
            memory_type: 0x1A,   // DDR4
            type_detail: 0x0080, // Synchronous
            speed: 3200,
            manufacturer: 3,
            serial_number: 4,
            asset_tag: 5,
            part_number: 6,
            attributes: 0x01, // Single rank
            extended_size: 0,
            configured_memory_clock_speed: 3200,
            minimum_voltage: 1200,
            maximum_voltage: 1200,
            configured_voltage: 1200,
            memory_technology: 0x02,                  // DRAM
            memory_operating_mode_capability: 0x0004, // Volatile
            firmware_version: 7,
            module_manufacturer_id: 0,
            module_product_id: 0,
            memory_subsystem_controller_manufacturer_id: 0,
            memory_subsystem_controller_product_id: 0,
            non_volatile_size: 0,
            volatile_size: 0x40000000, // 1 GB
            cache_size: 0,
            logical_size: 0,
            extended_speed: 0,
            extended_configured_memory_speed: 0,
            pmic0_manufacturer_id: 0,
            pmic0_revision_number: 0,
            rcd_manufacturer_id: 0,
            rcd_revision_number: 0,
            string_pool: vec![
                String::from("DIMM 0"),
                String::from("BANK 0"),
                String::from("QEMU"),
                String::from("SN-DIMM-001"),
                String::from("ASSET-DIMM-001"),
                String::from("QEMU-DIMM"),
                String::from("v1.0"),
            ],
        };

        match smbios.add_record(None, &memory_device) {
            Ok(handle) => log::trace!("  Type 17 (Memory Device) - Handle 0x{:04X}", handle),
            Err(e) => log::warn!("  Failed to add Type 17: {:?}", e),
        }

        // Type 19: Memory Array Mapped Address
        let memory_mapped = Type19MemoryArrayMappedAddress {
            header: SmbiosTableHeader::new(19, 0, SMBIOS_HANDLE_PI_RESERVED),
            starting_address: 0,
            ending_address: 0x000FFFFF, // 1 GB - 1 in KB
            memory_array_handle: type16_handle,
            partition_width: 1,
            extended_starting_address: 0,
            extended_ending_address: 0,
            string_pool: vec![],
        };

        match smbios.add_record(None, &memory_mapped) {
            Ok(handle) => log::trace!("  Type 19 (Memory Array Mapped Address) - Handle 0x{:04X}", handle),
            Err(e) => log::warn!("  Failed to add Type 19: {:?}", e),
        }

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
