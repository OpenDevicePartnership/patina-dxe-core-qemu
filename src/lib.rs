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
#![cfg_attr(
    any(test, feature = "aarch64", all(feature = "x64", feature = "dxe_core")),
    feature(coverage_attribute)
)]

#[cfg(any(feature = "aarch64", test))]
pub mod armvirt;
#[cfg(any(feature = "x64", test))]
pub mod q35;
