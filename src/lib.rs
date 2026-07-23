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
#[cfg_attr(coverage, coverage(off))]
#[cfg(any(feature = "aarch64", test))]
pub mod armvirt;
#[cfg(any(feature = "x64", test))]
pub mod q35;
