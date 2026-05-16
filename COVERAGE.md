# Speech.framework Wave-C coverage audit

This audit tracks the public Speech.framework surface explicitly requested for the v0.7.1 Wave-C sweep.

> Note: Xcode 26.2 exposes the dictation API under the Swift-only names `DictationTranscriber` and `DictationTranscriber.Result`. The user-requested names `DictationExtension` and `DictationExtensionTranscriptionResult` do not appear in the SDK, so the crate follows the SDK naming and adds doc aliases for the requested terms.

## Recognition core

| Apple API | Status | Notes |
| --- | --- | --- |
| `SFSpeechRecognizer` | ✅ implemented | `src/recognizer/mod.rs`, `swift-bridge/Sources/SpeechBridge/RecognizerExtras.swift`, `Speech.swift` |
| `SFSpeechRecognitionRequest` | ✅ implemented | `src/request.rs`, `Core.swift` |
| `SFSpeechURLRecognitionRequest` | ✅ implemented | `src/request.rs`, `RecognizerExtras.swift`, `TaskBridge.swift` |
| `SFSpeechAudioBufferRecognitionRequest` | ✅ implemented | `src/request.rs`, `src/task.rs`, `TaskBridge.swift` |
| `SFSpeechRecognitionResult` | ✅ implemented | `src/transcription.rs`, `Core.swift`, `RecognizerExtras.swift` |
| `SFSpeechRecognitionTask` | ✅ implemented | `src/task.rs`, `TaskBridge.swift` |
| `SFSpeechRecognitionTaskDelegate` | ✅ implemented | `RecognitionTaskEvent` maps all seven delegate callbacks; verified in `tests/api_coverage.rs` and bridged in `TaskBridge.swift` |

## Transcription + metadata

| Apple API | Status | Notes |
| --- | --- | --- |
| `SFTranscription` | ✅ implemented | `src/transcription.rs`, `Core.swift` |
| `SFTranscriptionSegment` | ✅ implemented | `src/transcription.rs`, `Core.swift` |
| `SFSpeechRecognitionMetadata` | ✅ implemented | `src/transcription.rs`, `Core.swift` |
| `SFVoiceAnalytics` | ✅ implemented | `src/transcription.rs`, `Core.swift` |
| `SFAcousticFeature` | ✅ implemented | `src/transcription.rs`, `Core.swift` |

## Custom language models

| Apple API | Status | Notes |
| --- | --- | --- |
| `SFSpeechLanguageModel` | ✅ implemented | `src/language_model.rs`, `swift-bridge/Sources/SpeechBridge/LanguageModelBridge.swift` |
| `SFSpeechLanguageModel.Configuration` | ✅ implemented | `src/language_model.rs`, `Core.swift` |

## Analyzer + asset inventory (Swift-only macOS 26 surface)

| Apple API | Status | Notes |
| --- | --- | --- |
| `SpeechAnalyzer` | ✅ implemented | `src/analyzer.rs`, `swift-bridge/Sources/SpeechBridge/AnalyzerBridge.swift` |
| `SpeechTranscriber` | ✅ implemented | `src/analyzer.rs`, `AnalyzerBridge.swift` |
| `SpeechDetector` | ✅ implemented | `src/analyzer.rs`, `AnalyzerBridge.swift` |
| `AssetInventory` / `AssetInstallationRequest` | ✅ implemented | `src/asset_inventory.rs`, `swift-bridge/Sources/SpeechBridge/AssetInventoryBridge.swift` |
| `AnalysisContext`, `AnalyzerInput`, `SpeechModels` | ✅ implemented | `src/analyzer.rs`, `AnalyzerBridge.swift` |
| `SpeechAttributes` helpers | ✅ implemented | `SpeechAttributedText`, `SpeechAttributeSpan`, `SpeechConfidenceAttribute`, `SpeechTimeRangeAttribute` in `src/analyzer.rs`, encoded by `AnalyzerBridge.swift` |

## Custom language-model authoring

| Apple API | Status | Notes |
| --- | --- | --- |
| `SFCustomLanguageModelData` | ✅ implemented | `src/custom_language_model.rs`, `swift-bridge/Sources/SpeechBridge/CustomLanguageModelDataBridge.swift` |
| `PhraseCount`, `CustomPronunciation`, builders, generators, templates` | ✅ implemented | `src/custom_language_model.rs` |

## Dictation (Swift-only macOS 26 surface)

| Apple API | Status | Notes |
| --- | --- | --- |
| `DictationTranscriber` (`DictationExtension`) | ✅ implemented | `src/dictation.rs`, `swift-bridge/Sources/SpeechBridge/DictationBridge.swift` |
| `DictationTranscriber.Preset` | ✅ implemented | `src/dictation.rs`, `DictationBridge.swift` |
| `DictationTranscriber.ContentHint` | ✅ implemented | `src/dictation.rs`, `DictationBridge.swift` |
| `DictationTranscriber.TranscriptionOption` | ✅ implemented | `src/dictation.rs`, `DictationBridge.swift` |
| `DictationTranscriber.ReportingOption` | ✅ implemented | `src/dictation.rs`, `DictationBridge.swift` |
| `DictationTranscriber.ResultAttributeOption` | ✅ implemented | `src/dictation.rs`, `DictationBridge.swift` |
| `DictationTranscriber.Result` (`DictationExtensionTranscriptionResult`) | ✅ implemented | `DictationTranscriptionResult` in `src/dictation.rs`, encoded by `DictationBridge.swift` |

## Verification

- Structural API audit: `cargo test --test api_coverage -- --nocapture`
- Dictation API unit tests: `cargo test --test dictation_tests`
- Analyzer / asset / custom-LM unit tests: `cargo test --test macos26_surface_tests`
- Full validation: `cargo clippy --all-targets -- -D warnings && cargo test && for ex in examples/*.rs; do cargo run --example "$(basename "$ex" .rs)"; done`
