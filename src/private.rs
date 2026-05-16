#![allow(clippy::missing_errors_doc)]

use core::ffi::c_char;
use std::ffi::{CStr, CString};
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::SpeechError;
use crate::ffi;

pub fn cstring_from_str(value: &str, context: &str) -> Result<CString, SpeechError> {
    CString::new(value)
        .map_err(|e| SpeechError::InvalidArgument(format!("{context} contains NUL byte: {e}")))
}

pub fn cstring_from_path(path: &Path, context: &str) -> Result<CString, SpeechError> {
    let value = path
        .to_str()
        .ok_or_else(|| SpeechError::InvalidArgument(format!("{context} is not valid UTF-8")))?;
    cstring_from_str(value, context)
}

pub fn json_cstring<T: Serialize>(value: &T, context: &str) -> Result<CString, SpeechError> {
    let json = serde_json::to_string(value).map_err(|e| {
        SpeechError::InvalidArgument(format!("failed to encode {context} as JSON: {e}"))
    })?;
    cstring_from_str(&json, context)
}

pub unsafe fn take_string(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let string = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    ffi::sp_string_free(ptr);
    Some(string)
}

pub unsafe fn parse_json_ptr<T: DeserializeOwned>(
    ptr: *mut c_char,
    context: &str,
) -> Result<T, SpeechError> {
    let json = take_string(ptr).ok_or_else(|| {
        SpeechError::InvalidArgument(format!("missing JSON payload for {context}"))
    })?;
    serde_json::from_str(&json).map_err(|e| {
        SpeechError::InvalidArgument(format!(
            "failed to parse {context} JSON: {e}; payload={json}"
        ))
    })
}

pub unsafe fn error_from_status(status: i32, err_msg: *mut c_char) -> SpeechError {
    crate::error::from_swift(status, err_msg)
}
