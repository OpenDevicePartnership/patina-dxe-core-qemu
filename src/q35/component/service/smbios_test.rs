//! QEMU Q35 SMBIOS Test
//!
//! Verifies that SMBIOS interfaces are working as expected on the QEMU Q35 platform
//! by exercising the EDK2-compatible C protocol FFI layer.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

extern crate alloc;
use alloc::{ffi::CString, vec, vec::Vec};
use core::ffi::c_char;

use patina::{
    boot_services::{BootServices, StandardBootServices},
    test::patina_test,
    u_assert, u_assert_eq, u_assert_ne,
};
use patina_smbios::service::{SMBIOS_HANDLE_PI_RESERVED, SmbiosHandle, SmbiosTableHeader};
use r_efi::efi;

/// Tests the SMBIOS C Protocol FFI layer by calling the protocol functions directly.
/// This exercises the EDK2-compatible protocol layer (Add, UpdateString, Remove, GetNext)
/// which are the FFI functions that C code calls.
#[patina_test]
fn q35_smbios_ffi_test(boot_services: StandardBootServices) -> patina::test::Result {
    log::debug!("SMBIOS FFI Test - Testing C Protocol FFI Layer");

    test_c_protocol_layer(&boot_services)?;

    log::debug!("SMBIOS FFI Test complete");
    Ok(())
}

/// Test the C Protocol FFI layer by calling the protocol functions directly
fn test_c_protocol_layer(boot_services: &StandardBootServices) -> patina::test::Result {
    log::trace!("Testing SMBIOS C Protocol functions...");

    // Define the SMBIOS protocol GUID
    const SMBIOS_PROTOCOL_GUID: efi::Guid =
        efi::Guid::from_fields(0x03583ff6, 0xcb36, 0x4940, 0x94, 0x7e, &[0xb9, 0xb3, 0x9f, 0x4a, 0xfa, 0xf7]);

    // Locate the SMBIOS protocol
    let protocol_ptr = unsafe {
        boot_services.locate_protocol_unchecked(&SMBIOS_PROTOCOL_GUID, core::ptr::null_mut()).map_err(|e| {
            log::error!("Failed to locate SMBIOS protocol: {:?}", e);
            "Failed to locate SMBIOS protocol"
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
    log::trace!("  Test 1: Protocol Add function...");
    let test_record = create_test_type2_record();
    let mut handle: SmbiosHandle = 0;

    let status = (protocol.add)(
        protocol,
        core::ptr::null_mut(), // producer_handle
        &mut handle,
        test_record.as_ptr() as *const SmbiosTableHeader,
    );
    u_assert_eq!(status, efi::Status::SUCCESS, "Protocol Add should succeed");
    log::trace!("    [PASS] Protocol Add succeeded - Handle: 0x{:04X}", handle);

    // Test 2: UpdateString using the C protocol UpdateString function
    log::trace!("  Test 2: Protocol UpdateString function...");
    let new_string = CString::new("Updated via C Protocol").unwrap();
    let mut string_number: usize = 1;

    let status = (protocol.update_string)(protocol, &mut handle, &mut string_number, new_string.as_ptr());
    u_assert_eq!(status, efi::Status::SUCCESS, "Protocol UpdateString should succeed");
    log::trace!("    [PASS] Protocol UpdateString succeeded");

    // Test 3: Remove using the C protocol Remove function
    log::trace!("  Test 3: Protocol Remove function...");
    let status = (protocol.remove)(protocol, handle);
    u_assert_eq!(status, efi::Status::SUCCESS, "Protocol Remove should succeed");
    log::trace!("    [PASS] Protocol Remove succeeded");

    // Test 4: Verify removal - UpdateString should now fail
    log::trace!("  Test 4: Verify record removed...");
    let status = (protocol.update_string)(protocol, &mut handle, &mut string_number, new_string.as_ptr());
    u_assert_ne!(status, efi::Status::SUCCESS, "UpdateString after removal should fail");
    log::trace!("    [PASS] UpdateString after removal correctly failed: {:?}", status);

    // Test 5: GetNext - enumerate records
    log::trace!("  Test 5: Protocol GetNext function...");
    let mut iter_handle: SmbiosHandle = SMBIOS_HANDLE_PI_RESERVED;
    let mut record_type: u8 = 0;
    let mut record_ptr: *mut SmbiosTableHeader = core::ptr::null_mut();
    let mut producer_handle: efi::Handle = core::ptr::null_mut();

    // Get first record
    let status =
        (protocol.get_next)(protocol, &mut iter_handle, &mut record_type, &mut record_ptr, &mut producer_handle);
    u_assert_eq!(status, efi::Status::SUCCESS, "Protocol GetNext (first) should succeed");

    // Copy fields from packed struct to avoid unaligned reference
    let (rec_type, rec_handle, rec_length) = unsafe {
        let header = &*record_ptr;
        (header.record_type, header.handle, header.length)
    };
    u_assert!(!record_ptr.is_null(), "Record pointer should not be null");
    log::trace!(
        "    [PASS] Protocol GetNext (first) succeeded - Type: {}, Handle: 0x{:04X}, Length: {}",
        rec_type,
        rec_handle,
        rec_length
    );

    // Get next record (optional - may not exist if only a few records in table)
    let status =
        (protocol.get_next)(protocol, &mut iter_handle, &mut record_type, &mut record_ptr, &mut producer_handle);

    if status == efi::Status::SUCCESS {
        // Copy fields from packed struct to avoid unaligned reference
        let (rec_type, rec_handle, rec_length) = unsafe {
            let header = &*record_ptr;
            (header.record_type, header.handle, header.length)
        };
        log::trace!(
            "    [PASS] Protocol GetNext (second) succeeded - Type: {}, Handle: 0x{:04X}, Length: {}",
            rec_type,
            rec_handle,
            rec_length
        );
    } else {
        log::trace!("    [INFO] Protocol GetNext (second) returned: {:?} (no more records)", status);
    }

    log::trace!("C Protocol FFI layer testing complete");
    Ok(())
}

/// Creates a Type 2 (Baseboard Information) record as raw bytes
fn create_test_type2_record() -> Vec<u8> {
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
