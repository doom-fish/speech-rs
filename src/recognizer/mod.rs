//! [`SpeechRecognizer`] — wraps `SFSpeechRecognizer` for file-based
//! transcription.

use core::ffi::{c_char, c_void};
use core::ptr;
use std::ffi::CString;
use std::path::Path;

use crate::error::{from_swift, AuthorizationStatus, SpeechError};
use crate::ffi;

/// One transcription segment with its position in the audio.
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptionSegment {
    pub text: String,
    /// Confidence in `0.0..=1.0`.
    pub confidence: f32,
    /// Position in the audio file (seconds since start).
    pub timestamp: f64,
    /// Duration of this segment (seconds).
    pub duration: f64,
}

/// Result of speech recognition.
#[derive(Debug, Clone, PartialEq)]
pub struct RecognitionResult {
    /// Full transcript, with capitalisation and punctuation Vision applies.
    pub transcript: String,
    /// Per-segment breakdown (one element per recognised word/phrase).
    pub segments: Vec<TranscriptionSegment>,
}

/// Speech recognition engine.
///
/// # Authorization
///
/// `SFSpeechRecognizer` requires user authorization. Check with
/// [`SpeechRecognizer::authorization_status`] and trigger the prompt with
/// [`SpeechRecognizer::request_authorization`]. CLI / daemon binaries
/// without an `Info.plist` will typically get `Denied`.
///
/// # Examples
///
/// ```rust,no_run
/// use speech::prelude::*;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// if !SpeechRecognizer::authorization_status().is_authorized() {
///     let new_status = SpeechRecognizer::request_authorization();
///     if !new_status.is_authorized() {
///         eprintln!("authorization denied: {new_status:?}");
///         return Ok(());
///     }
/// }
///
/// let recognizer = SpeechRecognizer::new();
/// let result = recognizer.recognize_in_path("/tmp/utterance.aiff")?;
/// println!("{}", result.transcript);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct SpeechRecognizer {
    locale_id: Option<CString>,
}

impl Default for SpeechRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeechRecognizer {
    /// Construct using the device's default locale.
    #[must_use]
    pub const fn new() -> Self {
        Self { locale_id: None }
    }

    /// Construct using the recognizer for `locale_id` (e.g. `"en-US"`,
    /// `"sv-SE"`).
    ///
    /// # Panics
    ///
    /// Panics if `locale_id` contains an interior NUL byte. Use
    /// [`Self::with_locale_checked`] for the fallible form.
    #[must_use]
    pub fn with_locale(locale_id: &str) -> Self {
        Self::with_locale_checked(locale_id).expect("locale must not contain NUL bytes")
    }

    /// Same as [`Self::with_locale`] but returns `None` when `locale_id`
    /// has interior NUL bytes.
    #[must_use]
    pub fn with_locale_checked(locale_id: &str) -> Option<Self> {
        Some(Self {
            locale_id: Some(CString::new(locale_id).ok()?),
        })
    }

    /// Current authorization state. Cheap to call.
    #[must_use]
    pub fn authorization_status() -> AuthorizationStatus {
        AuthorizationStatus::from_raw(unsafe { ffi::sp_authorization_status() })
    }

    /// Synchronously prompt the user for authorization. Blocks until the
    /// system responds (or 30 seconds elapse). Returns the resulting status.
    ///
    /// CLI binaries without a proper `Info.plist` will typically get
    /// [`AuthorizationStatus::Denied`].
    #[must_use]
    pub fn request_authorization() -> AuthorizationStatus {
        AuthorizationStatus::from_raw(unsafe { ffi::sp_request_authorization() })
    }

    /// Whether the on-device recognizer for the configured locale is
    /// available right now.
    #[must_use]
    pub fn is_available(&self) -> bool {
        let p = self
            .locale_id
            .as_ref()
            .map_or(ptr::null(), |c| c.as_ptr());
        unsafe { ffi::sp_recognizer_is_available(p) }
    }

    /// Identifier of the device's default speech-recognizer locale (or
    /// `None` if no recognizer is installed).
    #[must_use]
    pub fn default_locale_identifier() -> Option<String> {
        let p = unsafe { ffi::sp_recognizer_default_locale_identifier() };
        if p.is_null() {
            return None;
        }
        let s = unsafe { core::ffi::CStr::from_ptr(p) }
            .to_string_lossy()
            .into_owned();
        unsafe { ffi::sp_string_free(p) };
        Some(s)
    }

    /// Recognize speech in the audio file at `path`. Supports any audio
    /// format `AVFoundation` can read (AIFF, WAV, M4A, MP3, ...).
    ///
    /// Forces on-device recognition (no Apple-server round-trip).
    ///
    /// # Errors
    ///
    /// Returns [`SpeechError::NotAuthorized`] when the user hasn't granted
    /// authorization, [`SpeechError::AudioLoadFailed`] / [`SpeechError::RecognizerUnavailable`]
    /// for setup failures, or [`SpeechError::RecognitionFailed`] /
    /// [`SpeechError::TimedOut`] for runtime failures.
    pub fn recognize_in_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<RecognitionResult, SpeechError> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| SpeechError::InvalidArgument("non-UTF-8 path".into()))?;
        let path_c = CString::new(path_str)
            .map_err(|e| SpeechError::InvalidArgument(format!("path NUL byte: {e}")))?;

        let locale_p = self
            .locale_id
            .as_ref()
            .map_or(ptr::null(), |c| c.as_ptr());

        let mut transcript_raw: *mut c_char = ptr::null_mut();
        let mut segments_raw: *mut c_void = ptr::null_mut();
        let mut segment_count: usize = 0;
        let mut err_msg: *mut c_char = ptr::null_mut();

        let status = unsafe {
            ffi::sp_recognize_url(
                path_c.as_ptr(),
                locale_p,
                &mut transcript_raw,
                &mut segments_raw,
                &mut segment_count,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }

        let transcript = if transcript_raw.is_null() {
            String::new()
        } else {
            let s = unsafe { core::ffi::CStr::from_ptr(transcript_raw) }
                .to_string_lossy()
                .into_owned();
            unsafe { ffi::sp_string_free(transcript_raw) };
            s
        };

        let segments = if segments_raw.is_null() || segment_count == 0 {
            Vec::new()
        } else {
            let typed = segments_raw.cast::<ffi::TranscriptionSegmentRaw>();
            let mut v = Vec::with_capacity(segment_count);
            for i in 0..segment_count {
                let raw = unsafe { &*typed.add(i) };
                let text = if raw.text.is_null() {
                    String::new()
                } else {
                    unsafe { core::ffi::CStr::from_ptr(raw.text) }
                        .to_string_lossy()
                        .into_owned()
                };
                v.push(TranscriptionSegment {
                    text,
                    confidence: raw.confidence,
                    timestamp: raw.timestamp,
                    duration: raw.duration,
                });
            }
            unsafe { ffi::sp_transcription_segments_free(segments_raw, segment_count) };
            v
        };

        Ok(RecognitionResult {
            transcript,
            segments,
        })
    }
}
