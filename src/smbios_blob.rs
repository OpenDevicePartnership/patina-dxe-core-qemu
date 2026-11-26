//! QEMU SMBIOS Table Blob Parser
//!
//! Splits the `etc/smbios/smbios-tables` fw_cfg file provided by QEMU into individual SMBIOS structures. QEMU builds
//! this blob from its command line (`-smp`, `-m`, `-smbios`, etc.), so it is the single source for the Type 0, 1, 3,
//! 4, 16, 17, 19 and 32 records that the guest is expected to see.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use patina_smbios::service::Smbios;

extern crate alloc;
use alloc::vec::Vec;

/// Name of the fw_cfg file that holds the SMBIOS structure table.
pub const SMBIOS_TABLES_FILE: &str = "etc/smbios/smbios-tables";

/// SMBIOS structure type of the End-of-Table marker.
pub const SMBIOS_TYPE_END_OF_TABLE: u8 = 127;

/// SMBIOS structure type of BIOS Information.
pub const SMBIOS_TYPE_BIOS_INFORMATION: u8 = 0;

/// SMBIOS structure type of Base Board (or Module) Information.
pub const SMBIOS_TYPE_BASEBOARD_INFORMATION: u8 = 2;

/// Minimum length of an SMBIOS structure (the common header).
const SMBIOS_HEADER_LENGTH: usize = 4;

/// Reasons the SMBIOS table blob could not be split into structures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmbiosBlobError {
    /// The blob ended before the structure header or its string pool was complete.
    Truncated,
    /// A structure declared a formatted length smaller than the common header.
    InvalidLength,
}

/// Iterator over the raw structures (formatted area plus string pool) in an SMBIOS table blob.
///
/// Each item is a slice that can be passed directly to `Smbios::add_from_bytes`. Iteration stops after the first error.
pub struct SmbiosBlobIter<'a> {
    remaining: &'a [u8],
}

impl<'a> SmbiosBlobIter<'a> {
    /// Creates an iterator over the structures in `blob`.
    pub fn new(blob: &'a [u8]) -> Self {
        Self { remaining: blob }
    }
}

impl<'a> Iterator for SmbiosBlobIter<'a> {
    type Item = Result<&'a [u8], SmbiosBlobError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }

        let result = next_structure(self.remaining);
        self.remaining = match result {
            Ok(structure) => self.remaining.get(structure.len()..).unwrap_or(&[]),
            Err(_) => &[],
        };
        Some(result)
    }
}

fn next_structure(data: &[u8]) -> Result<&[u8], SmbiosBlobError> {
    let formatted_length = *data.get(1).ok_or(SmbiosBlobError::Truncated)? as usize;
    if data.len() < SMBIOS_HEADER_LENGTH {
        return Err(SmbiosBlobError::Truncated);
    }
    if formatted_length < SMBIOS_HEADER_LENGTH {
        return Err(SmbiosBlobError::InvalidLength);
    }

    // The string pool ends at the first double NUL at or after the formatted area. An empty pool is exactly two NULs.
    let end = data
        .get(formatted_length..)
        .ok_or(SmbiosBlobError::Truncated)?
        .windows(2)
        .position(|pair| pair == [0, 0])
        .ok_or(SmbiosBlobError::Truncated)?;

    data.get(..formatted_length + end + 2).ok_or(SmbiosBlobError::Truncated)
}

/// Adds every structure in the QEMU SMBIOS table blob and returns the number of records added.
///
/// The End-of-Table marker is skipped because the SMBIOS service manages it. Type 0 (BIOS Information) and Type 2
/// (Base Board Information) are skipped because the firmware supplies those statically. QEMU's fw_cfg does not describe
/// For Type 4 (Processor Information) and Type 17 (Memory Device), QEMU's defaults omit the Serial Number and Asset
/// Tag strings; `fill_missing_identifiers` fills those with static placeholders so they match the values the firmware
/// used to report. Records that the service rejects are logged and skipped.
pub fn import_tables(smbios: &dyn Smbios, blob: &[u8]) -> usize {
    let mut added = 0;
    for structure in SmbiosBlobIter::new(blob) {
        let structure = match structure {
            Ok(structure) => structure,
            Err(e) => {
                log::warn!("Stopped parsing fw_cfg SMBIOS tables: {:?}", e);
                break;
            }
        };

        let Some(&record_type) = structure.first() else {
            continue;
        };
        if record_type == SMBIOS_TYPE_END_OF_TABLE
            || record_type == SMBIOS_TYPE_BIOS_INFORMATION
            || record_type == SMBIOS_TYPE_BASEBOARD_INFORMATION
        {
            continue;
        }

        let structure = fill_missing_identifiers(structure);
        match smbios.add_from_bytes(None, &structure) {
            Ok(handle) => {
                log::trace!("  Type {} (fw_cfg) - Handle 0x{:04X}", record_type, handle);
                added += 1;
            }
            Err(e) => log::warn!("  Failed to add fw_cfg Type {}: {:?}", record_type, e),
        }
    }
    added
}

/// Byte offset of an identifying string field within a structure's formatted area, and the placeholder value to use
/// if QEMU did not assign a string there.
struct IdentifierField {
    offset: usize,
    placeholder: &'static str,
}

/// Returns the Serial Number / Asset Tag fields known for a structure type, if any.
///
/// Offsets are fixed by the SMBIOS specification and so apply regardless of which producer built the structure.
fn identifier_fields(record_type: u8) -> &'static [IdentifierField] {
    const TYPE4_FIELDS: [IdentifierField; 2] = [
        IdentifierField { offset: 0x20, placeholder: "SN-CPU-001" },
        IdentifierField { offset: 0x21, placeholder: "ASSET-CPU-001" },
    ];
    const TYPE17_FIELDS: [IdentifierField; 2] = [
        IdentifierField { offset: 0x18, placeholder: "SN-DIMM-001" },
        IdentifierField { offset: 0x19, placeholder: "ASSET-DIMM-001" },
    ];
    const TYPE3_FIELDS: [IdentifierField; 2] = [
        IdentifierField { offset: 0x07, placeholder: "CHASSIS-99999" },
        IdentifierField { offset: 0x08, placeholder: "ASSET-CHASSIS-001" },
    ];
    match record_type {
        3 => &TYPE3_FIELDS,
        4 => &TYPE4_FIELDS,
        17 => &TYPE17_FIELDS,
        _ => &[],
    }
}

/// Fills unassigned Serial Number / Asset Tag string fields in a Type 4 or Type 17 structure with static
/// placeholders, leaving everything else (including any string QEMU already assigned) untouched.
///
/// Returns `structure` unchanged, as an owned copy, if its type has no known identifier fields or every known field
/// already has a string assigned.
fn fill_missing_identifiers(structure: &[u8]) -> Vec<u8> {
    let fields = identifier_fields(structure.first().copied().unwrap_or(0xFF));
    let formatted_length = structure.get(1).copied().unwrap_or(0) as usize;
    if fields.is_empty() || formatted_length > structure.len() {
        return structure.to_vec();
    }

    // The string pool is either two NUL bytes or one or more NUL-terminated strings followed by one extra NUL.
    // Splitting the latter on NUL bytes yields one segment per string plus two trailing empty segments, one for the
    // final string's own terminator and one for the pool's extra terminator.
    let pool = &structure[formatted_length..];
    let mut strings: Vec<&[u8]> = if pool.first() == Some(&0) {
        Vec::new()
    } else {
        let segments: Vec<&[u8]> = pool.split(|&b| b == 0).collect();
        segments[..segments.len().saturating_sub(2)].to_vec()
    };

    let mut formatted = structure[..formatted_length].to_vec();
    let mut changed = false;
    for field in fields {
        match formatted.get(field.offset) {
            Some(0) => {}
            _ => continue, // Field does not exist in this record version, or QEMU already assigned a string.
        }

        strings.push(field.placeholder.as_bytes());
        if let Some(byte) = formatted.get_mut(field.offset) {
            *byte = strings.len() as u8;
        }
        changed = true;
    }

    if !changed {
        return structure.to_vec();
    }

    let mut rebuilt = formatted;
    for s in &strings {
        rebuilt.extend_from_slice(s);
        rebuilt.push(0);
    }
    rebuilt.push(0);
    rebuilt
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::vec::Vec;

    use patina_smbios::{error::SmbiosError, service::SmbiosHandle};

    use super::*;

    /// Minimal Type 4 structure (formatted area only, no strings) with Serial Number and Asset Tag unassigned
    /// (byte 0 at their offsets), matching what QEMU's fw_cfg defaults produce.
    fn type4_missing_identifiers() -> Vec<u8> {
        let mut record = alloc::vec![0u8; 0x23];
        record[0] = 4; // type
        record[1] = 0x23; // formatted length
        record[7] = 1; // processor_manufacturer string index
        record
    }

    fn type17_missing_identifiers() -> Vec<u8> {
        let mut record = alloc::vec![0u8; 0x1B];
        record[0] = 17;
        record[1] = 0x1B;
        record
    }

    #[test]
    fn fill_missing_identifiers_adds_placeholders_to_type4() {
        let mut record = type4_missing_identifiers();
        record.extend_from_slice(b"QEMU\0\0"); // existing string 1 (manufacturer)

        let filled = fill_missing_identifiers(&record);

        assert_eq!(filled[0x20], 2, "Serial Number should point at the first appended string");
        assert_eq!(filled[0x21], 3, "Asset Tag should point at the second appended string");
        let pool = &filled[0x23..];
        assert_eq!(pool, b"QEMU\0SN-CPU-001\0ASSET-CPU-001\0\0");
    }

    #[test]
    fn fill_missing_identifiers_adds_placeholders_to_type3() {
        let mut record = alloc::vec![0u8; 0x09];
        record[0] = 3;
        record[1] = 0x09;
        record[4] = 1; // manufacturer string index
        record.extend_from_slice(b"QEMU\0\0");

        let filled = fill_missing_identifiers(&record);

        assert_eq!(filled[0x07], 2);
        assert_eq!(filled[0x08], 3);
        assert_eq!(&filled[0x09..], b"QEMU\0CHASSIS-99999\0ASSET-CHASSIS-001\0\0");
    }

    #[test]
    fn fill_missing_identifiers_adds_placeholders_to_type17() {
        let mut record = type17_missing_identifiers();
        record.extend_from_slice(b"\0\0"); // empty string pool

        let filled = fill_missing_identifiers(&record);

        assert_eq!(filled[0x18], 1);
        assert_eq!(filled[0x19], 2);
        let pool = &filled[0x1B..];
        assert_eq!(pool, b"SN-DIMM-001\0ASSET-DIMM-001\0\0");
    }

    #[test]
    fn fill_missing_identifiers_leaves_assigned_strings_untouched() {
        let mut record = type4_missing_identifiers();
        record[0x20] = 2; // QEMU already assigned a serial number string
        record.extend_from_slice(b"QEMU\0SN-1234\0\0");

        let filled = fill_missing_identifiers(&record);

        assert_eq!(filled[0x20], 2, "existing Serial Number index must be preserved");
        assert_eq!(filled[0x21], 3, "Asset Tag should still get a placeholder");
        assert_eq!(&filled[0x23..], b"QEMU\0SN-1234\0ASSET-CPU-001\0\0");
    }

    #[test]
    fn fill_missing_identifiers_ignores_unknown_types() {
        let mut record = alloc::vec![1u8, 8, 0, 0, 0, 0, 0, 0];
        record.extend_from_slice(b"\0\0");
        assert_eq!(fill_missing_identifiers(&record), record);
    }

    #[test]
    fn fill_missing_identifiers_ignores_truncated_record() {
        let record = alloc::vec![4u8, 0x40, 0, 0, 0, 0];
        assert_eq!(fill_missing_identifiers(&record), record);
    }

    fn type1_with_strings() -> Vec<u8> {
        let mut record = alloc::vec![1u8, 8, 0x00, 0x01, 1, 2, 0, 0];
        record.extend_from_slice(b"QEMU\0Q35\0\0");
        record
    }

    fn type4_no_strings() -> Vec<u8> {
        alloc::vec![4u8, 6, 0x00, 0x04, 0xAA, 0xBB, 0, 0]
    }

    fn end_of_table() -> Vec<u8> {
        alloc::vec![127u8, 4, 0xFF, 0x7F, 0, 0]
    }

    #[test]
    fn import_tables_skips_type0_and_type127_and_counts_added() {
        let mut blob = alloc::vec![0u8, 4, 0, 0, 0, 0];
        blob.extend(type1_with_strings());
        blob.extend(type4_no_strings());
        blob.extend(end_of_table());

        let smbios = RecordingSmbios::default();
        assert_eq!(import_tables(&smbios, &blob), 2);
        assert_eq!(*smbios.types.borrow(), alloc::vec![1, 4]);
    }

    #[test]
    fn import_tables_does_not_count_rejected_records() {
        let mut blob = type1_with_strings();
        blob.extend(type4_no_strings());

        let smbios = RecordingSmbios { reject_type: Some(1), ..Default::default() };
        assert_eq!(import_tables(&smbios, &blob), 1);
    }

    #[derive(Default)]
    struct RecordingSmbios {
        types: core::cell::RefCell<Vec<u8>>,
        reject_type: Option<u8>,
    }

    impl Smbios for RecordingSmbios {
        fn version(&self) -> (u8, u8) {
            (3, 9)
        }

        fn publish_table(&self) -> Result<(u64, u64), SmbiosError> {
            Err(SmbiosError::NoRecordsAvailable)
        }

        fn update_string(&self, _: SmbiosHandle, _: usize, _: &str) -> Result<(), SmbiosError> {
            Ok(())
        }

        fn remove(&self, _: SmbiosHandle) -> Result<(), SmbiosError> {
            Ok(())
        }

        fn add_from_bytes(
            &self,
            _: Option<patina::standard::efi::Handle>,
            bytes: &[u8],
        ) -> Result<SmbiosHandle, SmbiosError> {
            if Some(bytes[0]) == self.reject_type {
                return Err(SmbiosError::HandleInUse);
            }
            self.types.borrow_mut().push(bytes[0]);
            Ok(0)
        }

        fn next_record(
            &self,
            _: SmbiosHandle,
            _: Option<u8>,
        ) -> Result<
            Option<(patina_smbios::service::SmbiosTableHeader, Option<patina::standard::efi::Handle>)>,
            SmbiosError,
        > {
            Ok(None)
        }

        fn record_pointer(
            &self,
            _: SmbiosHandle,
        ) -> Result<Option<(u64, Option<patina::standard::efi::Handle>)>, SmbiosError> {
            Ok(None)
        }
    }

    #[test]
    fn splits_structures_with_and_without_strings() {
        let mut blob = type1_with_strings();
        blob.extend(type4_no_strings());
        blob.extend(end_of_table());

        let records: Vec<_> = SmbiosBlobIter::new(&blob).collect::<Result<_, _>>().unwrap();

        assert_eq!(records.len(), 3);
        assert_eq!(records[0], type1_with_strings().as_slice());
        assert_eq!(records[1], type4_no_strings().as_slice());
        assert_eq!(records[2][0], SMBIOS_TYPE_END_OF_TABLE);
    }

    #[test]
    fn empty_blob_yields_nothing() {
        assert_eq!(SmbiosBlobIter::new(&[]).count(), 0);
    }

    #[test]
    fn truncated_string_pool_is_an_error_and_stops_iteration() {
        let mut blob = type4_no_strings();
        blob.extend_from_slice(&[1, 8, 0, 1, 1, 2, 0, 0, b'Q', b'E', b'M', b'U']);

        let mut iter = SmbiosBlobIter::new(&blob);
        assert!(iter.next().unwrap().is_ok());
        assert_eq!(iter.next().unwrap(), Err(SmbiosBlobError::Truncated));
        assert!(iter.next().is_none());
    }

    #[test]
    fn formatted_length_smaller_than_header_is_an_error() {
        let blob = [1u8, 2, 0, 0, 0, 0];
        assert_eq!(SmbiosBlobIter::new(&blob).next().unwrap(), Err(SmbiosBlobError::InvalidLength));
    }

    #[test]
    fn formatted_length_past_end_of_blob_is_an_error() {
        let blob = [1u8, 0x40, 0, 0, 0, 0];
        assert_eq!(SmbiosBlobIter::new(&blob).next().unwrap(), Err(SmbiosBlobError::Truncated));
    }
}
