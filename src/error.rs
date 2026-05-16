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

/// `SFSpeechErrorDomain` from `SFErrors.h`.
pub const SPEECH_ERROR_DOMAIN: &str = "SFSpeechErrorDomain";

/// Known `SFSpeechErrorCode` values from `SFErrors.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SpeechFrameworkErrorCode {
    InternalServiceError,
    AudioReadFailed,
    AudioDisordered,
    UnexpectedAudioFormat,
    NoModel,
    IncompatibleAudioFormats,
    UndefinedTemplateClassName,
    MalformedSupplementalModel,
    ModuleOutputFailed,
    AssetLocaleNotAllocated,
    TooManyAssetLocalesAllocated,
    Timeout,
    MissingParameter,
    CannotAllocateUnsupportedLocale,
    InsufficientResources,
    Unknown(i64),
}

impl SpeechFrameworkErrorCode {
    #[must_use]
    pub fn from_domain_and_code(domain: &str, code: i64) -> Self {
        Self::from_domain_code_and_message(domain, code, "")
    }

    #[must_use]
    pub fn from_domain_code_and_message(domain: &str, code: i64, message: &str) -> Self {
        if domain != SPEECH_ERROR_DOMAIN {
            return Self::Unknown(code);
        }
        match code {
            1 => Self::InternalServiceError,
            2 => {
                let lower = message.to_ascii_lowercase();
                if lower.contains("disordered") || lower.contains("out of order") {
                    Self::AudioDisordered
                } else {
                    Self::AudioReadFailed
                }
            }
            3 => Self::UnexpectedAudioFormat,
            4 => Self::NoModel,
            5 => Self::IncompatibleAudioFormats,
            7 => Self::UndefinedTemplateClassName,
            8 => Self::MalformedSupplementalModel,
            9 => Self::ModuleOutputFailed,
            10 => Self::AssetLocaleNotAllocated,
            11 => Self::TooManyAssetLocalesAllocated,
            12 => Self::Timeout,
            13 => Self::MissingParameter,
            15 => Self::CannotAllocateUnsupportedLocale,
            16 => Self::InsufficientResources,
            other => Self::Unknown(other),
        }
    }
}

/// A structured framework error returned by Apple's `Speech.framework`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeechFrameworkError {
    pub domain: String,
    pub code: i64,
    pub message: String,
    pub kind: SpeechFrameworkErrorCode,
}

impl fmt::Display for SpeechFrameworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}) [{}]", self.message, self.code, self.domain)
    }
}

impl std::error::Error for SpeechFrameworkError {}

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
    /// A structured error returned by `Speech.framework`.
    Framework(SpeechFrameworkError),
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
            Self::Framework(error) => write!(f, "speech framework error: {error}"),
            Self::Unknown { code, message } => write!(f, "speech error {code}: {message}"),
        }
    }
}

impl std::error::Error for SpeechError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Framework(error) => Some(error),
            _ => None,
        }
    }
}

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
