//! QEMU Q35 Registers
//!
//! This module defines constants for QEMU Q35 register offsets and masks,
//! including PCI Express base address and Intel I/O Controller Hub 9 (ICH9)
//! specific registers.
//!
//! ## References
//!
//! - [Intel I/O Controller Hub 9 (ICH9) Datasheet](https://www.intel.com/content/dam/doc/datasheet/io-controller-hub-9-datasheet.pdf)
//!
//! ## License
//!
//! Copyright (C) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

/// Base address for PCI Express
pub const PCI_EXPRESS_BASE_ADDRESS: u64 = 0xB0000000;

/// Intel I/O Controller Hub 9 (ICH9) registers
pub mod ich9 {
    /// ICH9 Power Management Base register offset
    pub const PMBASE: u32 = 0x40;
    /// ICH9 Power Management Base register mask
    pub const PMBASE_MASK: u16 = 0xFF00;
    /// SMI Enable offset (from PMBASE)
    pub const PMBASE_OFS_SMI_EN: u32 = 0x30;
    /// Global SMI Enable bit
    pub const SMI_EN_GBL_SMI_EN: u32 = 0x01;
    /// APMC Enable bit
    pub const SMI_EN_APMC_EN: u32 = 0x20;
    /// ICH9 General PM Control 1 register offset
    pub const GEN_PMCON_1: u32 = 0xA0;
    /// SMI Lock bit
    pub const GEN_PMCON_1_SMI_LOCK: u16 = 0x10;
}
