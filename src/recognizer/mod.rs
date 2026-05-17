//! [`SpeechRecognizer`] — wraps `SFSpeechRecognizer` for file-based
//! transcription and advanced Speech.framework task APIs.

#![allow(clippy::missing_const_for_fn, clippy::missing_errors_doc)]

use core::ffi::{c_char, c_void};
use core::ptr;
use std::ffi::CString;
use std::path::Path;
use std::sync::Arc;

use serde::Serialize;

use crate::error::{AuthorizationStatus, SpeechError};
use crate::ffi;
use crate::language_model::LanguageModelConfiguration;
use crate::private::{
    cstring_from_path, error_from_status, json_cstring, parse_json_ptr, take_string,
};
use crate::request::{
    AudioBufferRecognitionRequest, CallbackQueue, QueuePayload, RecognitionRequestOptions,
    TaskHint, UrlRecognitionRequest,
};
use crate::task::{
    availability_trampoline, make_availability_callback, make_task_callback, task_event_trampoline,
    AudioBufferRecognitionTask, RecognitionTask, RecognizerAvailabilityObserver,
};
use crate::transcription::{
    DetailedRecognitionMetadata, DetailedRecognitionResult, TranscriptionSegmentDetails,
};

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

/// Voice / pacing analytics returned by macOS 11+ Speech.
#[derive(Debug, Clone, PartialEq)]
pub struct RecognitionMetadata {
    /// Words per minute (or roughly equivalent unit).
    pub speaking_rate: f64,
    /// Mean inter-word pause (seconds).
    pub average_pause_duration: f64,
    /// Offset (seconds) of detected speech start within the audio.
    pub speech_start_timestamp: f64,
    /// Total seconds of detected speech.
    pub speech_duration: f64,
}

/// Result + optional metadata from
/// [`SpeechRecognizer::recognize_in_path_with_metadata`].
#[derive(Debug, Clone, PartialEq)]
pub struct RecognitionWithMetadata {
    pub result: RecognitionResult,
    /// `None` when the recogniser did not populate metadata.
    pub metadata: Option<RecognitionMetadata>,
}

/// Speech recognition engine.
#[derive(Debug, Clone)]
pub struct SpeechRecognizer {
    locale_id: Option<CString>,
    default_task_hint: TaskHint,
    callback_queue: CallbackQueue,
}

impl Default for SpeechRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecognizerPayload {
    default_task_hint: Option<i32>,
    queue: QueuePayload,
}

impl SpeechRecognizer {
    /// Construct using the device's default locale.
    #[must_use]
    pub fn new() -> Self {
        Self {
            locale_id: None,
            default_task_hint: TaskHint::Unspecified,
            callback_queue: CallbackQueue::Main,
        }
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
            default_task_hint: TaskHint::Unspecified,
            callback_queue: CallbackQueue::Main,
        })
    }

    /// Current authorization state. Cheap to call.
    #[must_use]
    pub fn authorization_status() -> AuthorizationStatus {
        AuthorizationStatus::from_raw(unsafe { ffi::sp_authorization_status() })
    }

    /// Synchronously prompt the user for authorization.
    #[must_use]
    pub fn request_authorization() -> AuthorizationStatus {
        AuthorizationStatus::from_raw(unsafe { ffi::sp_request_authorization() })
    }

    /// Returns the set of locales supported by `SFSpeechRecognizer`.
    pub fn supported_locales() -> Result<Vec<String>, SpeechError> {
        let ptr = unsafe { ffi::sp_supported_locales_json() };
        unsafe { parse_json_ptr::<Vec<String>>(ptr, "supported locales") }
    }

    /// Whether the recognizer for the configured locale is currently available.
    #[must_use]
    pub fn is_available(&self) -> bool {
        unsafe { ffi::sp_recognizer_is_available(self.locale_ptr()) }
    }

    /// The actual locale identifier Apple's recognizer resolved to.
    pub fn locale_identifier(&self) -> Result<String, SpeechError> {
        let recognizer_json = self.recognizer_json()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::sp_recognizer_locale_identifier(
                self.locale_ptr(),
                recognizer_json.as_ptr(),
                &mut err_msg,
            )
        };
        if ptr.is_null() {
            return Err(unsafe { error_from_status(ffi::status::RECOGNIZER_UNAVAILABLE, err_msg) });
        }
        unsafe { take_string(ptr) }.ok_or_else(|| {
            SpeechError::RecognizerUnavailable(
                "recognizer did not return a locale identifier".into(),
            )
        })
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

    /// Whether the recognizer can operate without network access.
    pub fn supports_on_device_recognition(&self) -> Result<bool, SpeechError> {
        let recognizer_json = self.recognizer_json()?;
        Ok(unsafe {
            ffi::sp_recognizer_supports_on_device_recognition(
                self.locale_ptr(),
                recognizer_json.as_ptr(),
            )
        })
    }

    /// The recognizer-wide default task hint applied to requests that do not override it.
    #[must_use]
    pub const fn default_task_hint(&self) -> TaskHint {
        self.default_task_hint
    }

    #[must_use]
    pub fn with_default_task_hint(mut self, default_task_hint: TaskHint) -> Self {
        self.default_task_hint = default_task_hint;
        self
    }

    pub fn set_default_task_hint(&mut self, default_task_hint: TaskHint) {
        self.default_task_hint = default_task_hint;
    }

    /// The queue used for asynchronous callbacks and delegate events.
    #[must_use]
    pub fn callback_queue(&self) -> &CallbackQueue {
        &self.callback_queue
    }

    #[must_use]
    pub fn with_callback_queue(mut self, callback_queue: CallbackQueue) -> Self {
        self.callback_queue = callback_queue;
        self
    }

    pub fn set_callback_queue(&mut self, callback_queue: CallbackQueue) {
        self.callback_queue = callback_queue;
    }

    /// Observe recognizer availability changes via `SFSpeechRecognizerDelegate`.
    pub fn observe_availability_changes<F>(
        &self,
        callback: F,
    ) -> Result<RecognizerAvailabilityObserver, SpeechError>
    where
        F: Fn(bool) + Send + Sync + 'static,
    {
        let recognizer_json = self.recognizer_json()?;
        let callback = make_availability_callback(callback);
        let callback_raw = Arc::as_ptr(&callback).cast::<c_void>().cast_mut();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let token = unsafe {
            ffi::sp_recognizer_observe_availability(
                self.locale_ptr(),
                recognizer_json.as_ptr(),
                availability_trampoline,
                callback_raw,
                &mut err_msg,
            )
        };
        if token.is_null() {
            Err(unsafe { error_from_status(ffi::status::RECOGNIZER_UNAVAILABLE, err_msg) })
        } else {
            Ok(RecognizerAvailabilityObserver::from_token(token, callback))
        }
    }

    /// Run synchronous file recognition with the full Speech.framework result surface.
    pub fn recognize_request(
        &self,
        request: &UrlRecognitionRequest,
    ) -> Result<DetailedRecognitionResult, SpeechError> {
        let path_c = cstring_from_path(request.path(), "audio path")?;
        let recognizer_json = self.recognizer_json()?;
        let request_json = request.options().to_json_cstring()?;
        let mut result_json: *mut c_char = ptr::null_mut();
        let mut err_msg: *mut c_char = ptr::null_mut();

        let status = unsafe {
            ffi::sp_recognize_url_detailed_json(
                path_c.as_ptr(),
                self.locale_ptr(),
                recognizer_json.as_ptr(),
                request_json.as_ptr(),
                &mut result_json,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status(status, err_msg) });
        }
        unsafe {
            parse_json_ptr::<DetailedRecognitionResult>(result_json, "detailed recognition result")
        }
    }

    /// Recognize speech in the audio file at `path`.
    pub fn recognize_in_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<RecognitionResult, SpeechError> {
        let request = UrlRecognitionRequest::new(path);
        let detailed = self.recognize_request(&request)?;
        Ok(simple_result_from_detailed(&detailed))
    }

    /// Like [`Self::recognize_in_path`] but also returns Apple's
    /// speech-recognition metadata.
    pub fn recognize_in_path_with_metadata(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<RecognitionWithMetadata, SpeechError> {
        let request = UrlRecognitionRequest::new(path);
        let detailed = self.recognize_request(&request)?;
        Ok(RecognitionWithMetadata {
            result: simple_result_from_detailed(&detailed),
            metadata: detailed
                .speech_recognition_metadata
                .as_ref()
                .map(legacy_metadata_from_detailed),
        })
    }

    /// Recognise audio at `path` against a custom on-device language
    /// model built with `SFSpeechLanguageModel.prepareCustomLanguageModel`.
    pub fn recognize_in_path_with_custom_model(
        &self,
        audio_path: impl AsRef<Path>,
        language_model: impl AsRef<Path>,
        vocabulary: Option<&Path>,
    ) -> Result<String, SpeechError> {
        let mut options =
            RecognitionRequestOptions::new().with_requires_on_device_recognition(true);
        let mut configuration = LanguageModelConfiguration::new(language_model);
        if let Some(vocabulary) = vocabulary {
            configuration = configuration.with_vocabulary(vocabulary);
        }
        options.set_customized_language_model(configuration);
        let request = UrlRecognitionRequest::new(audio_path).with_options(options);
        let detailed = self.recognize_request(&request)?;
        Ok(detailed.transcript().to_owned())
    }

    /// Start an asynchronous URL-based recognition task using Speech's delegate pipeline.
    pub fn start_url_task<F>(
        &self,
        request: &UrlRecognitionRequest,
        callback: F,
    ) -> Result<RecognitionTask, SpeechError>
    where
        F: Fn(crate::task::RecognitionTaskEvent) + Send + Sync + 'static,
    {
        let path_c = cstring_from_path(request.path(), "audio path")?;
        let recognizer_json = self.recognizer_json()?;
        let request_json = request.options().to_json_cstring()?;
        let callback = make_task_callback(callback);
        let callback_raw = Arc::as_ptr(&callback).cast::<c_void>().cast_mut();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let token = unsafe {
            ffi::sp_start_url_task(
                path_c.as_ptr(),
                self.locale_ptr(),
                recognizer_json.as_ptr(),
                request_json.as_ptr(),
                task_event_trampoline,
                callback_raw,
                &mut err_msg,
            )
        };
        if token.is_null() {
            Err(unsafe { error_from_status(ffi::status::RECOGNIZER_UNAVAILABLE, err_msg) })
        } else {
            Ok(RecognitionTask::from_token(token, callback))
        }
    }

    /// Start a manually-fed `SFSpeechAudioBufferRecognitionRequest`.
    pub fn start_audio_buffer_task<F>(
        &self,
        request: &AudioBufferRecognitionRequest,
        callback: F,
    ) -> Result<AudioBufferRecognitionTask, SpeechError>
    where
        F: Fn(crate::task::RecognitionTaskEvent) + Send + Sync + 'static,
    {
        let recognizer_json = self.recognizer_json()?;
        let request_json = request.options().to_json_cstring()?;
        let callback = make_task_callback(callback);
        let callback_raw = Arc::as_ptr(&callback).cast::<c_void>().cast_mut();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let token = unsafe {
            ffi::sp_start_audio_buffer_task(
                self.locale_ptr(),
                recognizer_json.as_ptr(),
                request_json.as_ptr(),
                task_event_trampoline,
                callback_raw,
                &mut err_msg,
            )
        };
        if token.is_null() {
            Err(unsafe { error_from_status(ffi::status::RECOGNIZER_UNAVAILABLE, err_msg) })
        } else {
            Ok(AudioBufferRecognitionTask::from_token(token, callback))
        }
    }

    /// Start microphone capture backed by `SFSpeechAudioBufferRecognitionRequest`.
    pub fn start_microphone_task<F>(
        &self,
        request: &AudioBufferRecognitionRequest,
        callback: F,
    ) -> Result<AudioBufferRecognitionTask, SpeechError>
    where
        F: Fn(crate::task::RecognitionTaskEvent) + Send + Sync + 'static,
    {
        let recognizer_json = self.recognizer_json()?;
        let request_json = request.options().to_json_cstring()?;
        let callback = make_task_callback(callback);
        let callback_raw = Arc::as_ptr(&callback).cast::<c_void>().cast_mut();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let token = unsafe {
            ffi::sp_start_microphone_task(
                self.locale_ptr(),
                recognizer_json.as_ptr(),
                request_json.as_ptr(),
                task_event_trampoline,
                callback_raw,
                &mut err_msg,
            )
        };
        if token.is_null() {
            Err(unsafe { error_from_status(ffi::status::RECOGNIZER_UNAVAILABLE, err_msg) })
        } else {
            Ok(AudioBufferRecognitionTask::from_token(token, callback))
        }
    }

    fn locale_ptr(&self) -> *const c_char {
        self.locale_id
            .as_ref()
            .map_or(ptr::null(), |value| value.as_ptr())
    }

    fn recognizer_json(&self) -> Result<CString, SpeechError> {
        json_cstring(
            &RecognizerPayload {
                default_task_hint: Some(self.default_task_hint.as_raw()),
                queue: QueuePayload::from(&self.callback_queue),
            },
            "recognizer configuration",
        )
    }

    /// Public(crate) accessor for the `async` feature — returns the recognizer
    /// JSON `CString` so `async_api` can pass it to the Swift thunk.
    pub(crate) fn recognizer_json_cstring(&self) -> Result<CString, SpeechError> {
        self.recognizer_json()
    }

    /// Public(crate) accessor for the `async` feature — returns an `Option<CString>`
    /// for the locale identifier, or `None` for the default locale.
    pub(crate) fn locale_cstring(&self) -> Option<CString> {
        self.locale_id.clone()
    }
}

fn simple_result_from_detailed(detailed: &DetailedRecognitionResult) -> RecognitionResult {
    RecognitionResult {
        transcript: detailed.best_transcription.formatted_string.clone(),
        segments: detailed
            .best_transcription
            .segments
            .iter()
            .map(simple_segment_from_detailed)
            .collect(),
    }
}

fn simple_segment_from_detailed(segment: &TranscriptionSegmentDetails) -> TranscriptionSegment {
    TranscriptionSegment {
        text: segment.substring.clone(),
        confidence: segment.confidence,
        timestamp: segment.timestamp,
        duration: segment.duration,
    }
}

fn legacy_metadata_from_detailed(metadata: &DetailedRecognitionMetadata) -> RecognitionMetadata {
    RecognitionMetadata {
        speaking_rate: metadata.speaking_rate,
        average_pause_duration: metadata.average_pause_duration,
        speech_start_timestamp: metadata.speech_start_timestamp,
        speech_duration: metadata.speech_duration,
    }
}
