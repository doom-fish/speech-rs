//! Live audio-buffer streaming recognition via
//! `SFSpeechAudioBufferRecognitionRequest` (Speech v0.2).

use core::ffi::{c_char, c_void};
use core::ptr;

use doom_fish_utils::callback_context::CallbackContext;

use crate::error::SpeechError;
use crate::ffi;
use crate::private::error_from_status;
use crate::recognizer::SpeechRecognizer;

/// One update from the live recogniser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveUpdate {
    /// Accumulated transcript so far.
    pub transcript: String,
    /// `true` once Apple has decided this utterance is complete.
    pub is_final: bool,
}

type Callback = Box<dyn Fn(LiveUpdate) + Send + Sync + 'static>;

/// RAII guard for a running live recognition session. Drop to stop.
pub struct LiveRecognition {
    token: *mut c_void,
    context: CallbackContext<Callback>,
}

unsafe impl Send for LiveRecognition {}
unsafe impl Sync for LiveRecognition {}

impl Drop for LiveRecognition {
    fn drop(&mut self) {
        self.context.deactivate();
        if !self.token.is_null() {
            unsafe { ffi::sp_live_recognition_stop(self.token) };
            self.token = ptr::null_mut();
        }
    }
}

/// # Safety
///
/// `user_info` must be `null` or a `CallbackContext<Callback>` pointer that
/// stays retained for the duration of the call.
/// `transcript` must be a valid NUL-terminated C string or `null`.
unsafe extern "C" fn trampoline(user_info: *mut c_void, transcript: *const c_char, is_final: bool) {
    let transcript = if transcript.is_null() {
        String::new()
    } else {
        // SAFETY: Caller guarantees transcript is a valid NUL-terminated C string.
        unsafe { core::ffi::CStr::from_ptr(transcript) }
            .to_string_lossy()
            .into_owned()
    };
    unsafe {
        CallbackContext::<Callback>::with(user_info, "speech::live::trampoline", |callback| {
            callback(LiveUpdate {
                transcript,
                is_final,
            });
        })
    };
}

impl LiveRecognition {
    /// Start live recognition with `recognizer`'s locale, default task hint
    /// and callback queue. The `callback` fires on that queue with each
    /// partial / final update.
    ///
    /// Requires microphone permission + `SFSpeechRecognizer` authorization.
    ///
    /// # Errors
    ///
    /// Returns [`SpeechError::NotAuthorized`] if speech recognition is not
    /// authorized, [`SpeechError::RecognizerUnavailable`] if Apple's
    /// recogniser is unavailable, [`SpeechError::AudioLoadFailed`] if no
    /// audio input can be started, or [`SpeechError::InvalidArgument`] for
    /// an invalid recognizer configuration.
    pub fn start<F>(recognizer: &SpeechRecognizer, callback: F) -> Result<Self, SpeechError>
    where
        F: Fn(LiveUpdate) + Send + Sync + 'static,
    {
        let recognizer_json = recognizer.recognizer_json()?;
        let callback: Callback = Box::new(callback);
        let context = CallbackContext::new(callback);
        let mut status = ffi::status::RECOGNIZER_UNAVAILABLE;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let token = unsafe {
            ffi::sp_live_recognition_start(
                recognizer.locale_ptr(),
                recognizer_json.as_ptr(),
                trampoline,
                context.as_ptr(),
                CallbackContext::<Callback>::RETAIN,
                CallbackContext::<Callback>::RELEASE,
                &raw mut status,
                &raw mut err_msg,
            )
        };
        if token.is_null() {
            return Err(unsafe { error_from_status(status, err_msg) });
        }
        Ok(Self { token, context })
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

#[cfg(test)]
mod tests {
    use core::ffi::c_void;
    use std::sync::{Arc, Mutex};

    use doom_fish_utils::callback_context::CallbackContext;

    use super::{trampoline, Callback, LiveUpdate};
    use crate::ffi;

    type Updates = Arc<Mutex<Vec<LiveUpdate>>>;

    fn recording_context() -> (CallbackContext<Callback>, Updates) {
        let updates = Updates::default();
        let sink = Arc::clone(&updates);
        let callback: Callback = Box::new(move |update| {
            sink.lock().unwrap().push(update);
        });
        (CallbackContext::new(callback), updates)
    }

    fn exercise_relay(context: *mut c_void, finals: &[bool]) {
        unsafe {
            ffi::sp_live_result_relay_exercise(
                trampoline,
                context,
                CallbackContext::<Callback>::RETAIN,
                CallbackContext::<Callback>::RELEASE,
                finals.as_ptr(),
                finals.len(),
            );
        }
    }

    fn update(index: usize, is_final: bool) -> LiveUpdate {
        LiveUpdate {
            transcript: format!("update {index}"),
            is_final,
        }
    }

    #[test]
    fn the_relay_stops_delivering_after_the_final_update() {
        let (context, updates) = recording_context();
        exercise_relay(context.as_ptr(), &[false, false, true, false]);
        assert_eq!(
            *updates.lock().unwrap(),
            vec![update(0, false), update(1, false), update(2, true)]
        );
        drop(context);
        assert_eq!(Arc::strong_count(&updates), 1);
    }

    #[test]
    fn the_relay_releases_its_reference_without_a_final_update() {
        let (context, updates) = recording_context();
        exercise_relay(context.as_ptr(), &[false, false]);
        assert_eq!(updates.lock().unwrap().len(), 2);
        drop(context);
        assert_eq!(Arc::strong_count(&updates), 1);
    }

    #[test]
    fn updates_after_the_handle_is_dropped_never_reach_the_callback() {
        let (context, updates) = recording_context();
        let session_reference = context.retained_ptr();
        drop(context);

        exercise_relay(session_reference, &[false, true]);
        assert!(updates.lock().unwrap().is_empty());
        assert_eq!(Arc::strong_count(&updates), 2);

        unsafe { (CallbackContext::<Callback>::RELEASE)(session_reference) };
        assert_eq!(Arc::strong_count(&updates), 1);
    }
}
