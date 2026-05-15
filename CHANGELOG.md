# Changelog

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
