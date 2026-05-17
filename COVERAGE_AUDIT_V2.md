# speech-rs coverage audit v2 (vs MacOSX26.2.sdk)

SDK_PUBLIC_SYMBOLS: 77
VERIFIED: 71
GAPS: 0
EXEMPT: 6
COVERAGE_PCT: 100.0%

Audit methodology: SDK enumeration was performed against Speech.framework's Obj-C headers (SFErrors.h, SFSpeechRecognizer.h, SFSpeechRecognitionTask.h, SFSpeechRecognitionRequest.h, SFSpeechRecognitionResult.h, SFTranscription.h, SFTranscriptionSegment.h, SFSpeechRecognitionMetadata.h, SFVoiceAnalytics.h, SFSpeechLanguageModel.h) and the macOS-specific Swift interface (arm64e-apple-macos.swiftinterface). Symbols enumerated include top-level types (interfaces, protocols, structs, enums), constants, and macOS 26.0-specific additions. Deprecated methods are tracked separately under EXEMPT with their respective SDK deprecation attributes per v2 instructions.

## 🟢 VERIFIED
| Symbol | Kind | Header | Wrapped by |
| --- | --- | --- | --- |
| `SFSpeechErrorDomain` | const | `SFErrors.h` | `SPEECH_ERROR_DOMAIN` |
| `SFSpeechErrorCode` | enum | `SFErrors.h` | `SpeechFrameworkErrorCode` |
| `SFSpeechRecognitionTaskHint` | enum | `SFSpeechRecognitionTaskHint.h` | `TaskHint` |
| `SFSpeechRecognizerAuthorizationStatus` | enum | `SFSpeechRecognizer.h` | `AuthorizationStatus` |
| `SFSpeechRecognizer` | class | `SFSpeechRecognizer.h` | `SpeechRecognizer` |
| `SFSpeechRecognizerDelegate` | protocol | `SFSpeechRecognizer.h` | `RecognizerAvailabilityObserver` |
| `SFSpeechRecognitionTaskState` | enum | `SFSpeechRecognitionTask.h` | `TaskState` |
| `SFSpeechRecognitionTask` | class | `SFSpeechRecognitionTask.h` | `RecognitionTask`, `AudioBufferRecognitionTask` |
| `SFSpeechRecognitionTaskDelegate` | protocol | `SFSpeechRecognitionTask.h` | `RecognitionTaskEvent` callback bridge |
| `SFSpeechRecognitionRequest` | class | `SFSpeechRecognitionRequest.h` | `RecognitionRequestOptions` |
| `SFSpeechURLRecognitionRequest` | class | `SFSpeechRecognitionRequest.h` | `UrlRecognitionRequest` |
| `SFSpeechAudioBufferRecognitionRequest` | class | `SFSpeechRecognitionRequest.h` | `AudioBufferRecognitionRequest`, `LiveRecognition` |
| `SFSpeechRecognitionResult` | class | `SFSpeechRecognitionResult.h` | `RecognitionResult`, `DetailedRecognitionResult` |
| `SFTranscription` | class | `SFTranscription.h` | `Transcription` |
| `SFTranscriptionSegment` | class | `SFTranscriptionSegment.h` | `TranscriptionSegment`, `TranscriptionSegmentDetails` |
| `SFSpeechRecognitionMetadata` | class | `SFSpeechRecognitionMetadata.h` | `RecognitionMetadata`, `DetailedRecognitionMetadata` |
| `SFAcousticFeature` | class | `SFVoiceAnalytics.h` | `AcousticFeature` |
| `SFVoiceAnalytics` | class | `SFVoiceAnalytics.h` | `VoiceAnalytics` |
| `SFSpeechLanguageModel.Configuration` | class | `SFSpeechLanguageModel.h` | `LanguageModelConfiguration` |
| `SFSpeechLanguageModel` | class | `SFSpeechLanguageModel.h` | `SpeechLanguageModel` |
| `SFSpeechLanguageModelConfiguration.weight` | property | `SFSpeechLanguageModel.h` | `LanguageModelConfiguration` |
| `DictationTranscriber` | class | `Speech.swiftinterface` | `DictationTranscriber` |
| `DictationTranscriber.Preset` | struct | `Speech.swiftinterface` | `DictationPreset` |
| `DictationTranscriber.ContentHint` | struct | `Speech.swiftinterface` | `DictationContentHint` |
| `DictationTranscriber.TranscriptionOption` | enum | `Speech.swiftinterface` | `DictationTranscriptionOption` |
| `DictationTranscriber.ReportingOption` | enum | `Speech.swiftinterface` | `DictationReportingOption` |
| `DictationTranscriber.ResultAttributeOption` | enum | `Speech.swiftinterface` | `DictationResultAttributeOption` |
| `DictationTranscriber.Result` | struct | `Speech.swiftinterface` | `DictationTranscriptionResult` |
| `AssetInventory` | class | `Speech.swiftinterface` | `AssetInventory` |
| `AssetInventory.Status` | enum | `Speech.swiftinterface` | `AssetInventoryStatus` |
| `LocaleDependentSpeechModule` | protocol | `Speech.swiftinterface` | `LocaleDependentSpeechModule` |
| `Foundation.AttributeScopes.SpeechAttributes` | struct | `Speech.swiftinterface` | `SpeechAttributes`, `SpeechAttributedText` |
| `Foundation.AttributeScopes.SpeechAttributes.ConfidenceAttribute` | struct | `Speech.swiftinterface` | `SpeechConfidenceAttribute` |
| `Foundation.AttributeScopes.SpeechAttributes.TimeRangeAttribute` | struct | `Speech.swiftinterface` | `SpeechTimeRangeAttribute` |
| `Foundation.AttributedString.rangeOfAudioTimeRangeAttributes(intersecting:)` | extension func | `Speech.swiftinterface` | `SpeechAttributedText::range_of_audio_time_range_attributes_intersecting` |
| `SpeechAnalyzer` | actor | `Speech.swiftinterface` | `SpeechAnalyzer` |
| `AnalyzerInput` | struct | `Speech.swiftinterface` | `AnalyzerInput` |
| `SpeechModels` | enum | `Speech.swiftinterface` | `SpeechModels` |
| `SpeechDetector` | class | `Speech.swiftinterface` | `SpeechDetector` |
| `SpeechDetector.SensitivityLevel` | enum | `Speech.swiftinterface` | `SpeechDetectorSensitivityLevel` |
| `SpeechDetector.DetectionOptions` | struct | `Speech.swiftinterface` | `SpeechDetectionOptions` |
| `SpeechDetector.Result` | struct | `Speech.swiftinterface` | `SpeechDetectionResult` |
| `SpeechModule` | protocol | `Speech.swiftinterface` | `SpeechModule` |
| `SpeechModuleResult` | protocol | `Speech.swiftinterface` | `SpeechModuleResult` |
| `SpeechModuleResult.isFinal` | extension var | `Speech.swiftinterface` | `SpeechModuleResult::is_final` |
| `SpeechTranscriber` | class | `Speech.swiftinterface` | `SpeechTranscriber` |
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
| `SFCustomLanguageModelData` | class | `Speech.swiftinterface` | `SFCustomLanguageModelData` |
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
| `SFSpeechRecognitionRequest.interactionIdentifier` | property | `SFSpeechRecognitionRequest.h` | Deprecated on macOS 12; `speech-rs` still exposes it via `RecognitionRequestOptions`, but excluded from coverage per audit v2. | `NS_DEPRECATED(10_15, 12_0, 10_0, 15_0, ...)` |
| `SFTranscription.speakingRate` | property | `SFTranscription.h` | Deprecated in favor of `SFSpeechRecognitionMetadata`; excluded from coverage math. | `NS_DEPRECATED(10_15, 11_3, 13_0, 14_5, ...)` |
| `SFTranscription.averagePauseDuration` | property | `SFTranscription.h` | Deprecated in favor of `SFSpeechRecognitionMetadata`; excluded from coverage math. | `NS_DEPRECATED(10_15, 11_3, 13_0, 14_5, ...)` |
| `SFTranscriptionSegment.voiceAnalytics` | property | `SFTranscriptionSegment.h` | Deprecated in favor of `SFSpeechRecognitionMetadata`; excluded from coverage math. | `NS_DEPRECATED(10_15, 11_3, 13_0, 14_5, ...)` |
| `SFSpeechLanguageModel.prepareCustomLanguageModelForUrl:clientIdentifier:configuration:completion:` | class method | `SFSpeechLanguageModel.h` | Deprecated in macOS 26; `speech-rs` keeps a deprecated wrapper for compatibility, but excluded from coverage. | `API_DEPRECATED_WITH_REPLACEMENT(... macos(14, 26.0) ...)` |
| `SFSpeechLanguageModel.prepareCustomLanguageModelForUrl:clientIdentifier:configuration:ignoresCache:completion:` | class method | `SFSpeechLanguageModel.h` | Deprecated in macOS 26; `speech-rs` keeps a deprecated wrapper for compatibility, but excluded from coverage. | `API_DEPRECATED_WITH_REPLACEMENT(... macos(14, 26.0) ...)` |
