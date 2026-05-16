//! Raw FFI declarations matching the Swift bridge.

#![allow(missing_docs, non_camel_case_types)]

use core::ffi::{c_char, c_void};

/// Mirrors `SPTranscriptionSegmentRaw` in Speech.swift.
#[repr(C)]
pub struct TranscriptionSegmentRaw {
    pub text: *mut c_char,
    pub confidence: f32,
    pub timestamp: f64,
    pub duration: f64,
}

extern "C" {
    pub fn sp_string_free(s: *mut c_char);

    pub fn sp_authorization_status() -> i32;
    pub fn sp_request_authorization() -> i32;

    pub fn sp_recognizer_is_available(locale_id: *const c_char) -> bool;
    pub fn sp_recognizer_default_locale_identifier() -> *mut c_char;

    pub fn sp_recognize_url(
        audio_path: *const c_char,
        locale_id: *const c_char,
        out_transcript: *mut *mut c_char,
        out_segments: *mut *mut c_void,
        out_segment_count: *mut usize,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn sp_transcription_segments_free(array: *mut c_void, count: usize);

    pub fn sp_recognize_url_with_metadata(
        audio_path: *const c_char,
        locale_id: *const c_char,
        out_transcript: *mut *mut c_char,
        out_segments: *mut *mut c_void,
        out_segment_count: *mut usize,
        out_metadata: *mut RecognitionMetadataRaw,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn sp_live_recognition_start(
        locale_id: *const c_char,
        callback: LiveCallback,
        user_info: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;
    pub fn sp_live_recognition_stop(token: *mut c_void);
    pub fn sp_live_recognition_end_audio(token: *mut c_void);
    pub fn sp_live_recognition_cancel(token: *mut c_void);

    pub fn sp_recognize_url_with_custom_model(
        audio_path: *const c_char,
        locale_id: *const c_char,
        language_model_path: *const c_char,
        vocabulary_path: *const c_char,
        out_transcript: *mut *mut c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;
}

#[repr(C)]
pub struct RecognitionMetadataRaw {
    pub has_metadata: bool,
    pub speaking_rate: f64,
    pub average_pause_duration: f64,
    pub speech_start_timestamp: f64,
    pub speech_duration: f64,
}

pub type LiveCallback = unsafe extern "C" fn(
    user_info: *mut c_void,
    transcript: *const c_char,
    is_final: bool,
);

pub mod status {
    pub const OK: i32 = 0;
    pub const INVALID_ARGUMENT: i32 = -1;
    pub const NOT_AUTHORIZED: i32 = -2;
    pub const RECOGNIZER_UNAVAILABLE: i32 = -3;
    pub const AUDIO_LOAD_FAILED: i32 = -4;
    pub const RECOGNITION_FAILED: i32 = -5;
    pub const TIMED_OUT: i32 = -6;
    pub const UNKNOWN: i32 = -99;
}
