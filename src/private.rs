#![allow(clippy::missing_errors_doc)]

use core::ffi::c_char;
use std::ffi::{CStr, CString};
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{SpeechError, SpeechFrameworkError, SpeechFrameworkErrorCode};
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

pub unsafe fn error_from_status_or_json(status: i32, err_msg: *mut c_char) -> SpeechError {
    let Some(message) = take_string(err_msg) else {
        return crate::error::from_swift(status, std::ptr::null_mut());
    };

    if let Ok(payload) = serde_json::from_str::<FrameworkErrorPayload>(&message) {
        let kind = SpeechFrameworkErrorCode::from_domain_code_and_message(
            &payload.domain,
            payload.code,
            &payload.localized_description,
        );
        return SpeechError::Framework(SpeechFrameworkError {
            domain: payload.domain,
            code: payload.code,
            message: payload.localized_description,
            kind,
        });
    }

    with_fallback_message(crate::error::from_swift(status, std::ptr::null_mut()), message)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FrameworkErrorPayload {
    domain: String,
    code: i64,
    localized_description: String,
}

fn with_fallback_message(error: SpeechError, message: String) -> SpeechError {
    match error {
        SpeechError::NotAuthorized(_) => SpeechError::NotAuthorized(message),
        SpeechError::RecognizerUnavailable(_) => SpeechError::RecognizerUnavailable(message),
        SpeechError::AudioLoadFailed(_) => SpeechError::AudioLoadFailed(message),
        SpeechError::RecognitionFailed(_) => SpeechError::RecognitionFailed(message),
        SpeechError::TimedOut(_) => SpeechError::TimedOut(message),
        SpeechError::InvalidArgument(_) => SpeechError::InvalidArgument(message),
        SpeechError::Unknown { code, .. } => SpeechError::Unknown { code, message },
        SpeechError::Framework(framework) => SpeechError::Framework(SpeechFrameworkError {
            message,
            ..framework
        }),
    }
}
