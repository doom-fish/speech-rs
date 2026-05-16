#![allow(
    clippy::missing_const_for_fn,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions
)]

use core::ffi::c_void;
use std::ptr;

use serde::Deserialize;

use crate::analyzer::{modules_json_cstring, SpeechModuleDescriptor};
use crate::error::SpeechError;
use crate::ffi;
use crate::private::{cstring_from_str, error_from_status_or_json, parse_json_ptr};

/// `AssetInventory.Status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AssetInventoryStatus {
    Unsupported,
    Supported,
    Downloading,
    Installed,
}

impl AssetInventoryStatus {
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::Supported,
            2 => Self::Downloading,
            3 => Self::Installed,
            _ => Self::Unsupported,
        }
    }
}

/// Snapshot of `Progress` from an `AssetInstallationRequest`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetInstallationProgress {
    pub fraction_completed: f64,
    pub completed_unit_count: i64,
    pub total_unit_count: i64,
    pub is_finished: bool,
    pub localized_description: Option<String>,
    pub localized_additional_description: Option<String>,
}

/// RAII wrapper around a retained `AssetInstallationRequest`.
pub struct AssetInstallationRequest {
    token: *mut c_void,
}

unsafe impl Send for AssetInstallationRequest {}
unsafe impl Sync for AssetInstallationRequest {}

impl Drop for AssetInstallationRequest {
    fn drop(&mut self) {
        if !self.token.is_null() {
            unsafe { ffi::sp_asset_installation_request_release(self.token) };
            self.token = ptr::null_mut();
        }
    }
}

impl AssetInstallationRequest {
    pub(crate) const fn from_token(token: *mut c_void) -> Self {
        Self { token }
    }

    #[must_use]
    pub const fn as_raw(&self) -> *mut c_void {
        self.token
    }

    pub fn progress(&self) -> Result<AssetInstallationProgress, SpeechError> {
        let ptr = unsafe { ffi::sp_asset_installation_request_progress_json(self.token) };
        unsafe { parse_json_ptr::<AssetInstallationProgress>(ptr, "asset installation progress") }
    }

    pub fn download_and_install(&self) -> Result<(), SpeechError> {
        let mut err_msg = ptr::null_mut();
        let status = unsafe {
            ffi::sp_asset_installation_request_download_and_install(self.token, &mut err_msg)
        };
        if status == ffi::status::OK {
            Ok(())
        } else {
            Err(unsafe { error_from_status_or_json(status, err_msg) })
        }
    }
}

/// Static helpers for reserving, querying, and preparing downloadable Speech assets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct AssetInventory;

impl AssetInventory {
    pub fn maximum_reserved_locales() -> Result<usize, SpeechError> {
        let mut value = 0usize;
        let mut err_msg = ptr::null_mut();
        let status = unsafe {
            ffi::sp_asset_inventory_maximum_reserved_locales(&mut value, &mut err_msg)
        };
        if status == ffi::status::OK {
            Ok(value)
        } else {
            Err(unsafe { error_from_status_or_json(status, err_msg) })
        }
    }

    pub fn reserved_locales() -> Result<Vec<String>, SpeechError> {
        let mut json = ptr::null_mut();
        let mut err_msg = ptr::null_mut();
        let status = unsafe { ffi::sp_asset_inventory_reserved_locales_json(&mut json, &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status_or_json(status, err_msg) });
        }
        unsafe { parse_json_ptr::<Vec<String>>(json, "reserved locales") }
    }

    pub fn reserve_locale(locale_identifier: &str) -> Result<bool, SpeechError> {
        let locale_identifier = cstring_from_str(locale_identifier, "asset locale identifier")?;
        let mut reserved = false;
        let mut err_msg = ptr::null_mut();
        let status = unsafe {
            ffi::sp_asset_inventory_reserve_locale(
                locale_identifier.as_ptr(),
                &mut reserved,
                &mut err_msg,
            )
        };
        if status == ffi::status::OK {
            Ok(reserved)
        } else {
            Err(unsafe { error_from_status_or_json(status, err_msg) })
        }
    }

    pub fn release_reserved_locale(locale_identifier: &str) -> Result<bool, SpeechError> {
        let locale_identifier = cstring_from_str(locale_identifier, "asset locale identifier")?;
        let mut released = false;
        let mut err_msg = ptr::null_mut();
        let status = unsafe {
            ffi::sp_asset_inventory_release_locale(
                locale_identifier.as_ptr(),
                &mut released,
                &mut err_msg,
            )
        };
        if status == ffi::status::OK {
            Ok(released)
        } else {
            Err(unsafe { error_from_status_or_json(status, err_msg) })
        }
    }

    pub fn status_for_modules<M, I>(modules: I) -> Result<AssetInventoryStatus, SpeechError>
    where
        I: IntoIterator<Item = M>,
        M: Into<SpeechModuleDescriptor>,
    {
        let module_descriptors = modules.into_iter().map(Into::into).collect::<Vec<_>>();
        let modules_json = modules_json_cstring(&module_descriptors, "asset inventory modules")?;
        let mut raw_status = 0i32;
        let mut err_msg = ptr::null_mut();
        let status = unsafe {
            ffi::sp_asset_inventory_status_for_modules(
                modules_json.as_ptr(),
                &mut raw_status,
                &mut err_msg,
            )
        };
        if status == ffi::status::OK {
            Ok(AssetInventoryStatus::from_raw(raw_status))
        } else {
            Err(unsafe { error_from_status_or_json(status, err_msg) })
        }
    }

    pub fn asset_installation_request_for_modules<M, I>(
        modules: I,
    ) -> Result<Option<AssetInstallationRequest>, SpeechError>
    where
        I: IntoIterator<Item = M>,
        M: Into<SpeechModuleDescriptor>,
    {
        let module_descriptors = modules.into_iter().map(Into::into).collect::<Vec<_>>();
        let modules_json = modules_json_cstring(&module_descriptors, "asset inventory modules")?;
        let mut has_request = false;
        let mut err_msg = ptr::null_mut();
        let token = unsafe {
            ffi::sp_asset_inventory_installation_request_for_modules(
                modules_json.as_ptr(),
                &mut has_request,
                &mut err_msg,
            )
        };
        if !err_msg.is_null() {
            return Err(unsafe { error_from_status_or_json(ffi::status::RECOGNITION_FAILED, err_msg) });
        }
        Ok(has_request.then(|| AssetInstallationRequest::from_token(token)))
    }
}
