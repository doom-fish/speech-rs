//! Async API for the `speech` crate — enabled with `features = ["async"]`.
//!
//! Wraps Apple's callback-based and `async throws` Speech framework APIs as
//! executor-agnostic [`std::future::Future`] newtypes.  Works with any async
//! runtime (Tokio, async-std, smol, `pollster`, …).
//!
//! ## API families
//!
//! | Type | API wrapped | OS req |
//! |------|-------------|--------|
//! | [`AsyncSpeechRecognizer::request_authorization`] | `SFSpeechRecognizer.requestAuthorization` | macOS 13+ |
//! | [`AsyncSpeechRecognizer::recognize_url`] | `SFSpeechRecognitionTask` one-shot | macOS 13+ |
//! | [`AsyncSpeechAnalyzer::analyze_in_path`] | `SpeechAnalyzer` native `async throws` | macOS 26+ |
//! | [`AsyncSpeechLanguageModel::prepare_custom_language_model`] | `SFSpeechLanguageModel.prepareCustomLanguageModel` | macOS 14+ |
//!
//! ## Notes
//!
//! - Multi-fire delegate APIs (`SFSpeechRecognitionTaskDelegate` event stream,
//!   live recognition updates) are **Tier-2** — they map to Streams, not
//!   Futures, and are not covered here.
//! - All futures are **cancel-safe**: dropping the future before it resolves
//!   simply discards the context pointer; the Swift callback still fires but
//!   its result is silently dropped.
//!
//! ## Example
//!
//! ```rust,no_run
//! use speech::async_api::AsyncSpeechRecognizer;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! pollster::block_on(async {
//!     let status = AsyncSpeechRecognizer::request_authorization().await?;
//!     println!("authorization: {status:?}");
//!     Ok(())
//! })
//! # }
//! ```

use std::ffi::{c_void, CStr};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};

use doom_fish_utils::completion::{error_from_cstr, AsyncCompletion, AsyncCompletionFuture};

use crate::analyzer::{parse_analyzer_output_json, SpeechAnalyzer, SpeechAnalyzerOutput};
use crate::error::{AuthorizationStatus, SpeechError};
use crate::ffi;
use crate::language_model::LanguageModelConfiguration;
use crate::private::{cstring_from_path, json_cstring};
use crate::recognizer::SpeechRecognizer;
use crate::request::UrlRecognitionRequest;
use crate::transcription::DetailedRecognitionResult;
use crate::analyzer::SpeechModuleDescriptor;

// ============================================================================
// Internal helpers
// ============================================================================

/// Parse a JSON string from the Swift bridge into a Rust value.
fn parse_json_string<T: serde::de::DeserializeOwned>(
    json: &str,
    context: &str,
) -> Result<T, SpeechError> {
    serde_json::from_str::<T>(json).map_err(|e| {
        SpeechError::InvalidArgument(format!(
            "failed to parse {context} JSON: {e}; payload={json}"
        ))
    })
}

// ============================================================================
// 1. AuthorizationFuture — SFSpeechRecognizer.requestAuthorization
// ============================================================================

/// C callback for `sp_request_authorization_async`.
///
/// # Safety
/// `ctx` must be a valid `AsyncCompletion<AuthorizationStatus>` context pointer.
unsafe extern "C" fn authorization_cb(status: i32, ctx: *mut c_void) {
    let auth = AuthorizationStatus::from_raw(status);
    // Safety: ctx is the context pointer created by AsyncCompletion::create().
    unsafe { AsyncCompletion::complete_ok(ctx, auth) };
}

/// Future returned by [`AsyncSpeechRecognizer::request_authorization`].
///
/// Resolves to the [`AuthorizationStatus`] reported by the system.  The
/// future never resolves to an error — authorization itself always succeeds;
/// the status may be `Denied` or `Restricted`.
#[must_use = "futures do nothing unless polled"]
pub struct AuthorizationFuture {
    inner: AsyncCompletionFuture<AuthorizationStatus>,
}

impl std::fmt::Debug for AuthorizationFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthorizationFuture").finish_non_exhaustive()
    }
}

impl Future for AuthorizationFuture {
    type Output = Result<AuthorizationStatus, SpeechError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner)
            .poll(cx)
            .map(|r| r.map_err(SpeechError::RecognitionFailed))
    }
}

// ============================================================================
// 2. RecognizeUrlFuture — SFSpeechRecognitionTask one-shot
// ============================================================================

/// C callback for `sp_recognize_url_async`.
///
/// # Safety
/// `ctx` must be a valid `AsyncCompletion<String>` context pointer.
unsafe extern "C" fn recognize_url_cb(
    json: *const i8,
    error: *const i8,
    ctx: *mut c_void,
) {
    if !error.is_null() {
        let msg = unsafe { error_from_cstr(error) };
        unsafe { AsyncCompletion::<String>::complete_err(ctx, msg) };
    } else if !json.is_null() {
        let s = unsafe { CStr::from_ptr(json).to_string_lossy().into_owned() };
        unsafe { AsyncCompletion::complete_ok(ctx, s) };
    } else {
        unsafe {
            AsyncCompletion::<String>::complete_err(ctx, "recognition returned no result".into());
        };
    }
}

/// Future returned by [`AsyncSpeechRecognizer::recognize_url`].
///
/// Resolves to a [`DetailedRecognitionResult`] when the final recognition
/// result has been produced, or a [`SpeechError`] on failure.
#[must_use = "futures do nothing unless polled"]
pub struct RecognizeUrlFuture {
    inner: AsyncCompletionFuture<String>,
}

impl std::fmt::Debug for RecognizeUrlFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecognizeUrlFuture").finish_non_exhaustive()
    }
}

impl Future for RecognizeUrlFuture {
    type Output = Result<DetailedRecognitionResult, SpeechError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx).map(|r| {
            r.map_err(SpeechError::RecognitionFailed).and_then(|json| {
                parse_json_string::<DetailedRecognitionResult>(&json, "detailed recognition result")
            })
        })
    }
}

// ============================================================================
// 3. AnalyzeUrlFuture — SpeechAnalyzer (macOS 26.0+)
// ============================================================================

/// C callback for `sp_speech_analyzer_analyze_url_async`.
///
/// # Safety
/// `ctx` must be a valid `AsyncCompletion<String>` context pointer.
unsafe extern "C" fn analyze_url_cb(
    json: *const i8,
    error: *const i8,
    ctx: *mut c_void,
) {
    if !error.is_null() {
        let msg = unsafe { error_from_cstr(error) };
        unsafe { AsyncCompletion::<String>::complete_err(ctx, msg) };
    } else if !json.is_null() {
        let s = unsafe { CStr::from_ptr(json).to_string_lossy().into_owned() };
        unsafe { AsyncCompletion::complete_ok(ctx, s) };
    } else {
        unsafe {
            AsyncCompletion::<String>::complete_err(ctx, "analyzer returned no result".into());
        };
    }
}

/// Future returned by [`AsyncSpeechAnalyzer::analyze_in_path`].
///
/// Resolves to a [`SpeechAnalyzerOutput`] on success, or a [`SpeechError`]
/// on failure.  Requires macOS 26.0+; on older OS versions the future
/// resolves immediately with [`SpeechError::RecognizerUnavailable`].
#[must_use = "futures do nothing unless polled"]
pub struct AnalyzeUrlFuture {
    inner: AsyncCompletionFuture<String>,
    /// The analyzer's module list, kept to reconstruct `SpeechAnalyzerOutput`
    /// from the JSON payload returned by the Swift bridge.
    modules: Vec<SpeechModuleDescriptor>,
}

impl std::fmt::Debug for AnalyzeUrlFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyzeUrlFuture").finish_non_exhaustive()
    }
}

impl Future for AnalyzeUrlFuture {
    type Output = Result<SpeechAnalyzerOutput, SpeechError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let modules = self.modules.clone();
        Pin::new(&mut self.inner).poll(cx).map(|r| {
            r.map_err(SpeechError::RecognitionFailed)
                .and_then(|json| parse_analyzer_output_json(&json, &modules))
        })
    }
}

// ============================================================================
// 4. PrepareLanguageModelFuture — SFSpeechLanguageModel.prepareCustomLanguageModel
// ============================================================================

/// C callback for `sp_prepare_custom_language_model_async`.
///
/// # Safety
/// `ctx` must be a valid `AsyncCompletion<()>` context pointer.
unsafe extern "C" fn prepare_language_model_cb(error: *const i8, ctx: *mut c_void) {
    if error.is_null() {
        unsafe { AsyncCompletion::complete_ok(ctx, ()) };
    } else {
        let msg = unsafe { error_from_cstr(error) };
        unsafe { AsyncCompletion::<()>::complete_err(ctx, msg) };
    }
}

/// Future returned by [`AsyncSpeechLanguageModel::prepare_custom_language_model`].
///
/// Resolves to `()` on success, or a [`SpeechError`] on failure.
/// Requires macOS 14.0+.
#[must_use = "futures do nothing unless polled"]
pub struct PrepareLanguageModelFuture {
    inner: AsyncCompletionFuture<()>,
}

impl std::fmt::Debug for PrepareLanguageModelFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrepareLanguageModelFuture")
            .finish_non_exhaustive()
    }
}

impl Future for PrepareLanguageModelFuture {
    type Output = Result<(), SpeechError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner)
            .poll(cx)
            .map(|r| r.map_err(SpeechError::RecognitionFailed))
    }
}

// ============================================================================
// Public entry-point structs
// ============================================================================

/// Async entry points for `SFSpeechRecognizer` operations.
pub struct AsyncSpeechRecognizer;

impl AsyncSpeechRecognizer {
    /// Request speech recognition authorization asynchronously.
    ///
    /// Returns the [`AuthorizationStatus`] granted by the system.
    /// This never fails — the status may be `Denied` or `Restricted`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use speech::async_api::AsyncSpeechRecognizer;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let status = pollster::block_on(AsyncSpeechRecognizer::request_authorization())?;
    /// println!("authorization: {status:?}");
    /// # Ok(()) }
    /// ```
    #[must_use = "this future does nothing unless polled"]
    pub fn request_authorization() -> AuthorizationFuture {
        let (future, ctx) = AsyncCompletion::create();
        // Safety: ctx is a valid AsyncCompletion context pointer, lifetime managed
        //         by the Arc inside future; authorization_cb will fire exactly once.
        unsafe { ffi::sp_request_authorization_async(authorization_cb, ctx) };
        AuthorizationFuture { inner: future }
    }

    /// Recognize speech in the audio file at `path` asynchronously.
    ///
    /// Uses `SFSpeechRecognitionTask` with a result-handler that fires once
    /// with the final result.  Returns `Err` if a [`CString`] can't be
    /// constructed from the path, locale, or options (e.g., NUL byte).
    ///
    /// # Errors
    ///
    /// Returns [`SpeechError::InvalidArgument`] if a path or JSON payload
    /// contains a NUL byte.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use speech::{async_api::AsyncSpeechRecognizer, recognizer::SpeechRecognizer,
    ///              request::UrlRecognitionRequest};
    /// use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let recognizer = SpeechRecognizer::new();
    /// let request = UrlRecognitionRequest::new(Path::new("audio.m4a"));
    /// let result = pollster::block_on(AsyncSpeechRecognizer::recognize_url(&recognizer, &request)?)?;
    /// println!("{}", result.best_transcription.formatted_string);
    /// # Ok(()) }
    /// ```
    pub fn recognize_url(
        recognizer: &SpeechRecognizer,
        request: &UrlRecognitionRequest,
    ) -> Result<RecognizeUrlFuture, SpeechError> {
        let audio_path = cstring_from_path(request.path(), "audio path")?;
        let recognizer_json = recognizer.recognizer_json_cstring()?;
        let request_json = request.options().to_json_cstring()?;
        let locale_id = recognizer.locale_cstring();

        let (future, ctx) = AsyncCompletion::create();
        // Safety: all C-string pointers are valid for the duration of the call;
        //         ctx is a valid AsyncCompletion context pointer.
        unsafe {
            ffi::sp_recognize_url_async(
                audio_path.as_ptr(),
                locale_id.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
                recognizer_json.as_ptr(),
                request_json.as_ptr(),
                recognize_url_cb,
                ctx,
            );
        }
        Ok(RecognizeUrlFuture { inner: future })
    }
}

/// Async entry points for `SpeechAnalyzer` / `SpeechTranscriber` (macOS 26.0+).
pub struct AsyncSpeechAnalyzer;

impl AsyncSpeechAnalyzer {
    /// Analyze an audio file asynchronously using the given [`SpeechAnalyzer`].
    ///
    /// Requires macOS 26.0+. On older OS versions the future resolves
    /// immediately with [`SpeechError::RecognizerUnavailable`].
    ///
    /// # Errors
    ///
    /// Returns [`SpeechError::InvalidArgument`] if a path or JSON payload
    /// contains a NUL byte, or if the analyzer has no modules configured.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use speech::{
    ///     async_api::AsyncSpeechAnalyzer,
    ///     analyzer::{SpeechAnalyzer, SpeechTranscriber, SpeechTranscriberPreset},
    /// };
    /// use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let transcriber = SpeechTranscriber::new("en-US", SpeechTranscriberPreset::Transcription);
    /// let analyzer = SpeechAnalyzer::new([transcriber]);
    /// let output = pollster::block_on(
    ///     AsyncSpeechAnalyzer::analyze_in_path(&analyzer, Path::new("audio.m4a"))?,
    /// )?;
    /// println!("{} modules", output.modules.len());
    /// # Ok(()) }
    /// ```
    pub fn analyze_in_path(
        analyzer: &SpeechAnalyzer,
        path: impl AsRef<Path>,
    ) -> Result<AnalyzeUrlFuture, SpeechError> {
        if analyzer.modules().is_empty() {
            return Err(SpeechError::InvalidArgument(
                "speech analyzer requires at least one module".into(),
            ));
        }
        let audio_path = cstring_from_path(path.as_ref(), "speech analyzer audio path")?;
        let analyzer_json = json_cstring(
            &crate::analyzer::SpeechAnalyzerPayload::from(analyzer),
            "speech analyzer configuration",
        )?;
        let modules = analyzer.modules().to_vec();

        let (future, ctx) = AsyncCompletion::create();
        // Safety: C-string pointers are valid for the duration of the call;
        //         ctx is a valid AsyncCompletion context pointer.
        unsafe {
            ffi::sp_speech_analyzer_analyze_url_async(
                audio_path.as_ptr(),
                analyzer_json.as_ptr(),
                analyze_url_cb,
                ctx,
            );
        }
        Ok(AnalyzeUrlFuture { inner: future, modules })
    }
}

/// Async entry points for `SFSpeechLanguageModel` operations.
pub struct AsyncSpeechLanguageModel;

impl AsyncSpeechLanguageModel {
    /// Prepare a custom language model asynchronously.
    ///
    /// Equivalent to [`crate::language_model::SpeechLanguageModel::prepare_custom_language_model`]
    /// but non-blocking.  Requires macOS 14.0+.
    ///
    /// # Errors
    ///
    /// Returns [`SpeechError::InvalidArgument`] if the asset path or
    /// configuration JSON contains a NUL byte.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use speech::{async_api::AsyncSpeechLanguageModel, language_model::LanguageModelConfiguration};
    /// use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = LanguageModelConfiguration::new("en-US");
    /// pollster::block_on(
    ///     AsyncSpeechLanguageModel::prepare_custom_language_model(
    ///         Path::new("model.bin"), &config
    ///     )?,
    /// )?;
    /// println!("model ready");
    /// # Ok(()) }
    /// ```
    pub fn prepare_custom_language_model(
        asset: impl AsRef<Path>,
        configuration: &LanguageModelConfiguration,
    ) -> Result<PrepareLanguageModelFuture, SpeechError> {
        Self::prepare_custom_language_model_inner(asset.as_ref(), configuration, false)
    }

    /// Like [`Self::prepare_custom_language_model`] but bypasses the cache.
    ///
    /// # Errors
    ///
    /// Returns [`SpeechError::InvalidArgument`] if the asset path or
    /// configuration JSON contains a NUL byte.
    pub fn prepare_custom_language_model_ignoring_cache(
        asset: impl AsRef<Path>,
        configuration: &LanguageModelConfiguration,
    ) -> Result<PrepareLanguageModelFuture, SpeechError> {
        Self::prepare_custom_language_model_inner(asset.as_ref(), configuration, true)
    }

    fn prepare_custom_language_model_inner(
        asset: &Path,
        configuration: &LanguageModelConfiguration,
        ignores_cache: bool,
    ) -> Result<PrepareLanguageModelFuture, SpeechError> {
        let asset_c = cstring_from_path(asset, "asset path")?;
        let config_c = configuration.to_json_cstring()?;

        let (future, ctx) = AsyncCompletion::create();
        // Safety: C-string pointers are valid for the duration of the call;
        //         ctx is a valid AsyncCompletion context pointer.
        unsafe {
            ffi::sp_prepare_custom_language_model_async(
                asset_c.as_ptr(),
                config_c.as_ptr(),
                ignores_cache,
                prepare_language_model_cb,
                ctx,
            );
        }
        Ok(PrepareLanguageModelFuture { inner: future })
    }
}
