#![allow(
    clippy::doc_markdown,
    clippy::manual_let_else,
    clippy::missing_const_for_fn,
    clippy::missing_errors_doc
)]

use core::ffi::{c_char, c_void};
use core::ptr;
use std::sync::Arc;

use serde::Deserialize;

use crate::error::{SpeechError, SpeechFrameworkErrorCode};
use crate::ffi;
use crate::private::{error_from_status, parse_json_ptr};
use crate::request::{AudioFormat, AudioFormatPayload};
use crate::transcription::{DetailedRecognitionResult, Transcription};

/// The current lifecycle state of a speech recognition task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskState {
    Starting,
    Running,
    Finishing,
    Canceling,
    Completed,
    Unknown(i32),
}

impl TaskState {
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Starting,
            1 => Self::Running,
            2 => Self::Finishing,
            3 => Self::Canceling,
            4 => Self::Completed,
            other => Self::Unknown(other),
        }
    }
}

/// A framework error returned by `SFSpeechRecognitionTask.error`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskErrorInfo {
    pub domain: String,
    pub code: i64,
    pub localized_description: String,
    pub kind: SpeechFrameworkErrorCode,
}

/// One delegate callback emitted by `SFSpeechRecognitionTaskDelegate`.
#[derive(Debug, Clone, PartialEq)]
pub enum RecognitionTaskEvent {
    DidDetectSpeech,
    DidHypothesizeTranscription(Transcription),
    DidFinishRecognition(DetailedRecognitionResult),
    FinishedReadingAudio,
    WasCancelled,
    DidFinishSuccessfully(bool),
    DidProcessAudioDuration(f64),
}

type TaskCallback = Box<dyn Fn(RecognitionTaskEvent) + Send + Sync + 'static>;

pub(crate) struct TaskCallbackBox {
    callback: TaskCallback,
}

type AvailabilityCallback = Box<dyn Fn(bool) + Send + Sync + 'static>;

pub(crate) struct AvailabilityCallbackBox {
    callback: AvailabilityCallback,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskEventPayload {
    event: String,
    transcription: Option<Transcription>,
    result: Option<DetailedRecognitionResult>,
    duration: Option<f64>,
    successfully: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskErrorPayload {
    domain: String,
    code: i64,
    localized_description: String,
}

struct TaskCore {
    token: *mut c_void,
    _callback: Arc<TaskCallbackBox>,
}

unsafe impl Send for TaskCore {}
unsafe impl Sync for TaskCore {}

impl Drop for TaskCore {
    fn drop(&mut self) {
        if !self.token.is_null() {
            unsafe { ffi::sp_task_release(self.token) };
            self.token = ptr::null_mut();
        }
    }
}

impl TaskCore {
    fn new(token: *mut c_void, callback: Arc<TaskCallbackBox>) -> Self {
        Self {
            token,
            _callback: callback,
        }
    }

    fn finish(&self) {
        if !self.token.is_null() {
            unsafe { ffi::sp_task_finish(self.token) };
        }
    }

    fn cancel(&self) {
        if !self.token.is_null() {
            unsafe { ffi::sp_task_cancel(self.token) };
        }
    }

    fn state(&self) -> TaskState {
        TaskState::from_raw(unsafe { ffi::sp_task_state(self.token) })
    }

    fn is_finishing(&self) -> bool {
        unsafe { ffi::sp_task_is_finishing(self.token) }
    }

    fn is_cancelled(&self) -> bool {
        unsafe { ffi::sp_task_is_cancelled(self.token) }
    }

    fn error(&self) -> Option<TaskErrorInfo> {
        let ptr = unsafe { ffi::sp_task_error_json(self.token) };
        if ptr.is_null() {
            return None;
        }
        let payload = unsafe { parse_json_ptr::<TaskErrorPayload>(ptr, "task error") }.ok()?;
        Some(TaskErrorInfo {
            kind: SpeechFrameworkErrorCode::from_domain_and_code(&payload.domain, payload.code),
            domain: payload.domain,
            code: payload.code,
            localized_description: payload.localized_description,
        })
    }
}

/// A file-backed asynchronous recognition task.
pub struct RecognitionTask {
    core: TaskCore,
}

unsafe impl Send for RecognitionTask {}
unsafe impl Sync for RecognitionTask {}

impl RecognitionTask {
    pub(crate) fn from_token(token: *mut c_void, callback: Arc<TaskCallbackBox>) -> Self {
        Self {
            core: TaskCore::new(token, callback),
        }
    }

    pub fn finish(&self) {
        self.core.finish();
    }

    pub fn cancel(&self) {
        self.core.cancel();
    }

    #[must_use]
    pub fn state(&self) -> TaskState {
        self.core.state()
    }

    #[must_use]
    pub fn is_finishing(&self) -> bool {
        self.core.is_finishing()
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.core.is_cancelled()
    }

    #[must_use]
    pub fn error(&self) -> Option<TaskErrorInfo> {
        self.core.error()
    }
}

/// An audio-buffer task that supports manual PCM/sample-buffer appends.
pub struct AudioBufferRecognitionTask {
    core: TaskCore,
}

unsafe impl Send for AudioBufferRecognitionTask {}
unsafe impl Sync for AudioBufferRecognitionTask {}

impl AudioBufferRecognitionTask {
    pub(crate) fn from_token(token: *mut c_void, callback: Arc<TaskCallbackBox>) -> Self {
        Self {
            core: TaskCore::new(token, callback),
        }
    }

    pub fn finish(&self) {
        self.core.finish();
    }

    pub fn cancel(&self) {
        self.core.cancel();
    }

    /// Equivalent to `SFSpeechAudioBufferRecognitionRequest.endAudio()`.
    pub fn end_audio(&self) {
        if !self.core.token.is_null() {
            unsafe { ffi::sp_audio_buffer_task_end_audio(self.core.token) };
        }
    }

    #[must_use]
    pub fn state(&self) -> TaskState {
        self.core.state()
    }

    #[must_use]
    pub fn is_finishing(&self) -> bool {
        self.core.is_finishing()
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.core.is_cancelled()
    }

    #[must_use]
    pub fn error(&self) -> Option<TaskErrorInfo> {
        self.core.error()
    }

    pub fn native_audio_format(&self) -> Result<AudioFormat, SpeechError> {
        let ptr = unsafe { ffi::sp_audio_buffer_task_native_format_json(self.core.token) };
        unsafe { parse_json_ptr::<AudioFormatPayload>(ptr, "audio-buffer task native format") }
            .map(AudioFormat::from)
    }

    pub fn append_interleaved_f32(
        &self,
        sample_rate: f64,
        channels: usize,
        samples: &[f32],
    ) -> Result<(), SpeechError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::sp_audio_buffer_task_append_f32(
                self.core.token,
                samples.as_ptr(),
                samples.len(),
                sample_rate,
                i32::try_from(channels).map_err(|_| {
                    SpeechError::InvalidArgument("channel count does not fit into i32".into())
                })?,
                true,
                &mut err_msg,
            )
        };
        if status == ffi::status::OK {
            Ok(())
        } else {
            Err(unsafe { error_from_status(status, err_msg) })
        }
    }

    pub fn append_interleaved_i16(
        &self,
        sample_rate: f64,
        channels: usize,
        samples: &[i16],
    ) -> Result<(), SpeechError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::sp_audio_buffer_task_append_i16(
                self.core.token,
                samples.as_ptr(),
                samples.len(),
                sample_rate,
                i32::try_from(channels).map_err(|_| {
                    SpeechError::InvalidArgument("channel count does not fit into i32".into())
                })?,
                true,
                &mut err_msg,
            )
        };
        if status == ffi::status::OK {
            Ok(())
        } else {
            Err(unsafe { error_from_status(status, err_msg) })
        }
    }

    /// # Safety
    ///
    /// `buffer` must be a valid `AVAudioPCMBuffer *` allocated by Apple's AVFoundation APIs.
    pub unsafe fn append_audio_pcm_buffer_raw(
        &self,
        buffer: *mut c_void,
    ) -> Result<(), SpeechError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status =
            ffi::sp_audio_buffer_task_append_pcm_buffer_raw(self.core.token, buffer, &mut err_msg);
        if status == ffi::status::OK {
            Ok(())
        } else {
            Err(error_from_status(status, err_msg))
        }
    }

    /// # Safety
    ///
    /// `sample_buffer` must be a valid `CMSampleBufferRef` allocated by CoreMedia.
    pub unsafe fn append_audio_sample_buffer_raw(
        &self,
        sample_buffer: *mut c_void,
    ) -> Result<(), SpeechError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = ffi::sp_audio_buffer_task_append_sample_buffer_raw(
            self.core.token,
            sample_buffer,
            &mut err_msg,
        );
        if status == ffi::status::OK {
            Ok(())
        } else {
            Err(error_from_status(status, err_msg))
        }
    }
}

/// RAII observer for `SFSpeechRecognizerDelegate.availabilityDidChange`.
pub struct RecognizerAvailabilityObserver {
    token: *mut c_void,
    _callback: Arc<AvailabilityCallbackBox>,
}

unsafe impl Send for RecognizerAvailabilityObserver {}
unsafe impl Sync for RecognizerAvailabilityObserver {}

impl Drop for RecognizerAvailabilityObserver {
    fn drop(&mut self) {
        if !self.token.is_null() {
            unsafe { ffi::sp_recognizer_availability_observer_stop(self.token) };
            self.token = ptr::null_mut();
        }
    }
}

impl RecognizerAvailabilityObserver {
    pub(crate) fn from_token(token: *mut c_void, callback: Arc<AvailabilityCallbackBox>) -> Self {
        Self {
            token,
            _callback: callback,
        }
    }
}

pub(crate) unsafe extern "C" fn task_event_trampoline(
    user_info: *mut c_void,
    payload_json: *const c_char,
) {
    if user_info.is_null() || payload_json.is_null() {
        return;
    }
    let callback = &*user_info.cast::<TaskCallbackBox>();
    let payload = match core::ffi::CStr::from_ptr(payload_json).to_str() {
        Ok(payload) => payload,
        Err(_) => return,
    };
    let raw = match serde_json::from_str::<TaskEventPayload>(payload) {
        Ok(raw) => raw,
        Err(_) => return,
    };

    let event = match raw.event.as_str() {
        "didDetectSpeech" => RecognitionTaskEvent::DidDetectSpeech,
        "didHypothesizeTranscription" => match raw.transcription {
            Some(transcription) => RecognitionTaskEvent::DidHypothesizeTranscription(transcription),
            None => return,
        },
        "didFinishRecognition" => match raw.result {
            Some(result) => RecognitionTaskEvent::DidFinishRecognition(result),
            None => return,
        },
        "finishedReadingAudio" => RecognitionTaskEvent::FinishedReadingAudio,
        "wasCancelled" => RecognitionTaskEvent::WasCancelled,
        "didFinishSuccessfully" => match raw.successfully {
            Some(successfully) => RecognitionTaskEvent::DidFinishSuccessfully(successfully),
            None => return,
        },
        "didProcessAudioDuration" => match raw.duration {
            Some(duration) => RecognitionTaskEvent::DidProcessAudioDuration(duration),
            None => return,
        },
        _ => return,
    };

    (callback.callback)(event);
}

pub(crate) unsafe extern "C" fn availability_trampoline(user_info: *mut c_void, available: bool) {
    if user_info.is_null() {
        return;
    }
    let callback = &*user_info.cast::<AvailabilityCallbackBox>();
    (callback.callback)(available);
}

pub(crate) fn make_task_callback<F>(callback: F) -> Arc<TaskCallbackBox>
where
    F: Fn(RecognitionTaskEvent) + Send + Sync + 'static,
{
    Arc::new(TaskCallbackBox {
        callback: Box::new(callback),
    })
}

pub(crate) fn make_availability_callback<F>(callback: F) -> Arc<AvailabilityCallbackBox>
where
    F: Fn(bool) + Send + Sync + 'static,
{
    Arc::new(AvailabilityCallbackBox {
        callback: Box::new(callback),
    })
}
