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

/// Name of the fw_cfg file that holds the SMBIOS structure table.
pub const SMBIOS_TABLES_FILE: &str = "etc/smbios/smbios-tables";

/// SMBIOS structure type of the End-of-Table marker.
pub const SMBIOS_TYPE_END_OF_TABLE: u8 = 127;

/// SMBIOS structure type of BIOS Information.
pub const SMBIOS_TYPE_BIOS_INFORMATION: u8 = 0;

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
/// The End-of-Table marker is skipped because the SMBIOS service manages it, and Type 0 is skipped because the
/// firmware produces its own. Records that the service rejects are logged and skipped.
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
        if record_type == SMBIOS_TYPE_END_OF_TABLE || record_type == SMBIOS_TYPE_BIOS_INFORMATION {
            continue;
        }

        match smbios.add_from_bytes(None, structure) {
            Ok(handle) => {
                log::trace!("  Type {} (fw_cfg) - Handle 0x{:04X}", record_type, handle);
                added += 1;
            }
            Err(e) => log::warn!("  Failed to add fw_cfg Type {}: {:?}", record_type, e),
        }
    }
    added
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::vec::Vec;

    use patina_smbios::{error::SmbiosError, service::SmbiosHandle};

    use super::*;

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
