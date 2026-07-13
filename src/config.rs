//! MSVM Configuration
//!
//! These definitions must be kept in sync with
//! MsvPkg/Include/Guid/PatinaConfigHob.h
//!
//! Copyright (C) Microsoft Corporation.
//!
//!

use core::ffi::c_void;
use patina::{
    BinaryGuid,
    error::EfiError,
    pi::hob::{Hob, PhaseHandoffInformationTable},
};
use zerocopy::{FromBytes, Immutable, KnownLayout, Ref};

const MSVM_PATINA_CONFIG_HOB_GUID: BinaryGuid = BinaryGuid::from_bytes(&[
    0x1f, 0xeb, 0x5a, 0x86, 0x76, 0x95, 0x4a, 0x5c, 0x93, 0x49, 0x7d, 0x24, 0x3a, 0x43, 0x99, 0x49,
]);

const MSVM_PATINA_CONFIG_HOB_VERSION_MAJOR: u32 = 1;
const MSVM_PATINA_CONFIG_HOB_VERSION_MINOR: u32 = 0;

// #pragma pack(push, 1)
// typedef struct _MSVM_PATINA_CONFIG {
//   UINT32                  VersionMajor;
//   UINT32                  VersionMinor;
// #if defined (MDE_CPU_AARCH64)
//   EFI_PHYSICAL_ADDRESS    GicDistributorBase;
//   EFI_PHYSICAL_ADDRESS    GicRedistributorBase;
// #endif
// } MSVM_PATINA_CONFIG;
// #pragma pack(pop)
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, FromBytes, KnownLayout, Immutable)]
pub struct MsvmPatinaConfig {
    pub version_major: u32,
    pub version_minor: u32,

    #[cfg(target_arch = "aarch64")]
    pub gic_distributor_base: r_efi::efi::PhysicalAddress,
    #[cfg(target_arch = "aarch64")]
    pub gic_redistributor_base: r_efi::efi::PhysicalAddress,
}

impl MsvmPatinaConfig {
    /// Find the MSVM Patina Config HOB in the given HOB list and returns it.
    ///
    /// # Safety
    /// The caller must provide a valid physical HOB list pointer.
    pub unsafe fn from_hob_list(hob_list: *const c_void) -> Result<Self, EfiError> {
        debug_assert!(!hob_list.is_null(), "Cannot initialize MsvmPatinaConfig from null HOB list");
        let hob_list_info =
            // SAFETY: The caller must provide a valid physical HOB list pointer.
            unsafe { (hob_list as *const PhaseHandoffInformationTable).as_ref() }.ok_or_else(|| {
                log::error!("Could not find MSVM Patina Config HOB due to null hob list.");
                EfiError::InvalidParameter
            })?;
        let hob_list = Hob::Handoff(hob_list_info);
        for hob in &hob_list {
            if let Hob::GuidHob(guid_hob, data) = hob
                && guid_hob.name == MSVM_PATINA_CONFIG_HOB_GUID
            {
                let config = Ref::<_, MsvmPatinaConfig>::from_bytes(data).map_err(|_| {
                    log::error!("MSVM Patina Config HOB payload has invalid size/alignment");
                    EfiError::InvalidParameter
                })?;

                if config.version_major != MSVM_PATINA_CONFIG_HOB_VERSION_MAJOR {
                    log::error!("MSVM Patina Config HOB has incorrect major version");
                    return Err(EfiError::InvalidParameter);
                }

                return Ok(*config);
            }
        }

        Err(EfiError::NotFound)
    }
}
