# Changelog

All notable changes to `speech` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.9.0] - Unreleased

### Security

- `AsyncSpeechRecognizer::recognize_url`, `AsyncSpeechAnalyzer::analyze_in_path`
  and `AsyncSpeechLanguageModel::prepare_custom_language_model` no longer read
  freed memory. The Swift thunks read the path, locale and JSON strings inside
  `Task.detached`, after Rust had freed them: a heap use-after-free on every
  call. They now copy every input before the call returns.
- `LiveRecognition` no longer frees its callback context while a callback can
  still arrive. Stopping released the session, and with it the context, before
  the cancellation callback was delivered. The context is now a doom-fish-utils
  `CallbackContext`; the Swift session holds a reference until its handler has
  delivered its last update, and dropping `LiveRecognition` deactivates the
  context first.
- **Breaking:** recognition is on-device by default.
  `RecognitionRequestOptions::default()` left `requiresOnDeviceRecognition`
  unset, so audio could be sent to Apple's servers. Every `SFSpeechRecognizer`
  path now requires on-device recognition unless you call
  `with_requires_on_device_recognition(false)`.

### Fixed

- `RecognizeUrlFuture` resolves with the final result. The first partial result
  used up its one-shot guard (partial results are on by default), so the future
  never completed.
- The global live-session table is guarded by a lock, and `end_audio`, `cancel`
  and stop are serialized per session.
- `AssetInstallationRequest::download_and_install` no longer cancels the
  download after 120 s.
- Custom-model recognition (`sp_recognize_url_with_custom_model`) returns
  `SP_TIMED_OUT` on timeout instead of success with an empty transcript.
- `start_audio_buffer_task` checks speech authorization, like the other task
  starters.
- `LiveRecognition::start` checks speech authorization before it opens the
  microphone, and returns an error instead of raising an Objective-C exception
  when there is no audio input.
- `start_microphone_task` taps the input with the hardware format instead of
  the request's 16 kHz native format, which AVAudioEngine rejects with an
  Objective-C exception, and cancels its task if the audio engine fails to
  start.
- Errors from the async API, the task starters and `LiveRecognition::start` keep
  their kind (`NotAuthorized`, `AudioLoadFailed`, `RecognizerUnavailable`,
  `TimedOut`, `Framework`) instead of always being `RecognitionFailed` or
  `RecognizerUnavailable`. The async URL path now also checks that the file
  exists and that recognition is authorized.
- Synchronous detailed recognition keeps only the first final result or error,
  so a later callback can no longer race the reader.
- COVERAGE lists `AnalyzerInput` as a gap (it can be built, but nothing accepts
  it) and `SpeechAnalyzer` as whole-file only.
- README: corrected the command-line authorization claim, and documented the
  minimum OS versions, microphone permission and the main-queue callback
  default.

### Changed

- **Breaking:** `RecognitionRequestOptions::requires_on_device_recognition`
  returns `bool` instead of `Option<bool>`.
- **Breaking:** `AssetInstallationRequest::download_and_install` takes an
  `Option<Duration>`. `None` waits until the download finishes. On timeout it
  returns `SpeechError::TimedOut` and the download keeps running; a later call
  joins it.
- **Breaking (raw FFI):** the async callbacks take a status code,
  `sp_live_recognition_start` takes a retain callback and an out-status, the
  task starters take an out-status, and
  `sp_asset_installation_request_download_and_install` takes a timeout.
- **Breaking:** recognizer callbacks default to a dedicated serial background
  queue instead of the main queue, so recognition completes in command-line
  tools and under `cargo test`, where nothing services the main queue.
  `CallbackQueue::Main` is still available as an explicit choice.
  `CallbackQueue::default()`, `background()` and `named()` are serial, a
  background queue with zero concurrency is rejected with `InvalidArgument`,
  and the bridge also uses a serial queue when a payload names none.
- `rust-version` is now 1.82 (was 1.76), and `doom-fish-utils` 0.4.1 is
  required.

### Removed

- **Breaking:** the `recognize_url` feature, which gated nothing.

## [0.8.7] - 2026-06-06

- Fixed use-after-free races in the task-delegate, live-recognition and availability-delegate callback contexts, and pinned the FFI struct layout with compile-time and cross-language checks.

## [0.8.6] - 2026-05-20

- Migrated local `take_string` body to call `doom_fish_utils::ffi_string::take_owned_cstring_c`. Centralises the duplicated FFI take-string pattern fleet-wide. No public API change.

## [0.8.5] - 2026-05-20

- Clippy hygiene sweep: cleared all `-D warnings` lints across the crate. No public API change.

## [0.8.4] - 2026-05-20

- Widen `doom-fish-utils` dependency bound to `<0.4` so the 0.3.x SPSC-ring release resolves cleanly. No source changes.

## [0.8.3] - 2026-05-19

- Bump MSRV from 1.70 to 1.76 to match fleet baseline.

## [0.8.2] - 2026-05-19

### Added

- Added the Apple-style `SFSpeechLanguageModelConfiguration` alias for `LanguageModelConfiguration` so the Objective-C class name is available directly from Rust.

## [0.8.1] - 2026-05-16

### Fixed

- **Panic safety**: `extern "C"` callbacks that invoke user closures
  (`live::trampoline`, `task::task_event_trampoline`,
  `task::availability_trampoline`) now wrap the user-closure call in
  `doom_fish_utils::panic_safe::catch_user_panic`, preventing undefined
  behaviour from Rust panics unwinding across the FFI boundary into Swift.
- **SAFETY comments**: added `/// # Safety` doc sections to all three callback
  functions, and `// SAFETY:` inline comments to the inner `unsafe {}` blocks
  in `live::trampoline`.
- **Cargo hygiene**: `doom-fish-utils` version constraint widened to
  `>=0.1, <0.3` per workspace policy.

## [0.8.0] - 2026-05-16

### Added

- **`async` feature gate** — new `async_api` module with four executor-agnostic
  Rust `Future` newtypes backed by non-blocking Swift `@_cdecl` thunks:
  - `AsyncSpeechRecognizer::request_authorization()` →
    `AuthorizationFuture` — wraps `SFSpeechRecognizer.requestAuthorization`
    (completion-handler, macOS 13+)
  - `AsyncSpeechRecognizer::recognize_url(recognizer, request)` →
    `RecognizeUrlFuture` — one-shot `SFSpeechRecognitionTask` that resolves
    with `DetailedRecognitionResult` on the final result (macOS 13+)
  - `AsyncSpeechAnalyzer::analyze_in_path(analyzer, path)` →
    `AnalyzeUrlFuture` — wraps the native `async throws` `SpeechAnalyzer`
    API (macOS 26.0+; immediately resolves with `RecognizerUnavailable` on
    older OS)
  - `AsyncSpeechLanguageModel::prepare_custom_language_model(asset, config)` →
    `PrepareLanguageModelFuture` — wraps
    `SFSpeechLanguageModel.prepareCustomLanguageModel` (completion-handler,
    macOS 14.0+)
- Swift bridge `AsyncThunks.swift` — four `@_cdecl` thunks using
  `Task.detached` + `withCheckedThrowingContinuation` so Rust threads are
  never blocked
- `doom-fish-utils` dependency (path sibling, `AsyncCompletion` + `error_from_cstr`)
- `pollster` dev-dependency for running async examples synchronously
- `examples/05_async_smoke.rs` — headless smoke test; skips permission-gated
  paths with a graceful message
- `tests/async_api_tests.rs` — happy path + four error-path tests using
  `pollster::block_on`

## [0.7.1] - 2026-05-17

### Added

- Added macOS 26 analyzer-family bindings:
  - `SpeechAnalyzer`
  - `SpeechAnalyzerOptions`
  - `SpeechAnalyzerModelRetention`
  - `SpeechTranscriber`
  - `SpeechDetector`
  - `AnalysisContext`
  - `AnalyzerInput`
  - `SpeechModels`
  - `AssetInventory`
  - `AssetInstallationRequest`
  - `SpeechModule` / `SpeechModuleResult`
  - attributed `SpeechTranscriptionResult` spans via `SpeechAttributedText`
- Added custom language-model authoring/export bindings:
  - `SFCustomLanguageModelData`
  - `PhraseCount`
  - `CustomPronunciation`
  - `DataInsertableBuilder`
  - `PhraseCountGenerator`
  - `TemplatePhraseCountGenerator`
  - `CompoundTemplate`
  - `TemplateInsertableBuilder`
  - `PhraseCountsFromTemplates`
- Added `tests/macos26_surface_tests.rs` and `examples/04_macos26_surface_smoke.rs`.

### Changed

- Bumped crate version from `0.7.0` to `0.7.1`.
- Closed the remaining 44 `COVERAGE_AUDIT.md` gaps, raising audited coverage from `38.0%` to `100.0%`.
- Extended `SpeechFrameworkErrorCode` with the macOS 26 `SFSpeechError.Code` extension cases.

## [0.7.0] - 2026-05-16

### Added

- Added macOS 26 `DictationTranscriber` support via a new Swift bridge area:
  - `DictationTranscriber`
  - `DictationPreset`
  - `DictationTranscriberOptions`
  - `DictationTranscriptionResult`
  - locale discovery helpers (`supported_locales`, `installed_locales`, `supported_locale_equivalent_to`)
  - selected-locale and compatible-audio-format inspection
- Added `examples/03_dictation_smoke.rs` for file-based dictation smoke testing.
- Added `tests/dictation_tests.rs` plus expanded `tests/api_coverage.rs` to verify `SFSpeechRecognitionTaskDelegate` coverage and macOS 26 dictation symbols.
- Added `COVERAGE.md` for the Wave-C audited Speech.framework surface.

### Changed

- Bumped crate version from `0.6.0` to `0.7.0`.
- Updated docs to reflect the audited Speech surface and the SDK's `DictationTranscriber` naming.
- Made `examples/01_recognize_smoke.rs` degrade gracefully when the legacy recognizer times out in headless environments.
- Updated the Swift build to detect the macOS 26 SDK and compile dictation bindings conditionally.

## [0.6.0] - 2026-05-16

### Added

- Completed public-class coverage for macOS `Speech.framework`.
- New request builders:
  - `UrlRecognitionRequest`
  - `AudioBufferRecognitionRequest`
  - `RecognitionRequestOptions`
- New recognizer capabilities:
  - `SpeechRecognizer::supported_locales()`
  - `SpeechRecognizer::locale_identifier()`
  - `SpeechRecognizer::supports_on_device_recognition()`
  - recognizer-wide `TaskHint` and callback-queue configuration
  - `RecognizerAvailabilityObserver` for `SFSpeechRecognizerDelegate`
- New async task APIs:
  - `RecognitionTask`
  - `AudioBufferRecognitionTask`
  - `RecognitionTaskEvent`
  - `TaskState`
  - `TaskErrorInfo`
- Added full delegate-event coverage for `SFSpeechRecognitionTaskDelegate`:
  - speech detection
  - hypothesized transcriptions
  - final recognition
  - finished-reading-audio
  - cancellation
  - success/failure completion
  - processed-audio-duration progress
- Added detailed recognition result types:
  - `DetailedRecognitionResult`
  - `Transcription`
  - `TranscriptionSegmentDetails`
  - `TextRange`
  - `DetailedRecognitionMetadata`
  - `VoiceAnalytics`
  - `AcousticFeature`
- Added custom language-model preparation support:
  - `SpeechLanguageModel`
  - `LanguageModelConfiguration`
  - support for vocabulary paths
  - support for weighted configurations on macOS 26+
  - deprecated `clientIdentifier` preparation overloads
- Added manual audio-buffer append APIs:
  - interleaved `f32` PCM
  - interleaved `i16` PCM
  - unsafe raw `AVAudioPCMBuffer *` append
  - unsafe raw `CMSampleBufferRef` append
- Added end-to-end smoke example `examples/02_framework_smoke.rs`.
- Expanded `tests/api_coverage.rs` to audit every public Speech framework class against the Swift bridge.
- Added structured `SpeechFrameworkError` / `SpeechFrameworkErrorCode` mappings for `SFErrors.h`.

### Changed

- Bumped crate version from `0.5.0` to `0.6.0`.
- Updated crate documentation to describe full Speech framework coverage.
- `recognize_in_path`, `recognize_in_path_with_metadata`, and custom-model recognition now route through the richer request pipeline while preserving their existing signatures.

## [0.1.0] - Initial release

### Added

- `SpeechRecognizer` wraps `SFSpeechRecognizer` for file-based on-device
  speech recognition.
- `recognize_in_path(&Path) -> Result<RecognitionResult, SpeechError>`
  forces `requiresOnDeviceRecognition = true` and `shouldReportPartialResults
  = false`. Returns the final transcript + per-segment breakdown
  (text, confidence, timestamp, duration).
- Authorization helpers: `authorization_status()` + `request_authorization()`
  return an `AuthorizationStatus` enum (NotDetermined / Denied / Restricted /
  Authorized / Unknown).
- Locale helpers: `with_locale("en-US")`, `is_available()`,
  `default_locale_identifier()`.
- `SpeechError` variants: NotAuthorized, RecognizerUnavailable,
  AudioLoadFailed, RecognitionFailed, TimedOut, InvalidArgument, Unknown.
- Swift bridge wraps `SFSpeechRecognizer` + `SFSpeechURLRecognitionRequest`
  with a synchronous semaphore-based recogniser (60s timeout). Test helper
  `sp_test_helper_synthesize` uses `AVSpeechSynthesizer` to render text →
  AIFF so smoke tests don't need fixture audio.
- `recognize_url` feature flag (on by default) lets future
  audio-buffer-streaming features stay independently optional.
