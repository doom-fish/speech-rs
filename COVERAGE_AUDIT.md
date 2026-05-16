# speech-rs coverage audit (vs MacOSX26.2.sdk)

This audit treats exported Obj-C types/protocols/enums/constants as one symbol each, then adds the Swift-overlay-only types and standalone extension helpers that materially expand Speech.framework on macOS 26.2. For the legacy Obj-C interfaces, member-level completeness is also validated by `cargo test --test api_coverage -- --nocapture`; deprecated members are broken out under EXEMPT and excluded from the coverage denominator even when `speech-rs` still wraps them.

SDK_PUBLIC_SYMBOLS: 77
VERIFIED: 71
GAPS: 0
EXEMPT: 6
COVERAGE_PCT: 100.0%

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
| `AssetInventory` | class | `Speech.swiftinterface` | `AssetInventory::{maximum_reserved_locales,reserved_locales,reserve_locale,release_reserved_locale,status_for_modules,asset_installation_request_for_modules}` |
| `AssetInventory.Status` | enum | `Speech.swiftinterface` | `AssetInventoryStatus` |
| `LocaleDependentSpeechModule` | protocol | `Speech.swiftinterface` | `LocaleDependentSpeechModule` |
| `Foundation.AttributeScopes.SpeechAttributes` | struct | `Speech.swiftinterface` | `SpeechAttributes`, `SpeechAttributedText` |
| `Foundation.AttributeScopes.SpeechAttributes.ConfidenceAttribute` | struct | `Speech.swiftinterface` | `SpeechConfidenceAttribute` |
| `Foundation.AttributeScopes.SpeechAttributes.TimeRangeAttribute` | struct | `Speech.swiftinterface` | `SpeechTimeRangeAttribute` |
| `Foundation.AttributedString.rangeOfAudioTimeRangeAttributes(intersecting:)` | extension func | `Speech.swiftinterface` | `SpeechAttributedText::range_of_audio_time_range_attributes_intersecting` |
| `SpeechAnalyzer` | actor | `Speech.swiftinterface` | `SpeechAnalyzer::{new,with_options,with_context,analyze_in_path,best_available_audio_format}` |
| `AnalyzerInput` | struct | `Speech.swiftinterface` | `AnalyzerInput::{from_audio_pcm_buffer_raw,from_audio_pcm_buffer_raw_with_start_time}` |
| `SpeechModels` | enum | `Speech.swiftinterface` | `SpeechModels::end_retention` |
| `SpeechDetector` | class | `Speech.swiftinterface` | `SpeechDetector::{new,default,available_compatible_audio_formats,detect_in_path}` |
| `SpeechDetector.SensitivityLevel` | enum | `Speech.swiftinterface` | `SpeechDetectorSensitivityLevel` |
| `SpeechDetector.DetectionOptions` | struct | `Speech.swiftinterface` | `SpeechDetectionOptions` |
| `SpeechDetector.Result` | struct | `Speech.swiftinterface` | `SpeechDetectionResult` |
| `SpeechModule` | protocol | `Speech.swiftinterface` | `SpeechModule`, `SpeechModuleDescriptor` |
| `SpeechModuleResult` | protocol | `Speech.swiftinterface` | `SpeechModuleResult` |
| `SpeechModuleResult.isFinal` | extension var | `Speech.swiftinterface` | `SpeechModuleResult::is_final` |
| `SpeechTranscriber` | class | `Speech.swiftinterface` | `SpeechTranscriber::{new,with_options,is_available,supported_locales,installed_locales,supported_locale_equivalent_to,selected_locales,available_compatible_audio_formats,transcribe_in_path}` |
| `SpeechTranscriber.Preset` | struct | `Speech.swiftinterface` | `SpeechTranscriberPreset` |
| `SpeechTranscriber.TranscriptionOption` | enum | `Speech.swiftinterface` | `SpeechTranscriptionOption` |
| `SpeechTranscriber.ReportingOption` | enum | `Speech.swiftinterface` | `SpeechTranscriberReportingOption` |
| `SpeechTranscriber.ResultAttributeOption` | enum | `Speech.swiftinterface` | `SpeechTranscriberResultAttributeOption` |
| `SpeechTranscriber.Result` | struct | `Speech.swiftinterface` | `SpeechTranscriptionResult` |
| `SpeechAnalyzer.Options` | struct | `Speech.swiftinterface` | `SpeechAnalyzerOptions` |
| `SpeechAnalyzer.Options.ModelRetention` | enum | `Speech.swiftinterface` | `SpeechAnalyzerModelRetention` |
| `AnalysisContext` | class | `Speech.swiftinterface` | `AnalysisContext` |
| `AnalysisContext.ContextualStringsTag` | struct | `Speech.swiftinterface` | `ContextualStringsTag` |
| `AnalysisContext.UserDataTag` | struct | `Speech.swiftinterface` | `UserDataTag` |
| `AssetInstallationRequest` | class | `Speech.swiftinterface` | `AssetInstallationRequest` |
| `DataInsertable` | protocol | `Speech.swiftinterface` | `DataInsertable` |
| `TemplateInsertable` | protocol | `Speech.swiftinterface` | `TemplateInsertable` |
| `SFCustomLanguageModelData` | class | `Speech.swiftinterface` | `SFCustomLanguageModelData::{new,insert,with_insertable,supported_phonemes,export_to}` |
| `SFCustomLanguageModelData.PhraseCount` | struct | `Speech.swiftinterface` | `PhraseCount` |
| `SFCustomLanguageModelData.CustomPronunciation` | struct | `Speech.swiftinterface` | `CustomPronunciation` |
| `SFCustomLanguageModelData.DataInsertableBuilder` | struct | `Speech.swiftinterface` | `DataInsertableBuilder` |
| `SFCustomLanguageModelData.PhraseCountGenerator` | class | `Speech.swiftinterface` | `PhraseCountGenerator` |
| `SFCustomLanguageModelData.PhraseCountGenerator.Iterator` | class | `Speech.swiftinterface` | `PhraseCountGeneratorIterator` |
| `SFCustomLanguageModelData.TemplatePhraseCountGenerator` | class | `Speech.swiftinterface` | `TemplatePhraseCountGenerator` |
| `SFCustomLanguageModelData.TemplatePhraseCountGenerator.Template` | struct | `Speech.swiftinterface` | `TemplatePhraseCountGeneratorTemplate` |
| `SFCustomLanguageModelData.TemplatePhraseCountGenerator.Iterator` | class | `Speech.swiftinterface` | `TemplatePhraseCountGeneratorIterator` |
| `SFCustomLanguageModelData.CompoundTemplate` | struct | `Speech.swiftinterface` | `CompoundTemplate` |
| `SFCustomLanguageModelData.TemplateInsertableBuilder` | struct | `Speech.swiftinterface` | `TemplateInsertableBuilder` |
| `SFCustomLanguageModelData.PhraseCountsFromTemplates` | struct | `Speech.swiftinterface` | `PhraseCountsFromTemplates` |
| `SFSpeechError.Code` (macOS 26 extension cases) | Swift extension | `Speech.swiftinterface` | `SpeechFrameworkErrorCode::{AudioDisordered,UnexpectedAudioFormat,NoModel,AssetLocaleNotAllocated,TooManyAssetLocalesAllocated,IncompatibleAudioFormats,ModuleOutputFailed,CannotAllocateUnsupportedLocale,InsufficientResources}` |

## 🔴 GAPS
None.

## ⏭️ EXEMPT
| Symbol | Kind | Header | Reason | SDK attribute |
| --- | --- | --- | --- | --- |
| `SFSpeechRecognitionRequest.interactionIdentifier` | property | `SFSpeechRecognitionRequest.h` | Deprecated on macOS 12; `speech-rs` still exposes it via `RecognitionRequestOptions`, but it is excluded from coverage math per audit instructions. | `NS_DEPRECATED(10_15, 12_0, 10_0, 15_0, ...)` |
| `SFTranscription.speakingRate` | property | `SFTranscription.h` | Deprecated in favor of `SFSpeechRecognitionMetadata`; excluded from coverage math. | `NS_DEPRECATED(10_15, 11_3, 13_0, 14_5, ...)` |
| `SFTranscription.averagePauseDuration` | property | `SFTranscription.h` | Deprecated in favor of `SFSpeechRecognitionMetadata`; excluded from coverage math. | `NS_DEPRECATED(10_15, 11_3, 13_0, 14_5, ...)` |
| `SFTranscriptionSegment.voiceAnalytics` | property | `SFTranscriptionSegment.h` | Deprecated in favor of `SFSpeechRecognitionMetadata`; excluded from coverage math. | `NS_DEPRECATED(10_15, 11_3, 13_0, 14_5, ...)` |
| `SFSpeechLanguageModel.prepareCustomLanguageModelForUrl:clientIdentifier:configuration:completion:` | class method | `SFSpeechLanguageModel.h` | Deprecated in macOS 26; `speech-rs` keeps a deprecated wrapper for compatibility, but the symbol is exempt. | `API_DEPRECATED_WITH_REPLACEMENT(... macos(14, 26.0) ...)` |
| `SFSpeechLanguageModel.prepareCustomLanguageModelForUrl:clientIdentifier:configuration:ignoresCache:completion:` | class method | `SFSpeechLanguageModel.h` | Deprecated in macOS 26; `speech-rs` keeps a deprecated wrapper for compatibility, but the symbol is exempt. | `API_DEPRECATED_WITH_REPLACEMENT(... macos(14, 26.0) ...)` |
