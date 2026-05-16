# speech

Safe Rust bindings for Apple's [Speech](https://developer.apple.com/documentation/speech) framework on macOS.

> **Status:** v0.7.0 audits the classic `SFSpeech*` recognition surface and adds macOS 26 `DictationTranscriber` support (the Xcode 26.2 SDK name for the dictation API sometimes described as “DictationExtension”).

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
- `SpeechLanguageModel::prepare_custom_language_model*` plus `LanguageModelConfiguration`
- `DictationTranscriber` with presets or explicit dictation options, locale discovery, compatible-audio-format inspection, and file-based transcription results
- `RecognizerAvailabilityObserver` for `SFSpeechRecognizerDelegate`

## Authorization

`SFSpeechRecognizer` requires `NSSpeechRecognitionUsageDescription` in your app's `Info.plist` plus an authorization request. CLI binaries without a proper bundle typically get `Denied`; the smoke example exits cleanly when authorization is unavailable.

## Smoke examples

Run the end-to-end framework smoke test with:

```bash
cargo run --all-features --example 02_framework_smoke
cargo run --all-features --example 03_dictation_smoke
```

`02_framework_smoke` exercises locale enumeration, recognizer configuration, native audio-format discovery, async task delegates, and synchronous URL recognition. `03_dictation_smoke` exercises the macOS 26 `DictationTranscriber` bridge. Both synthesize a short AIFF under `target/` and skip cleanly when authorization or OS support is unavailable.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
