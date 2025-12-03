//! QEMU Q35 Management Mode (MM) Test Component
//!
//! Verifies that MM interfaces are working as expected on the QEMU Q35 platform. By exercising a MM communication
//! transaction to the MM Supervisor.
//!
//! ## License
//!
//! Copyright (C) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use patina::component::{component, service::Service};
use patina_mm::service::MmCommunication;

/// MM Supervisor Request Header
///
/// Used to request information from the MM Supervisor.
///
/// ## Notes
///
/// - This structure is only defined here for test purposes.
#[repr(C, packed(1))]
struct MmSupervisorRequestHeader {
    signature: u32,
    revision: u32,
    request: u32,
    reserved: u32,
    result: u64,
}

/// MM Supervisor Version Info
///
/// Populated by the MM Supervisor in response to a version request.
///
/// ## Notes
///
/// - This structure is only defined here for test purposes.
#[repr(C, packed(1))]
struct MmSupervisorVersionInfo {
    version: u32,
    patch_level: u32,
    max_supervisor_request_level: u64,
}

/// QEMU Q35 MM Test Component
///
/// Responsible for testing the MM communication interface on the QEMU Q35 platform.
#[derive(Default)]
pub struct QemuQ35MmTest;

#[component]
impl QemuQ35MmTest {
    /// Creates a new instance of the QEMU Q35 MM Test component.
    pub fn new() -> Self {
        Self
    }

    /// Entry point for the MM Test component.
    ///
    /// Uses the `MmCommunication` service to send a request version information from the MM Supervisor. The MM
    /// Supervisor is expected to be the Standalone MM environment used on the QEMU Q35 platform.
    pub fn entry_point(self, mm_comm: Service<dyn MmCommunication>) -> patina::error::Result<()> {
        log::debug!("MM Test Entry Point - Testing MM Communication");

        let mm_supv_req_header = MmSupervisorRequestHeader {
            signature: u32::from_le_bytes([b'M', b'S', b'U', b'P']),
            revision: 1,
            request: 0x0003, // Request Version Info
            reserved: 0,
            result: 0,
        };

        let result = unsafe {
            mm_comm
                .communicate(
                    0,
                    core::slice::from_raw_parts(
                        &mm_supv_req_header as *const _ as *const u8,
                        core::mem::size_of::<MmSupervisorRequestHeader>(),
                    ),
                    patina::Guid::from_fields(
                        0x8c633b23,
                        0x1260,
                        0x4ea6,
                        0x83,
                        0x0F,
                        [0x7d, 0xdc, 0x97, 0x38, 0x21, 0x11],
                    ),
                )
                .map_err(|_| {
                    log::error!("MM Communication failed");
                    patina::error::EfiError::DeviceError // Todo: Map actual codes
                })?
        };

        let mm_supv_ver_info = unsafe {
            &*(result[core::mem::size_of::<MmSupervisorRequestHeader>()..].as_ptr() as *const MmSupervisorVersionInfo)
        };
        let version = mm_supv_ver_info.version;
        let patch_level = mm_supv_ver_info.patch_level;
        let max_request_level = mm_supv_ver_info.max_supervisor_request_level;
        log::info!(
            "MM Supervisor Version: {version:#X}, Patch Level: {patch_level:#X}, Max Request Level: {max_request_level:#X}",
        );

        Ok(())
    }
}
