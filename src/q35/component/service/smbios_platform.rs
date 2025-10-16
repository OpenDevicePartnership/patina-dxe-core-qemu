//! Q35 SMBIOS Platform Component
//!
//! This component creates platform-specific SMBIOS records and publishes
//! the complete SMBIOS table to the UEFI Configuration Table.
//!
//! This combines record creation and table publication into a single component,
//! providing a cleaner design where platform SMBIOS data is self-contained.

extern crate alloc;
use alloc::string::String;
use alloc::vec;

use patina::{
    boot_services::StandardBootServices,
    component::{IntoComponent, service::Service},
    error::Result,
};
use patina_smbios::{
    manager::{SmbiosRecords, SmbiosTableHeader, SMBIOS_HANDLE_PI_RESERVED},
    smbios_record::{Type0PlatformFirmwareInformation, Type1SystemInformation, Type127EndOfTable},
    SmbiosService,
};

#[derive(IntoComponent, Default)]
pub struct Q35SmbiosPlatform;

impl Q35SmbiosPlatform {
    pub fn new() -> Self {
        Self
    }

    fn entry_point(
        self,
        smbios_service: Service<dyn SmbiosRecords<'static>>,
        boot_services: StandardBootServices,
    ) -> Result<()> {
        log::info!("=== Q35 SMBIOS Platform Component ===");

        // Verify SMBIOS version
        let (major, minor) = smbios_service.version();
        log::info!("SMBIOS Version: {}.{}", major, minor);

        // Add platform SMBIOS records
        log::info!("Creating platform SMBIOS records...");
        
        // Type 0: BIOS/Firmware Information
        let bios_info = Type0PlatformFirmwareInformation {
            header: SmbiosTableHeader { record_type: 0, length: 0, handle: SMBIOS_HANDLE_PI_RESERVED },
            vendor: 1,
            firmware_version: 2,
            bios_starting_address_segment: 0xE800,
            firmware_release_date: 3,
            firmware_rom_size: 0xFF, // 16MB
            characteristics: 0x08,   // BIOS characteristics
            characteristics_ext1: 0x03,
            characteristics_ext2: 0x03,
            system_bios_major_release: 1,
            system_bios_minor_release: 0,
            embedded_controller_major_release: 0xFF,
            embedded_controller_minor_release: 0xFF,
            extended_bios_rom_size: 0,
            string_pool: vec![String::from("Patina Firmware"), String::from("1.0.0"), String::from("10/15/2025")],
        };

        match smbios_service.add_record(None, &bios_info) {
            Ok(handle) => log::info!("  Type 0 (BIOS Info) - Handle 0x{:04X}", handle),
            Err(e) => log::warn!("  Failed to add Type 0: {:?}", e),
        }

        // Type 1: System Information
        let system_info = Type1SystemInformation {
            header: SmbiosTableHeader { record_type: 1, length: 0, handle: SMBIOS_HANDLE_PI_RESERVED },
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
                String::from("Kat's Virtual Machine"),
            ],
        };

        match smbios_service.add_record(None, &system_info) {
            Ok(handle) => log::info!("  Type 1 (System Info) - Handle 0x{:04X}", handle),
            Err(e) => log::warn!("  Failed to add Type 1: {:?}", e),
        }

        // Type 127: End of Table
        let end_of_table = Type127EndOfTable {
            header: SmbiosTableHeader { record_type: 127, length: 0, handle: SMBIOS_HANDLE_PI_RESERVED },
            string_pool: vec![],
        };

        match smbios_service.add_record(None, &end_of_table) {
            Ok(handle) => log::info!("  Type 127 (End of Table) - Handle 0x{:04X}", handle),
            Err(e) => log::warn!("  Failed to add Type 127: {:?}", e),
        }

        log::info!("Platform SMBIOS records created successfully");

        // Publish the SMBIOS table immediately
        log::info!("Publishing SMBIOS table to Configuration Table...");
        match smbios_service.publish_table(&boot_services) {
            Ok((table_addr, entry_point_addr)) => {
                log::info!("SMBIOS table published successfully");
                log::info!("  Entry Point: 0x{:X}", entry_point_addr);
                log::info!("  Table Data: 0x{:X}", table_addr);
                log::info!("Use 'smbiosview' in UEFI Shell to view records");
            }
            Err(e) => {
                log::error!("Failed to publish SMBIOS table: {:?}", e);
                // Continue even if publication fails - this is not critical
            }
        }

        log::info!("SMBIOS platform component initialized successfully");
        Ok(())
    }
}
