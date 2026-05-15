#![doc = include_str!("../README.md")]
//!
//! ---
//!
//! # API Documentation
//!
//! Safe Rust bindings for Apple's
//! [Speech](https://developer.apple.com/documentation/speech) framework
//! — on-device speech recognition (`SFSpeechRecognizer`) on macOS.
//!
//! v0.1 ships file-based transcription via `SpeechRecognizer::recognize_in_path`.
//! Live audio-buffer streaming and partial-result callbacks land in v0.2.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod error;
pub mod ffi;

#[cfg(feature = "recognize_url")]
#[cfg_attr(docsrs, doc(cfg(feature = "recognize_url")))]
pub mod recognizer;

pub use error::{AuthorizationStatus, SpeechError};

#[cfg(feature = "recognize_url")]
pub use recognizer::{RecognitionResult, SpeechRecognizer, TranscriptionSegment};

/// Common imports.
pub mod prelude {
    pub use crate::error::{AuthorizationStatus, SpeechError};
    #[cfg(feature = "recognize_url")]
    pub use crate::recognizer::{RecognitionResult, SpeechRecognizer, TranscriptionSegment};
}
