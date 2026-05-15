//! Errors returned by the Speech bridge.

use core::fmt;

use crate::ffi;

/// Authorization state returned by Apple's `SFSpeechRecognizer.authorizationStatus()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AuthorizationStatus {
    NotDetermined,
    Denied,
    Restricted,
    Authorized,
    Unknown,
}

impl AuthorizationStatus {
    pub(crate) const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::NotDetermined,
            1 => Self::Denied,
            2 => Self::Restricted,
            3 => Self::Authorized,
            _ => Self::Unknown,
        }
    }

    /// Convenience: is this state `Authorized`?
    #[must_use]
    pub const fn is_authorized(self) -> bool {
        matches!(self, Self::Authorized)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SpeechError {
    /// Speech recognition is not authorized — see [`AuthorizationStatus`].
    NotAuthorized(String),
    /// `SFSpeechRecognizer` is unavailable for the requested locale (or the
    /// device is offline and the locale doesn't support on-device recognition).
    RecognizerUnavailable(String),
    /// The audio file at the given path could not be loaded.
    AudioLoadFailed(String),
    /// `SFSpeechRecognitionTask` reported an error.
    RecognitionFailed(String),
    /// Recognition didn't produce a final result within the configured timeout.
    TimedOut(String),
    /// Caller supplied an invalid argument (e.g. NUL byte in path).
    InvalidArgument(String),
    /// Catch-all for unmapped statuses from the Swift bridge.
    Unknown { code: i32, message: String },
}

impl fmt::Display for SpeechError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAuthorized(m) => write!(f, "speech recognition not authorized: {m}"),
            Self::RecognizerUnavailable(m) => write!(f, "recognizer unavailable: {m}"),
            Self::AudioLoadFailed(m) => write!(f, "audio load failed: {m}"),
            Self::RecognitionFailed(m) => write!(f, "recognition failed: {m}"),
            Self::TimedOut(m) => write!(f, "timed out: {m}"),
            Self::InvalidArgument(m) => write!(f, "invalid argument: {m}"),
            Self::Unknown { code, message } => write!(f, "speech error {code}: {message}"),
        }
    }
}

impl std::error::Error for SpeechError {}

pub(crate) unsafe fn from_swift(status: i32, error_str: *mut core::ffi::c_char) -> SpeechError {
    let message = if error_str.is_null() {
        String::new()
    } else {
        let s = core::ffi::CStr::from_ptr(error_str)
            .to_string_lossy()
            .into_owned();
        ffi::sp_string_free(error_str);
        s
    };
    match status {
        ffi::status::NOT_AUTHORIZED => SpeechError::NotAuthorized(message),
        ffi::status::RECOGNIZER_UNAVAILABLE => SpeechError::RecognizerUnavailable(message),
        ffi::status::AUDIO_LOAD_FAILED => SpeechError::AudioLoadFailed(message),
        ffi::status::RECOGNITION_FAILED => SpeechError::RecognitionFailed(message),
        ffi::status::TIMED_OUT => SpeechError::TimedOut(message),
        ffi::status::INVALID_ARGUMENT => SpeechError::InvalidArgument(message),
        code => SpeechError::Unknown { code, message },
    }
}
