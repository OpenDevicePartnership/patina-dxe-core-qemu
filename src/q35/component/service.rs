//! QEMU Q35 Services
//!
//! Services used in the QEMU Q35 Patina binary.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
#[cfg_attr(coverage, coverage(off))]
pub mod mm_config_provider;
#[cfg_attr(coverage, coverage(off))]
pub mod mm_control;
#[cfg_attr(coverage, coverage(off))]
pub mod mm_test;
#[cfg_attr(coverage, coverage(off))]
pub mod smbios_platform;
#[cfg_attr(coverage, coverage(off))]
pub mod smbios_test;
