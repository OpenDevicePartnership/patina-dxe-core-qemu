//! QEMU fw_cfg Reader (I/O port interface)
//!
//! fw_cfg transport for x86 using the selector (0x510) and data (0x511) I/O ports.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
#![cfg(all(target_os = "uefi", target_arch = "x86_64", feature = "x64"))]

extern crate alloc;
use alloc::vec::Vec;

use x86_64::instructions::port::Port;

use crate::fw_cfg::FwCfgTransport;

const FW_CFG_PORT_SELECTOR: u16 = 0x510;
const FW_CFG_PORT_DATA: u16 = 0x511;

struct PortTransport;

impl FwCfgTransport for PortTransport {
    fn select(&mut self, item: u16) {
        // SAFETY: Writing the item selector has no side effects other than selecting the item to read. The port is
        // only accessed on the QEMU Q35 platform.
        unsafe { Port::<u16>::new(FW_CFG_PORT_SELECTOR).write(item) };
    }

    fn read_bytes(&mut self, buffer: &mut [u8]) {
        let mut data = Port::<u8>::new(FW_CFG_PORT_DATA);
        for byte in buffer {
            // SAFETY: Reads the next byte of the selected fw_cfg item.
            *byte = unsafe { data.read() };
        }
    }
}

/// Reads the full contents of the fw_cfg file named `name`.
///
/// Returns `None` if there is no fw_cfg device, the file does not exist, or the file is empty or too large.
pub fn read_file(name: &str) -> Option<Vec<u8>> {
    crate::fw_cfg::read_file(&mut PortTransport, name)
}
