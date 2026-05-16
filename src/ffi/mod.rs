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

    pub fn sp_supported_locales_json() -> *mut c_char;
    pub fn sp_recognizer_locale_identifier(
        locale_id: *const c_char,
        recognizer_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;
    pub fn sp_recognizer_supports_on_device_recognition(
        locale_id: *const c_char,
        recognizer_json: *const c_char,
    ) -> bool;
    pub fn sp_recognizer_observe_availability(
        locale_id: *const c_char,
        recognizer_json: *const c_char,
        callback: AvailabilityCallback,
        user_info: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;
    pub fn sp_recognizer_availability_observer_stop(token: *mut c_void);

    pub fn sp_recognize_url_detailed_json(
        audio_path: *const c_char,
        locale_id: *const c_char,
        recognizer_json: *const c_char,
        request_json: *const c_char,
        out_result_json: *mut *mut c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn sp_start_url_task(
        audio_path: *const c_char,
        locale_id: *const c_char,
        recognizer_json: *const c_char,
        request_json: *const c_char,
        callback: TaskEventCallback,
        user_info: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;
    pub fn sp_start_audio_buffer_task(
        locale_id: *const c_char,
        recognizer_json: *const c_char,
        request_json: *const c_char,
        callback: TaskEventCallback,
        user_info: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;
    pub fn sp_start_microphone_task(
        locale_id: *const c_char,
        recognizer_json: *const c_char,
        request_json: *const c_char,
        callback: TaskEventCallback,
        user_info: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;
    pub fn sp_task_finish(token: *mut c_void);
    pub fn sp_task_cancel(token: *mut c_void);
    pub fn sp_task_state(token: *mut c_void) -> i32;
    pub fn sp_task_is_finishing(token: *mut c_void) -> bool;
    pub fn sp_task_is_cancelled(token: *mut c_void) -> bool;
    pub fn sp_task_error_json(token: *mut c_void) -> *mut c_char;
    pub fn sp_task_release(token: *mut c_void);

    pub fn sp_audio_buffer_request_native_format_json() -> *mut c_char;
    pub fn sp_audio_buffer_task_end_audio(token: *mut c_void);
    pub fn sp_audio_buffer_task_native_format_json(token: *mut c_void) -> *mut c_char;
    pub fn sp_audio_buffer_task_append_f32(
        token: *mut c_void,
        samples: *const f32,
        sample_count: usize,
        sample_rate: f64,
        channels: i32,
        interleaved: bool,
        out_error_message: *mut *mut c_char,
    ) -> i32;
    pub fn sp_audio_buffer_task_append_i16(
        token: *mut c_void,
        samples: *const i16,
        sample_count: usize,
        sample_rate: f64,
        channels: i32,
        interleaved: bool,
        out_error_message: *mut *mut c_char,
    ) -> i32;
    pub fn sp_audio_buffer_task_append_pcm_buffer_raw(
        token: *mut c_void,
        buffer: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> i32;
    pub fn sp_audio_buffer_task_append_sample_buffer_raw(
        token: *mut c_void,
        sample_buffer: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn sp_prepare_custom_language_model(
        asset_path: *const c_char,
        configuration_json: *const c_char,
        ignores_cache: bool,
        out_error_message: *mut *mut c_char,
    ) -> i32;
    pub fn sp_prepare_custom_language_model_with_client_identifier(
        asset_path: *const c_char,
        client_identifier: *const c_char,
        configuration_json: *const c_char,
        ignores_cache: bool,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn sp_dictation_supported_locales_json(
        out_json: *mut *mut c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;
    pub fn sp_dictation_installed_locales_json(
        out_json: *mut *mut c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;
    pub fn sp_dictation_supported_locale_identifier(
        locale_id: *const c_char,
        out_locale_id: *mut *mut c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;
    pub fn sp_dictation_selected_locales_json(
        configuration_json: *const c_char,
        out_json: *mut *mut c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;
    pub fn sp_dictation_available_audio_formats_json(
        configuration_json: *const c_char,
        out_json: *mut *mut c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;
    pub fn sp_dictation_transcribe_url_json(
        audio_path: *const c_char,
        configuration_json: *const c_char,
        out_json: *mut *mut c_char,
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

pub type LiveCallback =
    unsafe extern "C" fn(user_info: *mut c_void, transcript: *const c_char, is_final: bool);

pub type TaskEventCallback =
    unsafe extern "C" fn(user_info: *mut c_void, payload_json: *const c_char);
pub type AvailabilityCallback = unsafe extern "C" fn(user_info: *mut c_void, available: bool);

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
