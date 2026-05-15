# speech

Safe Rust bindings for Apple's [Speech](https://developer.apple.com/documentation/speech) framework — on-device speech recognition (`SFSpeechRecognizer`) on macOS.

> **Status:** experimental. v0.1 ships file-based transcription via `SpeechRecognizer::recognize_in_path`. Live audio-buffer streaming, partial-result callbacks, and language-model customisation land in v0.2.

## Quick start

```rust,no_run
use speech::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Speech recognition requires user authorization.
    if !SpeechRecognizer::authorization_status().is_authorized() {
        let status = SpeechRecognizer::request_authorization();
        if !status.is_authorized() {
            eprintln!("not authorized: {status:?}");
            return Ok(());
        }
    }

    let recognizer = SpeechRecognizer::new();
    let result = recognizer.recognize_in_path("/tmp/utterance.aiff")?;

    println!("Transcript: {}", result.transcript);
    for seg in &result.segments {
        println!("  [{:.2}] {} (t={:.2}s, dur={:.2}s)",
            seg.confidence, seg.text, seg.timestamp, seg.duration);
    }
    Ok(())
}
```

## Pipeline composition

```text
screencapturekit-rs ──► system audio ──► speech ──► transcript
                                                       │
                                                       ▼
                                                foundation-models
                                                ("summarise the meeting")
```

Pairs naturally with [`foundation-models`](https://github.com/doom-fish/foundation-models-rs) for transcribe → summarize / translate / extract pipelines, all running entirely on-device.

## Authorization

`SFSpeechRecognizer` requires `NSSpeechRecognitionUsageDescription` in your app's `Info.plist` plus an authorization request. CLI binaries / daemons without a proper bundle typically get `Denied` — the smoke test gracefully skips in that case.

## Roadmap

- [x] `SpeechRecognizer::recognize_in_path` (file-based, on-device)
- [ ] Live audio-buffer streaming (`SFSpeechAudioBufferRecognitionRequest`)
- [ ] Partial-result callbacks
- [ ] Per-segment alternative-string candidates
- [ ] Custom language-model support (`SFSpeechLanguageModel`)
- [ ] Async API

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
