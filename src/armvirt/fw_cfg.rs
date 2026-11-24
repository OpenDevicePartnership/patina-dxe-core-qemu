//! QEMU fw_cfg Reader (MMIO interface)
//!
//! fw_cfg transport for QEMU Arm Virt. The data register is at offset 0. The selector register is at
//! offset 8 from the device base.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
#![cfg(all(target_os = "uefi", target_arch = "aarch64", feature = "aarch64"))]

extern crate alloc;
use alloc::vec::Vec;

use crate::fw_cfg::FwCfgTransport;

/// Base address of the fw_cfg MMIO region on the QEMU `virt` machine.
const FW_CFG_BASE: usize = 0x0902_0000;
const FW_CFG_DATA_OFFSET: usize = 0x0;
const FW_CFG_SELECTOR_OFFSET: usize = 0x8;

struct MmioTransport;

impl FwCfgTransport for MmioTransport {
    fn select(&mut self, item: u16) {
        // SAFETY: The fw_cfg MMIO region is described to the DXE core by the platform's resource descriptor HOBs.
        // The selector register takes a big-endian value.
        unsafe { core::ptr::write_volatile((FW_CFG_BASE + FW_CFG_SELECTOR_OFFSET) as *mut u16, item.to_be()) };
    }

    fn read_bytes(&mut self, buffer: &mut [u8]) {
        for byte in buffer {
            // SAFETY: Reads the next byte of the selected fw_cfg item from the data register.
            *byte = unsafe { core::ptr::read_volatile((FW_CFG_BASE + FW_CFG_DATA_OFFSET) as *const u8) };
        }
    }
}

/// Reads the full contents of the fw_cfg file named `name`.
///
/// Returns `None` if there is no fw_cfg device, the file does not exist, or the file is empty or too large.
pub fn read_file(name: &str) -> Option<Vec<u8>> {
    crate::fw_cfg::read_file(&mut MmioTransport, name)
}
