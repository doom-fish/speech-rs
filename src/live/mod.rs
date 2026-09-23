//! Live audio-buffer streaming recognition via
//! `SFSpeechAudioBufferRecognitionRequest` (Speech v0.2).

use core::ffi::{c_char, c_void};
use core::ptr;
use std::ffi::CString;
use std::sync::Arc;

use doom_fish_utils::panic_safe::catch_user_panic;

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

/// C trampoline handed to the Swift `LiveSession`, invoked from its `deinit` to
/// drop the `Arc<CallbackBox>` reference that was transferred across FFI via
/// `Arc::into_raw` in [`LiveRecognition::start`]. Because the Swift session owns
/// this reference and only releases it in `deinit` (after the underlying
/// recognition task has been cancelled), the callback context outlives any
/// in-flight callback — closing the use-after-free window that existed when a
/// bare `Arc::as_ptr` pointer was handed across FFI with no ownership transfer.
///
/// # Safety
///
/// `user_info` must be `null` or a pointer previously produced by
/// `Arc::into_raw` for a `CallbackBox` that has not yet been released.
unsafe extern "C" fn ctx_release(user_info: *mut c_void) {
    if user_info.is_null() {
        return;
    }
    // SAFETY: balances the `Arc::into_raw` in `LiveRecognition::start`.
    drop(unsafe { Arc::from_raw(user_info.cast::<CallbackBox>()) });
}

/// # Safety
///
/// `user_info` must be a valid, non-aliased `*const CallbackBox` pointer kept
/// alive for the lifetime of the recognition session, or `null`.
/// `transcript` must be a valid NUL-terminated C string or `null`.
unsafe extern "C" fn trampoline(user_info: *mut c_void, transcript: *const c_char, is_final: bool) {
    if user_info.is_null() {
        return;
    }
    // SAFETY: Caller guarantees user_info is a valid *const CallbackBox for the
    // session lifetime; null check above ensures it is non-null.
    let cb = unsafe { &*user_info.cast::<CallbackBox>() };
    let s = if transcript.is_null() {
        String::new()
    } else {
        // SAFETY: Caller guarantees transcript is a valid NUL-terminated C string.
        unsafe { core::ffi::CStr::from_ptr(transcript) }
            .to_string_lossy()
            .into_owned()
    };
    catch_user_panic("speech::live::trampoline", || {
        (cb.callback)(LiveUpdate {
            transcript: s,
            is_final,
        });
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
        // Transfer ownership of the `Arc` to the Swift bridge. It is reclaimed
        // (and dropped) by `ctx_release`, invoked from the Swift session's
        // `deinit`, or here in the error path if the start call fails.
        let cb_raw = Arc::into_raw(cb_box).cast::<c_void>().cast_mut();

        let mut err_msg: *mut c_char = ptr::null_mut();
        let token = unsafe {
            ffi::sp_live_recognition_start(
                locale_c.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
                trampoline,
                cb_raw,
                ctx_release,
                &raw mut err_msg,
            )
        };
        if token.is_null() {
            // Start failed: Swift never took ownership, so reclaim the `Arc`.
            drop(unsafe { Arc::from_raw(cb_raw.cast::<CallbackBox>()) });
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
        Ok(Self { token })
    }

    /// End the audio stream cleanly. Apple finalises any in-flight
    /// audio and fires the callback one last time with `is_final=true`.
    /// The session is still alive after this; drop the
    /// [`LiveRecognition`] when you're done observing results.
    pub fn end_audio(&self) {
        if !self.token.is_null() {
            unsafe { ffi::sp_live_recognition_end_audio(self.token) };
        }
    }

    /// Cancel the recognition task immediately and discard any
    /// in-flight audio. The session is still alive (the callback may
    /// fire once more with a cancellation error); drop the
    /// [`LiveRecognition`] when done.
    pub fn cancel(&self) {
        if !self.token.is_null() {
            unsafe { ffi::sp_live_recognition_cancel(self.token) };
        }
    }
}
