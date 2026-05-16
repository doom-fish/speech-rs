#![allow(clippy::missing_const_for_fn, clippy::missing_errors_doc)]

use std::ffi::CString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::SpeechError;
use crate::ffi;
use crate::language_model::{LanguageModelConfiguration, LanguageModelConfigurationPayload};
use crate::private::{json_cstring, parse_json_ptr};

/// The type of recognition task being performed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TaskHint {
    #[default]
    Unspecified,
    Dictation,
    Search,
    Confirmation,
}

impl TaskHint {
    pub(crate) const fn as_raw(self) -> i32 {
        match self {
            Self::Unspecified => 0,
            Self::Dictation => 1,
            Self::Search => 2,
            Self::Confirmation => 3,
        }
    }

    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::Dictation,
            2 => Self::Search,
            3 => Self::Confirmation,
            _ => Self::Unspecified,
        }
    }
}

/// Controls which `NSOperationQueue` Apple's callbacks run on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum CallbackQueue {
    #[default]
    Main,
    Background {
        name: Option<String>,
        max_concurrent_operation_count: Option<usize>,
    },
}

impl CallbackQueue {
    #[must_use]
    pub fn background() -> Self {
        Self::Background {
            name: None,
            max_concurrent_operation_count: None,
        }
    }

    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        Self::Background {
            name: Some(name.into()),
            max_concurrent_operation_count: None,
        }
    }

    #[must_use]
    pub fn with_max_concurrent_operations(self, max_concurrent_operation_count: usize) -> Self {
        match self {
            Self::Main => Self::Background {
                name: None,
                max_concurrent_operation_count: Some(max_concurrent_operation_count),
            },
            Self::Background { name, .. } => Self::Background {
                name,
                max_concurrent_operation_count: Some(max_concurrent_operation_count),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueuePayload {
    kind: String,
    name: Option<String>,
    max_concurrent_operation_count: Option<usize>,
}

impl From<&CallbackQueue> for QueuePayload {
    fn from(value: &CallbackQueue) -> Self {
        match value {
            CallbackQueue::Main => Self {
                kind: "main".into(),
                name: None,
                max_concurrent_operation_count: None,
            },
            CallbackQueue::Background {
                name,
                max_concurrent_operation_count,
            } => Self {
                kind: "background".into(),
                name: name.clone(),
                max_concurrent_operation_count: *max_concurrent_operation_count,
            },
        }
    }
}

/// Configuration shared by URL and audio-buffer recognition requests.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RecognitionRequestOptions {
    task_hint: Option<TaskHint>,
    should_report_partial_results: Option<bool>,
    contextual_strings: Vec<String>,
    interaction_identifier: Option<String>,
    requires_on_device_recognition: Option<bool>,
    adds_punctuation: Option<bool>,
    customized_language_model: Option<LanguageModelConfiguration>,
}

impl RecognitionRequestOptions {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn task_hint(&self) -> Option<TaskHint> {
        self.task_hint
    }

    #[must_use]
    pub fn with_task_hint(mut self, task_hint: TaskHint) -> Self {
        self.task_hint = Some(task_hint);
        self
    }

    pub fn set_task_hint(&mut self, task_hint: TaskHint) {
        self.task_hint = Some(task_hint);
    }

    #[must_use]
    pub const fn should_report_partial_results(&self) -> Option<bool> {
        self.should_report_partial_results
    }

    #[must_use]
    pub fn with_should_report_partial_results(
        mut self,
        should_report_partial_results: bool,
    ) -> Self {
        self.should_report_partial_results = Some(should_report_partial_results);
        self
    }

    pub fn set_should_report_partial_results(&mut self, should_report_partial_results: bool) {
        self.should_report_partial_results = Some(should_report_partial_results);
    }

    #[must_use]
    pub fn contextual_strings(&self) -> &[String] {
        &self.contextual_strings
    }

    #[must_use]
    pub fn with_contextual_strings<I, S>(mut self, contextual_strings: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.contextual_strings = contextual_strings.into_iter().map(Into::into).collect();
        self
    }

    pub fn set_contextual_strings<I, S>(&mut self, contextual_strings: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.contextual_strings = contextual_strings.into_iter().map(Into::into).collect();
    }

    #[must_use]
    pub fn interaction_identifier(&self) -> Option<&str> {
        self.interaction_identifier.as_deref()
    }

    #[must_use]
    pub fn with_interaction_identifier(
        mut self,
        interaction_identifier: impl Into<String>,
    ) -> Self {
        self.interaction_identifier = Some(interaction_identifier.into());
        self
    }

    pub fn set_interaction_identifier(&mut self, interaction_identifier: impl Into<String>) {
        self.interaction_identifier = Some(interaction_identifier.into());
    }

    #[must_use]
    pub const fn requires_on_device_recognition(&self) -> Option<bool> {
        self.requires_on_device_recognition
    }

    #[must_use]
    pub fn with_requires_on_device_recognition(
        mut self,
        requires_on_device_recognition: bool,
    ) -> Self {
        self.requires_on_device_recognition = Some(requires_on_device_recognition);
        self
    }

    pub fn set_requires_on_device_recognition(&mut self, requires_on_device_recognition: bool) {
        self.requires_on_device_recognition = Some(requires_on_device_recognition);
    }

    #[must_use]
    pub const fn adds_punctuation(&self) -> Option<bool> {
        self.adds_punctuation
    }

    #[must_use]
    pub fn with_adds_punctuation(mut self, adds_punctuation: bool) -> Self {
        self.adds_punctuation = Some(adds_punctuation);
        self
    }

    pub fn set_adds_punctuation(&mut self, adds_punctuation: bool) {
        self.adds_punctuation = Some(adds_punctuation);
    }

    #[must_use]
    pub fn customized_language_model(&self) -> Option<&LanguageModelConfiguration> {
        self.customized_language_model.as_ref()
    }

    #[must_use]
    pub fn with_customized_language_model(
        mut self,
        customized_language_model: LanguageModelConfiguration,
    ) -> Self {
        self.customized_language_model = Some(customized_language_model);
        self
    }

    pub fn set_customized_language_model(
        &mut self,
        customized_language_model: LanguageModelConfiguration,
    ) {
        self.customized_language_model = Some(customized_language_model);
    }

    pub(crate) fn to_json_cstring(&self) -> Result<CString, SpeechError> {
        json_cstring(
            &RequestPayload::try_from(self)?,
            "recognition request options",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestPayload {
    task_hint: Option<i32>,
    should_report_partial_results: Option<bool>,
    contextual_strings: Option<Vec<String>>,
    interaction_identifier: Option<String>,
    requires_on_device_recognition: Option<bool>,
    adds_punctuation: Option<bool>,
    customized_language_model: Option<LanguageModelConfigurationPayload>,
}

impl TryFrom<&RecognitionRequestOptions> for RequestPayload {
    type Error = SpeechError;

    fn try_from(value: &RecognitionRequestOptions) -> Result<Self, Self::Error> {
        Ok(Self {
            task_hint: value.task_hint.map(TaskHint::as_raw),
            should_report_partial_results: value.should_report_partial_results,
            contextual_strings: (!value.contextual_strings.is_empty())
                .then(|| value.contextual_strings.clone()),
            interaction_identifier: value.interaction_identifier.clone(),
            requires_on_device_recognition: value.requires_on_device_recognition,
            adds_punctuation: value.adds_punctuation,
            customized_language_model: value
                .customized_language_model
                .as_ref()
                .map(LanguageModelConfiguration::to_payload)
                .transpose()?,
        })
    }
}

/// A file-backed speech recognition request.
#[derive(Debug, Clone, PartialEq)]
pub struct UrlRecognitionRequest {
    path: PathBuf,
    options: RecognitionRequestOptions,
}

impl UrlRecognitionRequest {
    #[must_use]
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            options: RecognitionRequestOptions::default(),
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn url(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn options(&self) -> &RecognitionRequestOptions {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut RecognitionRequestOptions {
        &mut self.options
    }

    #[must_use]
    pub fn with_options(mut self, options: RecognitionRequestOptions) -> Self {
        self.options = options;
        self
    }
}

/// A live or manually-fed audio-buffer recognition request.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioBufferRecognitionRequest {
    options: RecognitionRequestOptions,
}

impl AudioBufferRecognitionRequest {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn options(&self) -> &RecognitionRequestOptions {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut RecognitionRequestOptions {
        &mut self.options
    }

    #[must_use]
    pub fn with_options(mut self, options: RecognitionRequestOptions) -> Self {
        self.options = options;
        self
    }

    /// Apple's preferred audio format for `SFSpeechAudioBufferRecognitionRequest`.
    pub fn native_audio_format(&self) -> Result<AudioFormat, SpeechError> {
        let json = unsafe { ffi::sp_audio_buffer_request_native_format_json() };
        unsafe { parse_json_ptr::<AudioFormatPayload>(json, "native audio format") }
            .map(AudioFormat::from)
    }
}

/// The PCM sample format represented by an `AVAudioFormat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioCommonFormat {
    Other,
    Float32,
    Float64,
    Int16,
    Int32,
    Unknown(i32),
}

impl AudioCommonFormat {
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Other,
            1 => Self::Float32,
            2 => Self::Float64,
            3 => Self::Int16,
            4 => Self::Int32,
            other => Self::Unknown(other),
        }
    }
}

/// Audio format information returned by `SFSpeechAudioBufferRecognitionRequest.nativeAudioFormat`.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioFormat {
    pub sample_rate: f64,
    pub channel_count: usize,
    pub is_interleaved: bool,
    pub common_format: AudioCommonFormat,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AudioFormatPayload {
    sample_rate: f64,
    channel_count: usize,
    is_interleaved: bool,
    common_format: i32,
}

impl From<AudioFormatPayload> for AudioFormat {
    fn from(value: AudioFormatPayload) -> Self {
        Self {
            sample_rate: value.sample_rate,
            channel_count: value.channel_count,
            is_interleaved: value.is_interleaved,
            common_format: AudioCommonFormat::from_raw(value.common_format),
        }
    }
}
