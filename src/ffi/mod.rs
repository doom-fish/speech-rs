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
}

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
