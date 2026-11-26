//! QEMU fw_cfg Directory Reader
//!
//! Transport-independent logic for reading files from QEMU's fw_cfg device. The platform supplies a
//! [`FwCfgTransport`] (I/O ports on x86, MMIO on Arm) and this module walks the file directory.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

extern crate alloc;
use alloc::vec::Vec;

const FW_CFG_SIGNATURE: u16 = 0x0000;
const FW_CFG_FILE_DIR: u16 = 0x0019;

const FW_CFG_FILE_NAME_LENGTH: usize = 56;
const FW_CFG_FILE_ENTRY_LENGTH: usize = 64;

/// Upper bound for the number of directory entries to scan.
const FW_CFG_MAX_FILE_COUNT: u32 = 4096;

/// Upper bound for a file read, to avoid a large allocation if the device returns bogus sizes.
const FW_CFG_MAX_FILE_SIZE: usize = 1024 * 1024;

/// Access to the fw_cfg selector and data registers.
pub trait FwCfgTransport {
    /// Selects the item that subsequent [`read_bytes`](Self::read_bytes) calls read from.
    fn select(&mut self, item: u16);

    /// Fills `buffer` with the next bytes of the selected item.
    fn read_bytes(&mut self, buffer: &mut [u8]);
}

fn read_u32_be(transport: &mut impl FwCfgTransport) -> u32 {
    let mut bytes = [0u8; 4];
    transport.read_bytes(&mut bytes);
    u32::from_be_bytes(bytes)
}

/// Returns true if the transport is backed by a fw_cfg device.
pub fn is_present(transport: &mut impl FwCfgTransport) -> bool {
    transport.select(FW_CFG_SIGNATURE);
    let mut signature = [0u8; 4];
    transport.read_bytes(&mut signature);
    &signature == b"QEMU"
}

/// Reads the full contents of the fw_cfg file named `name`.
///
/// Returns `None` if there is no fw_cfg device, the file does not exist, or the file is empty or too large.
pub fn read_file(transport: &mut impl FwCfgTransport, name: &str) -> Option<Vec<u8>> {
    if !is_present(transport) {
        return None;
    }

    transport.select(FW_CFG_FILE_DIR);
    let file_count = read_u32_be(transport);
    if file_count > FW_CFG_MAX_FILE_COUNT {
        log::warn!("fw_cfg reports an implausible file count {}", file_count);
        return None;
    }

    for _ in 0..file_count {
        let mut entry = [0u8; FW_CFG_FILE_ENTRY_LENGTH];
        transport.read_bytes(&mut entry);

        let size = u32::from_be_bytes([entry[0], entry[1], entry[2], entry[3]]) as usize;
        let item = u16::from_be_bytes([entry[4], entry[5]]);
        let name_field = &entry[8..8 + FW_CFG_FILE_NAME_LENGTH];
        let name_end = name_field.iter().position(|&b| b == 0).unwrap_or(FW_CFG_FILE_NAME_LENGTH);

        if &name_field[..name_end] != name.as_bytes() {
            continue;
        }

        if size == 0 || size > FW_CFG_MAX_FILE_SIZE {
            log::warn!("fw_cfg file {} has unsupported size {}", name, size);
            return None;
        }

        let mut contents = alloc::vec![0u8; size];
        transport.select(item);
        transport.read_bytes(&mut contents);
        return Some(contents);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In-memory fw_cfg device holding a signature, a directory and file contents.
    struct MockFwCfg {
        items: Vec<(u16, Vec<u8>)>,
        selected: usize,
        offset: usize,
    }

    impl MockFwCfg {
        fn new(signature: &[u8], files: &[(&str, &[u8])]) -> Self {
            let mut directory = (files.len() as u32).to_be_bytes().to_vec();
            let mut items = Vec::new();
            for (index, (name, contents)) in files.iter().enumerate() {
                let item = 0x20 + index as u16;
                let mut entry = [0u8; FW_CFG_FILE_ENTRY_LENGTH];
                entry[0..4].copy_from_slice(&(contents.len() as u32).to_be_bytes());
                entry[4..6].copy_from_slice(&item.to_be_bytes());
                entry[8..8 + name.len()].copy_from_slice(name.as_bytes());
                directory.extend_from_slice(&entry);
                items.push((item, contents.to_vec()));
            }
            items.push((FW_CFG_SIGNATURE, signature.to_vec()));
            items.push((FW_CFG_FILE_DIR, directory));
            Self { items, selected: usize::MAX, offset: 0 }
        }
    }

    impl FwCfgTransport for MockFwCfg {
        fn select(&mut self, item: u16) {
            self.selected = self.items.iter().position(|(id, _)| *id == item).unwrap_or(usize::MAX);
            self.offset = 0;
        }

        fn read_bytes(&mut self, buffer: &mut [u8]) {
            for byte in buffer {
                *byte = self.items.get(self.selected).and_then(|(_, data)| data.get(self.offset)).copied().unwrap_or(0);
                self.offset += 1;
            }
        }
    }

    #[test]
    fn reads_named_file_from_directory() {
        let mut device = MockFwCfg::new(b"QEMU", &[("etc/other", b"xx"), ("etc/smbios/smbios-tables", b"tables")]);
        assert_eq!(read_file(&mut device, "etc/smbios/smbios-tables").as_deref(), Some(&b"tables"[..]));
    }

    #[test]
    fn missing_file_returns_none() {
        let mut device = MockFwCfg::new(b"QEMU", &[("etc/other", b"xx")]);
        assert!(read_file(&mut device, "etc/smbios/smbios-tables").is_none());
    }

    #[test]
    fn missing_signature_returns_none() {
        let mut device = MockFwCfg::new(b"\0\0\0\0", &[("etc/smbios/smbios-tables", b"tables")]);
        assert!(read_file(&mut device, "etc/smbios/smbios-tables").is_none());
    }

    #[test]
    fn empty_file_returns_none() {
        let mut device = MockFwCfg::new(b"QEMU", &[("etc/smbios/smbios-tables", b"")]);
        assert!(read_file(&mut device, "etc/smbios/smbios-tables").is_none());
    }

    #[test]
    fn implausible_file_count_returns_none() {
        let mut device = MockFwCfg::new(b"QEMU", &[]);
        device.items.retain(|(id, _)| *id != FW_CFG_FILE_DIR);
        device.items.push((FW_CFG_FILE_DIR, u32::MAX.to_be_bytes().to_vec()));
        assert!(read_file(&mut device, "etc/smbios/smbios-tables").is_none());
    }
}
