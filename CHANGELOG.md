# Changelog

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
