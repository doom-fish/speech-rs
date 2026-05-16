//! Live audio-buffer streaming recognition via
//! `SFSpeechAudioBufferRecognitionRequest` (Speech v0.2).

use core::ffi::{c_char, c_void};
use core::ptr;
use std::ffi::CString;
use std::sync::Arc;

use crate::error::SpeechError;
use crate::ffi;

/// One update from the live recogniser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveUpdate {
    /// Accumulated transcript so far.
    pub transcript: String,
    /// `true` once Apple has decided this utterance is complete.
    pub is_final: bool,
}

type Callback = Box<dyn Fn(LiveUpdate) + Send + Sync + 'static>;

struct CallbackBox {
    callback: Callback,
}

/// RAII guard for a running live recognition session. Drop to stop.
pub struct LiveRecognition {
    token: *mut c_void,
    _callback: Arc<CallbackBox>,
}

unsafe impl Send for LiveRecognition {}
unsafe impl Sync for LiveRecognition {}

impl Drop for LiveRecognition {
    fn drop(&mut self) {
        if !self.token.is_null() {
            unsafe { ffi::sp_live_recognition_stop(self.token) };
            self.token = ptr::null_mut();
        }
    }
}

unsafe extern "C" fn trampoline(
    user_info: *mut c_void,
    transcript: *const c_char,
    is_final: bool,
) {
    if user_info.is_null() {
        return;
    }
    let cb = unsafe { &*user_info.cast::<CallbackBox>() };
    let s = if transcript.is_null() {
        String::new()
    } else {
        unsafe { core::ffi::CStr::from_ptr(transcript) }
            .to_string_lossy()
            .into_owned()
    };
    (cb.callback)(LiveUpdate {
        transcript: s,
        is_final,
    });
}

impl LiveRecognition {
    /// Start live recognition. The `callback` fires on Apple's recognition
    /// queue with each partial / final update.
    ///
    /// `locale` is a BCP-47 identifier (e.g. `"en-US"`, `"sv-SE"`). Pass
    /// `None` for the system default.
    ///
    /// Requires microphone permission + `SFSpeechRecognizer` authorization.
    ///
    /// # Errors
    ///
    /// Returns [`SpeechError::RecognizerUnavailable`] if Apple's recogniser
    /// is unavailable, or [`SpeechError::InvalidArgument`] for invalid locale.
    pub fn start<F>(locale: Option<&str>, callback: F) -> Result<Self, SpeechError>
    where
        F: Fn(LiveUpdate) + Send + Sync + 'static,
    {
        let locale_c = match locale {
            Some(l) => Some(
                CString::new(l)
                    .map_err(|e| SpeechError::InvalidArgument(format!("locale NUL: {e}")))?,
            ),
            None => None,
        };

        let cb_box = Arc::new(CallbackBox {
            callback: Box::new(callback),
        });
        let cb_raw = Arc::as_ptr(&cb_box).cast::<c_void>().cast_mut();

        let mut err_msg: *mut c_char = ptr::null_mut();
        let token = unsafe {
            ffi::sp_live_recognition_start(
                locale_c.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
                trampoline,
                cb_raw,
                &mut err_msg,
            )
        };
        if token.is_null() {
            let msg = if err_msg.is_null() {
                "live recognition start failed".to_string()
            } else {
                let s = unsafe { core::ffi::CStr::from_ptr(err_msg) }
                    .to_string_lossy()
                    .into_owned();
                unsafe { ffi::sp_string_free(err_msg) };
                s
            };
            return Err(SpeechError::RecognizerUnavailable(msg));
        }
        Ok(Self {
            token,
            _callback: cb_box,
        })
    }
}
