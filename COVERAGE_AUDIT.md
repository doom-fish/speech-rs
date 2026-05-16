# speech-rs coverage audit (vs MacOSX26.2.sdk)

This audit treats exported Obj-C types/protocols/enums/constants as one symbol each, then adds the Swift-overlay-only types and standalone extension helpers that materially expand Speech.framework on macOS 26.2. For the legacy Obj-C interfaces, member-level completeness is also validated by `cargo test --test api_coverage -- --nocapture`; deprecated members are broken out under EXEMPT and excluded from the coverage denominator even when `speech-rs` still wraps them.

SDK_PUBLIC_SYMBOLS: 77
VERIFIED: 27
GAPS: 44
EXEMPT: 6
COVERAGE_PCT: 38.0%

## 🟢 VERIFIED
| Symbol | Kind | Header | Wrapped by |
| --- | --- | --- | --- |
| `SFSpeechErrorDomain` | const | `SFErrors.h` | `SPEECH_ERROR_DOMAIN` |
| `SFSpeechErrorCode` | enum | `SFErrors.h` | `SpeechFrameworkErrorCode` |
| `SFSpeechRecognitionTaskHint` | enum | `SFSpeechRecognitionTaskHint.h` | `TaskHint` |
| `SFSpeechRecognizerAuthorizationStatus` | enum | `SFSpeechRecognizer.h` | `AuthorizationStatus` |
| `SFSpeechRecognizer` | class | `SFSpeechRecognizer.h` | `SpeechRecognizer` (member surface verified in `tests/api_coverage.rs`) |
| `SFSpeechRecognizerDelegate` | protocol | `SFSpeechRecognizer.h` | `SpeechRecognizer::observe_availability_changes`, `RecognizerAvailabilityObserver` |
| `SFSpeechRecognitionTaskState` | enum | `SFSpeechRecognitionTask.h` | `TaskState` |
| `SFSpeechRecognitionTask` | class | `SFSpeechRecognitionTask.h` | `RecognitionTask`, `AudioBufferRecognitionTask` |
| `SFSpeechRecognitionTaskDelegate` | protocol | `SFSpeechRecognitionTask.h` | `RecognitionTaskEvent` callback bridge |
| `SFSpeechRecognitionRequest` | class | `SFSpeechRecognitionRequest.h` | `RecognitionRequestOptions` (non-deprecated surface) |
| `SFSpeechURLRecognitionRequest` | class | `SFSpeechRecognitionRequest.h` | `UrlRecognitionRequest` |
| `SFSpeechAudioBufferRecognitionRequest` | class | `SFSpeechRecognitionRequest.h` | `AudioBufferRecognitionRequest`, `AudioBufferRecognitionTask`, `LiveRecognition` |
| `SFSpeechRecognitionResult` | class | `SFSpeechRecognitionResult.h` | `DetailedRecognitionResult`, `RecognitionResult` |
| `SFTranscription` | class | `SFTranscription.h` | `Transcription` (non-deprecated surface) |
| `SFTranscriptionSegment` | class | `SFTranscriptionSegment.h` | `TranscriptionSegmentDetails`, `TranscriptionSegment` (non-deprecated surface) |
| `SFSpeechRecognitionMetadata` | class | `SFSpeechRecognitionMetadata.h` | `DetailedRecognitionMetadata`, `RecognitionMetadata` |
| `SFAcousticFeature` | class | `SFVoiceAnalytics.h` | `AcousticFeature` |
| `SFVoiceAnalytics` | class | `SFVoiceAnalytics.h` | `VoiceAnalytics` |
| `SFSpeechLanguageModel.Configuration` | class | `SFSpeechLanguageModel.h` | `LanguageModelConfiguration` |
| `SFSpeechLanguageModel` | class | `SFSpeechLanguageModel.h` | `SpeechLanguageModel` |
| `DictationTranscriber` | class | `Speech.swiftinterface` | `DictationTranscriber::{new,with_options,supported_locales,installed_locales,supported_locale_equivalent_to,selected_locales,available_compatible_audio_formats,transcribe_in_path}` |
| `DictationTranscriber.Preset` | struct | `Speech.swiftinterface` | `DictationPreset` |
| `DictationTranscriber.ContentHint` | struct | `Speech.swiftinterface` | `DictationContentHint` |
| `DictationTranscriber.TranscriptionOption` | enum | `Speech.swiftinterface` | `DictationTranscriptionOption` |
| `DictationTranscriber.ReportingOption` | enum | `Speech.swiftinterface` | `DictationReportingOption` |
| `DictationTranscriber.ResultAttributeOption` | enum | `Speech.swiftinterface` | `DictationResultAttributeOption` |
| `DictationTranscriber.Result` | struct | `Speech.swiftinterface` | `DictationTranscriptionResult` |

## 🔴 GAPS
| Symbol | Kind | Header | Notes |
| --- | --- | --- | --- |
| `AssetInventory` | class | `Speech.swiftinterface` | No Rust API for reserving, releasing, or querying downloadable Speech assets. |
| `AssetInventory.Status` | enum | `Speech.swiftinterface` | No equivalent status enum for asset installation state. |
| `LocaleDependentSpeechModule` | protocol | `Speech.swiftinterface` | No trait/protocol modeling the locale-dependent module family introduced in macOS 26. |
| `Foundation.AttributeScopes.SpeechAttributes` | struct | `Speech.swiftinterface` | `speech-rs` flattens transcriber output into plain strings instead of exposing attributed Speech attributes. |
| `Foundation.AttributeScopes.SpeechAttributes.ConfidenceAttribute` | struct | `Speech.swiftinterface` | No attributed-string key for transcription confidence. |
| `Foundation.AttributeScopes.SpeechAttributes.TimeRangeAttribute` | struct | `Speech.swiftinterface` | No attributed-string key for audio time ranges. |
| `Foundation.AttributedString.rangeOfAudioTimeRangeAttributes(intersecting:)` | extension func | `Speech.swiftinterface` | No helper for querying Speech time-range attributes in attributed text. |
| `SpeechAnalyzer` | actor | `Speech.swiftinterface` | No analyzer pipeline for composing macOS 26 Speech modules over async audio input. |
| `AnalyzerInput` | struct | `Speech.swiftinterface` | No typed analyzer-input wrapper for `AVAudioPCMBuffer` plus timestamps. |
| `SpeechModels` | enum | `Speech.swiftinterface` | No binding for `endRetention()` model-lifecycle control. |
| `SpeechDetector` | class | `Speech.swiftinterface` | No voice-activity / speech-detection module. |
| `SpeechDetector.SensitivityLevel` | enum | `Speech.swiftinterface` | No equivalent sensitivity enum. |
| `SpeechDetector.DetectionOptions` | struct | `Speech.swiftinterface` | No detector configuration type. |
| `SpeechDetector.Result` | struct | `Speech.swiftinterface` | No detector result type. |
| `SpeechModule` | protocol | `Speech.swiftinterface` | No general module abstraction for the analyzer-based API family. |
| `SpeechModuleResult` | protocol | `Speech.swiftinterface` | No generic module-result protocol. |
| `SpeechModuleResult.isFinal` | extension var | `Speech.swiftinterface` | No generalized finalization helper for analyzer-module results. |
| `SpeechTranscriber` | class | `Speech.swiftinterface` | No binding for the newer analyzer-based transcription API. |
| `SpeechTranscriber.Preset` | struct | `Speech.swiftinterface` | No equivalent preset type. |
| `SpeechTranscriber.TranscriptionOption` | enum | `Speech.swiftinterface` | No equivalent transcription-option enum. |
| `SpeechTranscriber.ReportingOption` | enum | `Speech.swiftinterface` | No equivalent reporting-option enum. |
| `SpeechTranscriber.ResultAttributeOption` | enum | `Speech.swiftinterface` | No equivalent result-attribute enum. |
| `SpeechTranscriber.Result` | struct | `Speech.swiftinterface` | No analyzer-based transcription result type. |
| `SpeechAnalyzer.Options` | struct | `Speech.swiftinterface` | No analyzer options type. |
| `SpeechAnalyzer.Options.ModelRetention` | enum | `Speech.swiftinterface` | No equivalent model-retention enum. |
| `AnalysisContext` | class | `Speech.swiftinterface` | No API for analyzer contextual strings or user-data injection. |
| `AnalysisContext.ContextualStringsTag` | struct | `Speech.swiftinterface` | No equivalent contextual-string tag type. |
| `AnalysisContext.UserDataTag` | struct | `Speech.swiftinterface` | No equivalent user-data tag type. |
| `AssetInstallationRequest` | class | `Speech.swiftinterface` | No binding for async asset download/install progress. |
| `DataInsertable` | protocol | `Speech.swiftinterface` | No training-data builder trait. |
| `TemplateInsertable` | protocol | `Speech.swiftinterface` | No template-training builder trait. |
| `SFCustomLanguageModelData` | class | `Speech.swiftinterface` | `speech-rs` can prepare compiled assets, but cannot author/export custom language-model training data. |
| `SFCustomLanguageModelData.PhraseCount` | struct | `Speech.swiftinterface` | No equivalent phrase-count training-data node. |
| `SFCustomLanguageModelData.CustomPronunciation` | struct | `Speech.swiftinterface` | No equivalent custom-pronunciation node. |
| `SFCustomLanguageModelData.DataInsertableBuilder` | struct | `Speech.swiftinterface` | No result-builder-style API for composing training data. |
| `SFCustomLanguageModelData.PhraseCountGenerator` | class | `Speech.swiftinterface` | No async phrase-count generator binding. |
| `SFCustomLanguageModelData.PhraseCountGenerator.Iterator` | class | `Speech.swiftinterface` | No iterator binding for phrase-count generation. |
| `SFCustomLanguageModelData.TemplatePhraseCountGenerator` | class | `Speech.swiftinterface` | No template-based phrase-count generator binding. |
| `SFCustomLanguageModelData.TemplatePhraseCountGenerator.Template` | struct | `Speech.swiftinterface` | No template node binding. |
| `SFCustomLanguageModelData.TemplatePhraseCountGenerator.Iterator` | class | `Speech.swiftinterface` | No iterator binding for template-driven phrase-count generation. |
| `SFCustomLanguageModelData.CompoundTemplate` | struct | `Speech.swiftinterface` | No compound-template binding. |
| `SFCustomLanguageModelData.TemplateInsertableBuilder` | struct | `Speech.swiftinterface` | No result-builder-style API for composing template inputs. |
| `SFCustomLanguageModelData.PhraseCountsFromTemplates` | struct | `Speech.swiftinterface` | No helper that expands template classes into phrase counts. |
| `SFSpeechError.Code` (macOS 26 extension cases) | Swift extension | `Speech.swiftinterface` | `SpeechFrameworkErrorCode` stops at the header-defined cases; `audioDisordered`, `unexpectedAudioFormat`, `noModel`, `assetLocaleNotAllocated`, `tooManyAssetLocalesAllocated`, `incompatibleAudioFormats`, `moduleOutputFailed`, `cannotAllocateUnsupportedLocale`, and `insufficientResources` are not surfaced. |

## ⏭️ EXEMPT
| Symbol | Kind | Header | Reason | SDK attribute |
| --- | --- | --- | --- | --- |
| `SFSpeechRecognitionRequest.interactionIdentifier` | property | `SFSpeechRecognitionRequest.h` | Deprecated on macOS 12; `speech-rs` still exposes it via `RecognitionRequestOptions`, but it is excluded from coverage math per audit instructions. | `NS_DEPRECATED(10_15, 12_0, 10_0, 15_0, ...)` |
| `SFTranscription.speakingRate` | property | `SFTranscription.h` | Deprecated in favor of `SFSpeechRecognitionMetadata`; excluded from coverage math. | `NS_DEPRECATED(10_15, 11_3, 13_0, 14_5, ...)` |
| `SFTranscription.averagePauseDuration` | property | `SFTranscription.h` | Deprecated in favor of `SFSpeechRecognitionMetadata`; excluded from coverage math. | `NS_DEPRECATED(10_15, 11_3, 13_0, 14_5, ...)` |
| `SFTranscriptionSegment.voiceAnalytics` | property | `SFTranscriptionSegment.h` | Deprecated in favor of `SFSpeechRecognitionMetadata`; excluded from coverage math. | `NS_DEPRECATED(10_15, 11_3, 13_0, 14_5, ...)` |
| `SFSpeechLanguageModel.prepareCustomLanguageModelForUrl:clientIdentifier:configuration:completion:` | class method | `SFSpeechLanguageModel.h` | Deprecated in macOS 26; `speech-rs` keeps a deprecated wrapper for compatibility, but the symbol is exempt. | `API_DEPRECATED_WITH_REPLACEMENT(... macos(14, 26.0) ...)` |
| `SFSpeechLanguageModel.prepareCustomLanguageModelForUrl:clientIdentifier:configuration:ignoresCache:completion:` | class method | `SFSpeechLanguageModel.h` | Deprecated in macOS 26; `speech-rs` keeps a deprecated wrapper for compatibility, but the symbol is exempt. | `API_DEPRECATED_WITH_REPLACEMENT(... macos(14, 26.0) ...)` |
