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
    smbios_types::{
        AssociativityField, BiosCharacteristics, BiosCharacteristicsExt1, BiosCharacteristicsExt2, BoardType,
        BootUpState, CacheConfiguration, CacheErrorCorrectionType, CacheSize, CacheSize2, CacheSramTypeData,
        ExtendedBiosRomSize, FeatureFlags, MemoryArrayErrorCorrectionType, MemoryArrayLocation, MemoryArrayUse,
        MemoryCapability, MemoryDeviceAttributes, MemoryDeviceTechnology, MemoryDeviceType, MemoryDeviceTypeDetails,
        MemoryFormFactor, PowerSupplyState, ProcessorCharacteristics, ProcessorFamilyData, ProcessorInformationStatus,
        ProcessorTypeData, ProcessorUpgrade, ProcessorVoltage, SecurityStatus, SystemCacheType, ThermalState,
        WakeUpType,
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
            characteristics: BiosCharacteristics::new().with_pci_supported(true),
            characteristics_ext1: BiosCharacteristicsExt1::new()
                .with_acpi_supported(true)
                .with_usb_legacy_supported(true)
                .with_smart_battery_supported(true),
            characteristics_ext2: BiosCharacteristicsExt2::new()
                .with_bios_boot_specification_supported(true)
                .with_uefi_spec_supported(true),
            system_bios_major_release: 1,
            system_bios_minor_release: 0,
            embedded_controller_major_release: 0xFF,
            embedded_controller_minor_release: 0xFF,
            extended_bios_rom_size: ExtendedBiosRomSize::new(),
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
            wake_up_type: WakeUpType::PowerSwitch,
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
            bootup_state: BootUpState::Safe,
            power_supply_state: PowerSupplyState::Safe,
            thermal_state: ThermalState::Safe,
            security_status: SecurityStatus::Unknown,
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
            feature_flags: FeatureFlags::new().with_hosting_board(true),
            location_in_chassis: 6,
            chassis_handle: type3_handle,
            board_type: BoardType::Motherboard,
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
            cache_configuration: CacheConfiguration::new()
                .with_cache_level(0)
                .with_enabled_disabled(true)
                .with_operational_mode(1), // L1, enabled, write-back
            maximum_cache_size: CacheSize::new().with_max_size(64), // 64 KB
            installed_size: CacheSize::new().with_max_size(64),
            supported_sram_type: CacheSramTypeData::new().with_unknown(true),
            current_sram_type: CacheSramTypeData::new().with_unknown(true),
            cache_speed: 0,
            error_correction_type: CacheErrorCorrectionType::SingleBitEcc,
            system_cache_type: SystemCacheType::Unified,
            associativity: AssociativityField::FullyAssociative,
            maximum_cache_size2: CacheSize2::new().with_max_size(64),
            installed_size2: CacheSize2::new().with_max_size(64),
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
            cache_configuration: CacheConfiguration::new()
                .with_cache_level(1)
                .with_enabled_disabled(true)
                .with_operational_mode(1), // L2, enabled, write-back
            maximum_cache_size: CacheSize::new().with_max_size(256), // 256 KB
            installed_size: CacheSize::new().with_max_size(256),
            supported_sram_type: CacheSramTypeData::new().with_unknown(true),
            current_sram_type: CacheSramTypeData::new().with_unknown(true),
            cache_speed: 0,
            error_correction_type: CacheErrorCorrectionType::SingleBitEcc,
            system_cache_type: SystemCacheType::Unified,
            associativity: AssociativityField::FullyAssociative,
            maximum_cache_size2: CacheSize2::new().with_max_size(256),
            installed_size2: CacheSize2::new().with_max_size(256),
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
            processor_type: ProcessorTypeData::CentralProcessor,
            processor_family: 0xFE, // Use processor_family2
            processor_manufacturer: 2,
            processor_id: [0u8; 8],
            processor_version: 3,
            voltage: ProcessorVoltage::new().with_processor_voltage_indicate_legacy(true),
            external_clock: 0, // Unknown
            max_speed: 2000,
            current_speed: 2000,
            status: ProcessorInformationStatus::new().with_cpu_status(1).with_cpu_socket_populated(true),
            processor_upgrade: ProcessorUpgrade::NoUpgrade, // None
            l1_cache_handle,
            l2_cache_handle,
            l3_cache_handle: 0xFFFF, // Not provided
            serial_number: 4,
            asset_tag: 5,
            part_number: 6,
            core_count: 1,
            core_enabled: 1,
            thread_count: 1,
            processor_characteristics: ProcessorCharacteristics::new().with_capable_64bit(true), // 64-bit capable
            processor_family2: ProcessorFamilyData::ARMv8,
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
            location: MemoryArrayLocation::SystemBoard,
            use_field: MemoryArrayUse::SystemMemory,
            memory_error_correction: MemoryArrayErrorCorrectionType::NoEcc,
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
            size: 0x0400, // 1024 MB
            form_factor: MemoryFormFactor::Dimm,
            device_set: 0,
            device_locator: 1,
            bank_locator: 2,
            memory_type: MemoryDeviceType::Ddr4,
            type_detail: MemoryDeviceTypeDetails::new().with_synchronous(true),
            speed: 3200,
            manufacturer: 3,
            serial_number: 4,
            asset_tag: 5,
            part_number: 6,
            attributes: MemoryDeviceAttributes::new().with_rank(1),
            extended_size: 0,
            configured_memory_clock_speed: 3200,
            minimum_voltage: 1200,
            maximum_voltage: 1200,
            configured_voltage: 1200,
            memory_technology: MemoryDeviceTechnology::Unknown,
            memory_operating_mode_capability: MemoryCapability::new().with_volatile_memory(true),
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
