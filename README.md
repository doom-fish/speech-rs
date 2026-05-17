# speech

Safe Rust bindings for Apple's [Speech](https://developer.apple.com/documentation/speech) framework on macOS.

> **Status:** v0.8.0 adds a Tier-1 `async` feature with four executor-agnostic
> Future newtypes.  v0.7 audited the classic `SFSpeech*` recognition surface
> and covers the macOS 26 analyzer, asset-inventory, and custom language-model
> authoring APIs alongside `DictationTranscriber`.

## Async API

Enable the `async` feature to get executor-agnostic `Future` wrappers for
Speech.framework's callback-handler and `async throws` APIs:

```toml
[dependencies]
speech = { version = "0.8", features = ["async"] }
```

```rust,no_run
use speech::async_api::{AsyncSpeechRecognizer, AsyncSpeechAnalyzer};
use speech::analyzer::{SpeechAnalyzer, SpeechTranscriber, SpeechTranscriberPreset};

# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
- `SpeechAnalyzer`, `SpeechTranscriber`, `SpeechDetector`, `AnalysisContext`, `SpeechModels`, and `AssetInventory`
- attributed `SpeechTranscriptionResult` values with Speech confidence/time-range spans plus `SpeechModule`/`SpeechModuleResult` traits
- `DictationTranscriber` with presets or explicit dictation options, locale discovery, compatible-audio-format inspection, and file-based transcription results
- `RecognizerAvailabilityObserver` for `SFSpeechRecognizerDelegate`

## Authorization

`SFSpeechRecognizer` requires `NSSpeechRecognitionUsageDescription` in your app's `Info.plist` plus an authorization request. CLI binaries without a proper bundle typically get `Denied`; the smoke example exits cleanly when authorization is unavailable.

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
