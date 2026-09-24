# speech

Safe Rust bindings for Apple's [Speech](https://developer.apple.com/documentation/speech) framework on macOS.

> **Status:** v0.9 requires on-device recognition by default and fixes
> use-after-free bugs in the async and live-recognition bridges. v0.8 added
> the Tier-1 `async` feature; v0.7 audited the classic `SFSpeech*`
> recognition surface and covers the macOS 26 analyzer, asset-inventory, and
> custom language-model authoring APIs alongside `DictationTranscriber`.

```toml
[dependencies]
speech = "0.9"
```

## Requirements

macOS 13 or later. Custom language models need macOS 14. The macOS 26
analyzer family (`SpeechAnalyzer`, `SpeechTranscriber`, `SpeechDetector`,
`AssetInventory`) and `DictationTranscriber` need macOS 26 at run time and a
build with the macOS 26 SDK; otherwise those calls return
`SpeechError::RecognizerUnavailable`.

## Async API

Enable the `async` feature to get executor-agnostic `Future` wrappers for
Speech.framework's callback-handler and `async throws` APIs:

```toml
[dependencies]
speech = { version = "0.9", features = ["async"] }
```

```rust,no_run
use speech::async_api::{AsyncSpeechRecognizer, AsyncSpeechAnalyzer};
use speech::analyzer::{SpeechAnalyzer, SpeechTranscriber, SpeechTranscriberPreset};

# async fn demo() -> Result<(), Box<dyn std::error::Error>> {
// 1. Request authorization (non-blocking)
let status = AsyncSpeechRecognizer::request_authorization().await?;
println!("status: {status:?}");

// 2. Recognize a URL file (one-shot, resolves with final result)
use speech::{recognizer::SpeechRecognizer, request::UrlRecognitionRequest};
let recognizer = SpeechRecognizer::new();
let request = UrlRecognitionRequest::new("audio.m4a");
let result = AsyncSpeechRecognizer::recognize_url(&recognizer, &request)?.await?;
println!("{}", result.best_transcription.formatted_string);
# Ok(())
# }
```

| Future | API | OS |
|--------|-----|----|
| `AuthorizationFuture` | `SFSpeechRecognizer.requestAuthorization` | macOS 13+ |
| `RecognizeUrlFuture` | `SFSpeechRecognitionTask` one-shot | macOS 13+ |
| `AnalyzeUrlFuture` | `SpeechAnalyzer` native `async throws` | macOS 26+ |
| `PrepareLanguageModelFuture` | `SFSpeechLanguageModel.prepareCustomLanguageModel` | macOS 14+ |

> **Tier-2 note:** multi-fire delegate patterns (live recognition updates,
> `SFSpeechRecognitionTaskDelegate` event streams) map to `Stream`s, not
> `Future`s, and are tracked in the Tier-2 milestone.

## Quick start

```rust,no_run
use speech::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !SpeechRecognizer::authorization_status().is_authorized() {
        let status = SpeechRecognizer::request_authorization();
        if !status.is_authorized() {
            eprintln!("speech authorization denied: {status:?}");
            return Ok(());
        }
    }

    let recognizer = SpeechRecognizer::new()
        .with_default_task_hint(TaskHint::Dictation)
        .with_callback_queue(CallbackQueue::named("speech-demo"));

    println!("supported locales: {}", SpeechRecognizer::supported_locales()?.len());

    let request = UrlRecognitionRequest::new("target/utterance.aiff").with_options(
        RecognitionRequestOptions::new()
            .with_contextual_strings(["doom fish", "speech rs"])
            .with_requires_on_device_recognition(true)
            .with_adds_punctuation(true),
    );

    let result = recognizer.recognize_request(&request)?;
    println!("best transcript: {}", result.transcript());
    println!("alternatives: {}", result.transcriptions.len());
    println!("metadata: {:?}", result.speech_recognition_metadata);
    Ok(())
}
```

## Highlights

- `SpeechRecognizer::supported_locales`, `locale_identifier`, `supports_on_device_recognition`
- `UrlRecognitionRequest` and `AudioBufferRecognitionRequest`
- `RecognitionRequestOptions` for `taskHint`, `contextualStrings`, `interactionIdentifier`, `shouldReportPartialResults`, `requiresOnDeviceRecognition`, `addsPunctuation`, and custom language models
- `RecognitionTask` / `AudioBufferRecognitionTask` with RAII cleanup, task state inspection, delegate events, cancellation, finishing, and manual PCM/sample-buffer appends
- `DetailedRecognitionResult`, `Transcription`, `TranscriptionSegmentDetails`, `DetailedRecognitionMetadata`, `VoiceAnalytics`, `AcousticFeature`
- `SpeechLanguageModel::prepare_custom_language_model*`, `LanguageModelConfiguration`, and `SFCustomLanguageModelData` authoring/export helpers
- `SpeechAnalyzer`, `SpeechTranscriber`, `SpeechDetector`, `AnalysisContext`, `SpeechModels`, and `AssetInventory` for whole-file analysis; live `SpeechAnalyzer` input (`AnalyzerInput` streams) is not supported yet
- attributed `SpeechTranscriptionResult` values with Speech confidence/time-range spans plus `SpeechModule`/`SpeechModuleResult` traits
- `DictationTranscriber` with presets or explicit dictation options, locale discovery, compatible-audio-format inspection, and file-based transcription results
- `RecognizerAvailabilityObserver` for `SFSpeechRecognizerDelegate`

## Authorization

`SFSpeechRecognizer` requires `NSSpeechRecognitionUsageDescription` in your app's `Info.plist` plus an authorization request. A command-line binary is authorized through the app that launched it (for example your terminal); if that app has no speech-recognition permission the status is `Denied` or `NotDetermined`, and the smoke examples exit cleanly. `LiveRecognition` and `start_microphone_task` also record from the microphone, which needs `NSMicrophoneUsageDescription` (or microphone permission for the launching app).

## Callback queues

Result handlers and task delegate events run on the recognizer's callback queue. By default every recognition gets its own serial background queue (`CallbackQueue::default()`, the same as `CallbackQueue::background()`), so callbacks arrive in order and don't depend on the main run loop. `CallbackQueue::Main` is an explicit choice for apps that run the main run loop; in a program that doesn't (most command-line tools, and `cargo test`), main-queue callbacks never run, recognition never completes, and the blocking calls time out. `LiveRecognition::start` takes a `SpeechRecognizer`, so live updates follow the same locale and callback queue.

## Privacy: on-device by default

`RecognitionRequestOptions::default()` requires on-device recognition, and every `SFSpeechRecognizer` path in this crate sends that setting explicitly: `recognize_in_path`, `recognize_request`, URL and audio-buffer tasks, `LiveRecognition`, and the async API. Audio is never sent to Apple's servers unless you opt in:

```rust,no_run
use speech::prelude::*;

let request = UrlRecognitionRequest::new("audio.m4a").with_options(
    RecognitionRequestOptions::new().with_requires_on_device_recognition(false),
);
```

On-device requests fail when the locale has no on-device model, so check `SpeechRecognizer::supports_on_device_recognition()` before relying on them. The macOS 26 analyzer modules (`SpeechTranscriber`, `SpeechDetector`, `DictationTranscriber`) use models installed through `AssetInventory` and don't take this option.

## Smoke examples

Run the end-to-end framework smoke test with:

```bash
cargo run --all-features --example 02_framework_smoke
cargo run --all-features --example 03_dictation_smoke
cargo run --all-features --example 04_macos26_surface_smoke
```

`02_framework_smoke` exercises locale enumeration, recognizer configuration, native audio-format discovery, async task delegates, and synchronous URL recognition. `03_dictation_smoke` exercises the macOS 26 `DictationTranscriber` bridge. `04_macos26_surface_smoke` walks the analyzer/transcriber/detector pipeline, asset-inventory queries, and `SFCustomLanguageModelData` export helpers. All examples synthesize short AIFF files under `target/` and skip cleanly when authorization or OS support is unavailable.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
