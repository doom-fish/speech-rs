# Changelog

## [0.7.1] - 2026-05-16

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
