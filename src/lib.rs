//! QEMU Resources
//!
//! This module provides resources such as components and services used in the QEMU platform.
//!
//! ## License
//!
//! Copyright (C) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
#![no_std]
#![cfg_attr(coverage, feature(coverage_attribute))]
#[cfg_attr(coverage, coverage(off))]
#[cfg(any(feature = "aarch64", test))]
pub mod armvirt;
#[cfg(any(feature = "x64", test))]
pub mod q35;

// This is a placeholder test to allow test commands to run without
// error given there are no other unit tests in this crate right now.
#[cfg(test)]
mod tests {
    #[test]
    fn placeholder() {}
}
