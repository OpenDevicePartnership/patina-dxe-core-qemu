//! Q35 SMBIOS Platform Component
//!
//! This component demonstrates the recommended SMBIOS integration pattern:
//! 1. Uses the type-safe `add_record<T>()` API for adding SMBIOS records
//! 2. Platform component publishes the table after all records are added
//! 3. Uses structured record types (Type0, Type1, Type127)
//!
//! ## License
//!
//! Copyright (C) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

extern crate alloc;
use alloc::{ffi::CString, string::String, vec};
use core::ffi::c_char;

use patina::{
    boot_services::{BootServices, StandardBootServices},
    component::{Storage, component, service::Service},
    error::Result,
};
use patina_smbios::{
    service::{SMBIOS_HANDLE_PI_RESERVED, Smbios, SmbiosExt, SmbiosHandle, SmbiosTableHeader},
    smbios_record::{Type0PlatformFirmwareInformation, Type1SystemInformation},
};
use r_efi::efi;

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

    fn entry_point(self, smbios: Service<dyn Smbios>, storage: &mut Storage) -> Result<()> {
        log::info!("=== Q35 SMBIOS Platform Component ===");

        // Verify SMBIOS version
        let (major, minor) = smbios.version();
        log::info!("SMBIOS Version: {}.{}", major, minor);

        // Add platform SMBIOS records using the type-safe API
        log::info!("Creating platform SMBIOS records...");

        // Type 0: BIOS/Firmware Information
        // Uses add_record<T>() - the recommended type-safe API
        let bios_info = Type0PlatformFirmwareInformation {
            header: SmbiosTableHeader::new(0, 0, SMBIOS_HANDLE_PI_RESERVED),
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
            string_pool: vec![String::from("Patina Firmware"), String::from("1.0.0"), String::from("10/30/2025")],
        };

        match smbios.add_record(None, &bios_info) {
            Ok(handle) => log::info!("  Type 0 (BIOS Info) - Handle 0x{:04X}", handle),
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
            Ok(handle) => log::info!("  Type 1 (System Info) - Handle 0x{:04X}", handle),
            Err(e) => log::warn!("  Failed to add Type 1: {:?}", e),
        }

        // Type 127 End-of-Table marker is automatically added by the manager during initialization
        log::info!("Platform SMBIOS records created successfully");

        // Test C Protocol FFI Layer
        // This exercises the EDK2-compatible protocol functions to verify they work correctly
        log::info!("=== Testing C Protocol FFI Layer ===");
        Self::test_c_protocol_layer(storage.boot_services())?;

        // Publish the SMBIOS table
        // This makes the table available to the OS via UEFI Configuration Table
        log::info!("Publishing SMBIOS table to Configuration Table...");
        match smbios.publish_table() {
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

    /// Test the C Protocol FFI layer by calling the protocol functions directly
    ///
    /// This tests the EDK2-compatible protocol layer (Add, UpdateString, Remove)
    /// which are the FFI functions that C code calls. We invoke them through the
    /// installed protocol to verify the FFI boundary works correctly.
    fn test_c_protocol_layer(boot_services: &StandardBootServices) -> Result<()> {
        log::info!("Testing SMBIOS C Protocol functions...");

        // Define the SMBIOS protocol GUID
        const SMBIOS_PROTOCOL_GUID: efi::Guid =
            efi::Guid::from_fields(0x03583ff6, 0xcb36, 0x4940, 0x94, 0x7e, &[0xb9, 0xb3, 0x9f, 0x4a, 0xfa, 0xf7]);

        // Locate the SMBIOS protocol
        let protocol_ptr = unsafe {
            boot_services.locate_protocol_unchecked(&SMBIOS_PROTOCOL_GUID, core::ptr::null_mut()).map_err(|e| {
                log::error!("Failed to locate SMBIOS protocol: {:?}", e);
                patina::error::EfiError::from(e)
            })?
        };

        // Cast to protocol structure
        // SAFETY: We know this is the correct protocol structure because we just located it
        #[repr(C)]
        struct SmbiosProtocol {
            add: extern "efiapi" fn(
                *const SmbiosProtocol,
                efi::Handle,
                *mut SmbiosHandle,
                *const SmbiosTableHeader,
            ) -> efi::Status,
            update_string:
                extern "efiapi" fn(*const SmbiosProtocol, *mut SmbiosHandle, *mut usize, *const c_char) -> efi::Status,
            remove: extern "efiapi" fn(*const SmbiosProtocol, SmbiosHandle) -> efi::Status,
            get_next: extern "efiapi" fn(
                *const SmbiosProtocol,
                *mut SmbiosHandle,
                *mut u8,
                *mut *mut SmbiosTableHeader,
                *mut efi::Handle,
            ) -> efi::Status,
            major_version: u8,
            minor_version: u8,
        }

        let protocol = unsafe { &*(protocol_ptr as *const SmbiosProtocol) };

        // Test 1: Add a record using the C protocol Add function
        log::info!("  Test 1: Protocol Add function...");
        let test_record = Self::create_test_type2_record();
        let mut handle: SmbiosHandle = 0;

        let status = (protocol.add)(
            protocol,
            core::ptr::null_mut(), // producer_handle
            &mut handle,
            test_record.as_ptr() as *const SmbiosTableHeader,
        );

        if status != efi::Status::SUCCESS {
            log::warn!("    [FAIL] Protocol Add failed: {:?}", status);
        } else {
            log::info!("    [PASS] Protocol Add succeeded - Handle: 0x{:04X}", handle);

            // Test 2: UpdateString using the C protocol UpdateString function
            log::info!("  Test 2: Protocol UpdateString function...");
            let new_string = CString::new("Updated via C Protocol").unwrap();
            let mut string_number: usize = 1;

            let status = (protocol.update_string)(protocol, &mut handle, &mut string_number, new_string.as_ptr());

            if status != efi::Status::SUCCESS {
                log::warn!("    [FAIL] Protocol UpdateString failed: {:?}", status);
            } else {
                log::info!("    [PASS] Protocol UpdateString succeeded");
            }

            // Test 3: Remove using the C protocol Remove function
            log::info!("  Test 3: Protocol Remove function...");
            let status = (protocol.remove)(protocol, handle);

            if status != efi::Status::SUCCESS {
                log::warn!("    [FAIL] Protocol Remove failed: {:?}", status);
            } else {
                log::info!("    [PASS] Protocol Remove succeeded");
            }

            // Test 4: Verify removal - UpdateString should now fail
            log::info!("  Test 4: Verify record removed...");
            let status = (protocol.update_string)(protocol, &mut handle, &mut string_number, new_string.as_ptr());

            if status == efi::Status::SUCCESS {
                log::warn!("    [FAIL] UpdateString after removal should have failed");
            } else {
                log::info!("    [PASS] UpdateString after removal correctly failed: {:?}", status);
            }

            // Test 5: GetNext - enumerate records (test with the handle before it was removed)
            log::info!("  Test 5: Protocol GetNext function...");
            let mut iter_handle: SmbiosHandle = SMBIOS_HANDLE_PI_RESERVED;
            let mut record_type: u8 = 0;
            let mut record_ptr: *mut SmbiosTableHeader = core::ptr::null_mut();
            let mut producer_handle: efi::Handle = core::ptr::null_mut();

            // Get first record
            let status = (protocol.get_next)(
                protocol,
                &mut iter_handle,
                &mut record_type,
                &mut record_ptr,
                &mut producer_handle,
            );

            if status != efi::Status::SUCCESS {
                log::warn!("    [FAIL] Protocol GetNext (first) failed: {:?}", status);
            } else {
                // Copy fields from packed struct to avoid unaligned reference
                let (rec_type, rec_handle, rec_length) = unsafe {
                    let header = &*record_ptr;
                    (header.record_type, header.handle, header.length)
                };
                log::info!(
                    "    [PASS] Protocol GetNext (first) succeeded - Type: {}, Handle: 0x{:04X}, Length: {}",
                    rec_type,
                    rec_handle,
                    rec_length
                );

                // Get next record
                let status = (protocol.get_next)(
                    protocol,
                    &mut iter_handle,
                    &mut record_type,
                    &mut record_ptr,
                    &mut producer_handle,
                );

                if status != efi::Status::SUCCESS {
                    log::warn!("    [FAIL] Protocol GetNext (second) failed: {:?}", status);
                } else {
                    // Copy fields from packed struct to avoid unaligned reference
                    let (rec_type, rec_handle, rec_length) = unsafe {
                        let header = &*record_ptr;
                        (header.record_type, header.handle, header.length)
                    };
                    log::info!(
                        "    [PASS] Protocol GetNext (second) succeeded - Type: {}, Handle: 0x{:04X}, Length: {}",
                        rec_type,
                        rec_handle,
                        rec_length
                    );
                }
            }
        }

        log::info!("C Protocol FFI layer testing complete");
        Ok(())
    }

    /// Creates a Type 2 (Baseboard Information) record as raw bytes
    ///
    /// This demonstrates the byte-level format used by the C protocol layer.
    fn create_test_type2_record() -> alloc::vec::Vec<u8> {
        let mut record = vec![];

        // Header: type=2, length=0x08, handle=auto-assign
        record.push(2); // type
        record.push(0x08); // length (8 bytes total for Type 2 minimum)
        record.extend_from_slice(&SMBIOS_HANDLE_PI_RESERVED.to_le_bytes());

        // Type 2 fixed data (4 bytes after header to reach length of 8)
        record.push(1); // manufacturer (string 1)
        record.push(2); // product (string 2)
        record.push(3); // version (string 3)
        record.push(4); // serial number (string 4)

        // String pool
        record.extend_from_slice(b"Test Manufacturer\0");
        record.extend_from_slice(b"Test Product\0");
        record.extend_from_slice(b"1.0\0");
        record.extend_from_slice(b"SN-12345\0");

        // String pool terminator (double null)
        record.push(0);

        record
    }
}
