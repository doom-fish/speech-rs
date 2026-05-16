#![doc = include_str!("../README.md")]
//!
//! ---
//!
//! # API Documentation
//!
//! Safe Rust bindings for Apple's
//! [Speech](https://developer.apple.com/documentation/speech) framework.
//!
//! This crate audits the classic `SFSpeech*` recognition surface on macOS and
//! adds the macOS 26 analyzer family (`SpeechAnalyzer`, `SpeechTranscriber`,
//! `SpeechDetector`, and `AssetInventory`) alongside `DictationTranscriber`, URL
//! and audio-buffer requests, delegate-driven task events, detailed
//! transcription metadata, voice analytics, and custom language model
//! preparation and authoring.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod analyzer;
pub mod asset_inventory;
pub mod custom_language_model;
pub mod dictation;
pub mod error;
pub mod ffi;
pub mod language_model;
mod private;
pub mod recognizer;
pub mod request;
pub mod task;
pub mod transcription;

pub mod live;

pub use analyzer::{
    AnalysisContext, AnalyzerInput, AudioTimeRange, ContextualStringsTag,
    LocaleDependentSpeechModule, SpeechAnalyzer, SpeechAnalyzerModelRetention,
    SpeechAnalyzerModuleOutput, SpeechAnalyzerModuleResults, SpeechAnalyzerOptions,
    SpeechAnalyzerPriority, SpeechAttributedText, SpeechAttributeSpan, SpeechAttributes,
    SpeechConfidenceAttribute, SpeechDetectionOptions, SpeechDetectionResult, SpeechDetector,
    SpeechDetectorSensitivityLevel, SpeechModels, SpeechModule, SpeechModuleDescriptor,
    SpeechModuleResult, SpeechTimeRangeAttribute, SpeechTranscriber,
    SpeechTranscriberOptions, SpeechTranscriberPreset, SpeechTranscriberReportingOption,
    SpeechTranscriberResultAttributeOption, SpeechTranscriptionOption,
    SpeechTranscriptionResult, UserDataTag,
};
pub use asset_inventory::{
    AssetInstallationProgress, AssetInstallationRequest, AssetInventory, AssetInventoryStatus,
};
pub use custom_language_model::{
    CompoundTemplate, CustomPronunciation, DataInsertable, DataInsertableBuilder, PhraseCount,
    PhraseCountGenerator, PhraseCountGeneratorIterator, PhraseCountsFromTemplates,
    SFCustomLanguageModelData, TemplateInsertable, TemplateInsertableBuilder,
    TemplatePhraseCountGenerator, TemplatePhraseCountGeneratorIterator,
    TemplatePhraseCountGeneratorTemplate,
};
pub use dictation::{
    DictationAudioTimeRange, DictationContentHint, DictationPreset, DictationReportingOption,
    DictationResultAttributeOption, DictationTranscriber, DictationTranscriberOptions,
    DictationTranscriptionOption, DictationTranscriptionResult,
};
pub use error::{
    AuthorizationStatus, SpeechError, SpeechFrameworkError, SpeechFrameworkErrorCode,
    SPEECH_ERROR_DOMAIN,
};
pub use language_model::{LanguageModelConfiguration, SpeechLanguageModel};
pub use live::{LiveRecognition, LiveUpdate};
pub use recognizer::{
    RecognitionMetadata, RecognitionResult, RecognitionWithMetadata, SpeechRecognizer,
    TranscriptionSegment,
};
pub use request::{
    AudioBufferRecognitionRequest, AudioCommonFormat, AudioFormat, CallbackQueue,
    RecognitionRequestOptions, TaskHint, UrlRecognitionRequest,
};
pub use task::{
    AudioBufferRecognitionTask, RecognitionTask, RecognitionTaskEvent,
    RecognizerAvailabilityObserver, TaskErrorInfo, TaskState,
};
pub use transcription::{
    AcousticFeature, DetailedRecognitionMetadata, DetailedRecognitionResult, TextRange,
    Transcription, TranscriptionSegmentDetails, VoiceAnalytics,
};

/// Common imports.
pub mod prelude {
    pub use crate::analyzer::{
        AnalysisContext, AnalyzerInput, AudioTimeRange, ContextualStringsTag,
        LocaleDependentSpeechModule, SpeechAnalyzer, SpeechAnalyzerModelRetention,
        SpeechAnalyzerModuleOutput, SpeechAnalyzerModuleResults, SpeechAnalyzerOptions,
        SpeechAnalyzerPriority, SpeechAttributedText, SpeechAttributeSpan, SpeechAttributes,
        SpeechConfidenceAttribute, SpeechDetectionOptions, SpeechDetectionResult,
        SpeechDetector, SpeechDetectorSensitivityLevel, SpeechModels, SpeechModule,
        SpeechModuleDescriptor, SpeechModuleResult, SpeechTimeRangeAttribute,
        SpeechTranscriber, SpeechTranscriberOptions, SpeechTranscriberPreset,
        SpeechTranscriberReportingOption, SpeechTranscriberResultAttributeOption,
        SpeechTranscriptionOption, SpeechTranscriptionResult, UserDataTag,
    };
    pub use crate::asset_inventory::{
        AssetInstallationProgress, AssetInstallationRequest, AssetInventory,
        AssetInventoryStatus,
    };
    pub use crate::custom_language_model::{
        CompoundTemplate, CustomPronunciation, DataInsertable, DataInsertableBuilder,
        PhraseCount, PhraseCountGenerator, PhraseCountGeneratorIterator,
        PhraseCountsFromTemplates, SFCustomLanguageModelData, TemplateInsertable,
        TemplateInsertableBuilder, TemplatePhraseCountGenerator,
        TemplatePhraseCountGeneratorIterator, TemplatePhraseCountGeneratorTemplate,
    };
    pub use crate::dictation::{
        DictationAudioTimeRange, DictationContentHint, DictationPreset, DictationReportingOption,
        DictationResultAttributeOption, DictationTranscriber, DictationTranscriberOptions,
        DictationTranscriptionOption, DictationTranscriptionResult,
    };
    pub use crate::error::{
        AuthorizationStatus, SpeechError, SpeechFrameworkError, SpeechFrameworkErrorCode,
        SPEECH_ERROR_DOMAIN,
    };
    pub use crate::language_model::{LanguageModelConfiguration, SpeechLanguageModel};
    pub use crate::live::{LiveRecognition, LiveUpdate};
    pub use crate::recognizer::{
        RecognitionMetadata, RecognitionResult, RecognitionWithMetadata, SpeechRecognizer,
        TranscriptionSegment,
    };
    pub use crate::request::{
        AudioBufferRecognitionRequest, AudioCommonFormat, AudioFormat, CallbackQueue,
        RecognitionRequestOptions, TaskHint, UrlRecognitionRequest,
    };
    pub use crate::task::{
        AudioBufferRecognitionTask, RecognitionTask, RecognitionTaskEvent,
        RecognizerAvailabilityObserver, TaskErrorInfo, TaskState,
    };
    pub use crate::transcription::{
        AcousticFeature, DetailedRecognitionMetadata, DetailedRecognitionResult, TextRange,
        Transcription, TranscriptionSegmentDetails, VoiceAnalytics,
    };
}
