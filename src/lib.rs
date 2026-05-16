#![doc = include_str!("../README.md")]
//!
//! ---
//!
//! # API Documentation
//!
//! Safe Rust bindings for Apple's
//! [Speech](https://developer.apple.com/documentation/speech) framework.
//!
//! This crate now covers all public `Speech.framework` classes on macOS,
//! including URL and audio-buffer requests, delegate-driven task events,
//! detailed transcription metadata, voice analytics, and custom language
//! model preparation.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod error;
pub mod ffi;
pub mod language_model;
mod private;
pub mod recognizer;
pub mod request;
pub mod task;
pub mod transcription;

pub mod live;

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
