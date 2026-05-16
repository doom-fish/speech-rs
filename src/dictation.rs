#![allow(clippy::missing_const_for_fn, clippy::missing_errors_doc)]

use core::ptr;
use std::ffi::CString;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::SpeechError;
use crate::ffi;
use crate::language_model::{LanguageModelConfiguration, LanguageModelConfigurationPayload};
use crate::private::{
    cstring_from_path, cstring_from_str, error_from_status, json_cstring, parse_json_ptr,
    take_string,
};
use crate::request::{AudioFormat, AudioFormatPayload};

/// Predefined `DictationTranscriber` configurations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc(alias = "DictationExtension")]
pub enum DictationPreset {
    Phrase,
    ShortDictation,
    ProgressiveShortDictation,
    LongDictation,
    ProgressiveLongDictation,
    TimeIndexedLongDictation,
}

impl DictationPreset {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Phrase => "phrase",
            Self::ShortDictation => "shortDictation",
            Self::ProgressiveShortDictation => "progressiveShortDictation",
            Self::LongDictation => "longDictation",
            Self::ProgressiveLongDictation => "progressiveLongDictation",
            Self::TimeIndexedLongDictation => "timeIndexedLongDictation",
        }
    }
}

/// `DictationTranscriber.ContentHint`.
#[derive(Debug, Clone, PartialEq)]
pub enum DictationContentHint {
    ShortForm,
    FarField,
    AtypicalSpeech,
    CustomizedLanguageModel(LanguageModelConfiguration),
}

impl DictationContentHint {
    #[must_use]
    pub fn customized_language_model(configuration: LanguageModelConfiguration) -> Self {
        Self::CustomizedLanguageModel(configuration)
    }
}

/// `DictationTranscriber.TranscriptionOption`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DictationTranscriptionOption {
    Punctuation,
    Emoji,
    EtiquetteReplacements,
}

impl DictationTranscriptionOption {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Punctuation => "punctuation",
            Self::Emoji => "emoji",
            Self::EtiquetteReplacements => "etiquetteReplacements",
        }
    }
}

/// `DictationTranscriber.ReportingOption`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DictationReportingOption {
    VolatileResults,
    AlternativeTranscriptions,
    FrequentFinalization,
}

impl DictationReportingOption {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VolatileResults => "volatileResults",
            Self::AlternativeTranscriptions => "alternativeTranscriptions",
            Self::FrequentFinalization => "frequentFinalization",
        }
    }
}

/// `DictationTranscriber.ResultAttributeOption`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DictationResultAttributeOption {
    AudioTimeRange,
    TranscriptionConfidence,
}

impl DictationResultAttributeOption {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AudioTimeRange => "audioTimeRange",
            Self::TranscriptionConfidence => "transcriptionConfidence",
        }
    }
}

/// Explicit `DictationTranscriber` configuration.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DictationTranscriberOptions {
    content_hints: Vec<DictationContentHint>,
    transcription_options: Vec<DictationTranscriptionOption>,
    reporting_options: Vec<DictationReportingOption>,
    attribute_options: Vec<DictationResultAttributeOption>,
}

impl DictationTranscriberOptions {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn content_hints(&self) -> &[DictationContentHint] {
        &self.content_hints
    }

    #[must_use]
    pub fn with_content_hints<I>(mut self, content_hints: I) -> Self
    where
        I: IntoIterator<Item = DictationContentHint>,
    {
        self.content_hints = content_hints.into_iter().collect();
        self
    }

    pub fn set_content_hints<I>(&mut self, content_hints: I)
    where
        I: IntoIterator<Item = DictationContentHint>,
    {
        self.content_hints = content_hints.into_iter().collect();
    }

    #[must_use]
    pub fn transcription_options(&self) -> &[DictationTranscriptionOption] {
        &self.transcription_options
    }

    #[must_use]
    pub fn with_transcription_options<I>(mut self, transcription_options: I) -> Self
    where
        I: IntoIterator<Item = DictationTranscriptionOption>,
    {
        self.transcription_options = transcription_options.into_iter().collect();
        self
    }

    pub fn set_transcription_options<I>(&mut self, transcription_options: I)
    where
        I: IntoIterator<Item = DictationTranscriptionOption>,
    {
        self.transcription_options = transcription_options.into_iter().collect();
    }

    #[must_use]
    pub fn reporting_options(&self) -> &[DictationReportingOption] {
        &self.reporting_options
    }

    #[must_use]
    pub fn with_reporting_options<I>(mut self, reporting_options: I) -> Self
    where
        I: IntoIterator<Item = DictationReportingOption>,
    {
        self.reporting_options = reporting_options.into_iter().collect();
        self
    }

    pub fn set_reporting_options<I>(&mut self, reporting_options: I)
    where
        I: IntoIterator<Item = DictationReportingOption>,
    {
        self.reporting_options = reporting_options.into_iter().collect();
    }

    #[must_use]
    pub fn attribute_options(&self) -> &[DictationResultAttributeOption] {
        &self.attribute_options
    }

    #[must_use]
    pub fn with_attribute_options<I>(mut self, attribute_options: I) -> Self
    where
        I: IntoIterator<Item = DictationResultAttributeOption>,
    {
        self.attribute_options = attribute_options.into_iter().collect();
        self
    }

    pub fn set_attribute_options<I>(&mut self, attribute_options: I)
    where
        I: IntoIterator<Item = DictationResultAttributeOption>,
    {
        self.attribute_options = attribute_options.into_iter().collect();
    }
}

#[derive(Debug, Clone, PartialEq)]
enum DictationConfiguration {
    Preset(DictationPreset),
    Custom(DictationTranscriberOptions),
}

/// Safe wrapper around Speech.framework's macOS 26 `DictationTranscriber`.
#[derive(Debug, Clone, PartialEq)]
#[doc(alias = "DictationExtension")]
pub struct DictationTranscriber {
    locale_identifier: String,
    configuration: DictationConfiguration,
}

impl DictationTranscriber {
    #[must_use]
    pub fn new(locale_identifier: impl Into<String>, preset: DictationPreset) -> Self {
        Self {
            locale_identifier: locale_identifier.into(),
            configuration: DictationConfiguration::Preset(preset),
        }
    }

    #[must_use]
    pub fn with_options(
        locale_identifier: impl Into<String>,
        options: DictationTranscriberOptions,
    ) -> Self {
        Self {
            locale_identifier: locale_identifier.into(),
            configuration: DictationConfiguration::Custom(options),
        }
    }

    #[must_use]
    pub fn locale_identifier(&self) -> &str {
        &self.locale_identifier
    }

    #[must_use]
    pub fn preset(&self) -> Option<DictationPreset> {
        match self.configuration {
            DictationConfiguration::Preset(preset) => Some(preset),
            DictationConfiguration::Custom(_) => None,
        }
    }

    #[must_use]
    pub fn options(&self) -> Option<&DictationTranscriberOptions> {
        match &self.configuration {
            DictationConfiguration::Preset(_) => None,
            DictationConfiguration::Custom(options) => Some(options),
        }
    }

    pub fn supported_locales() -> Result<Vec<String>, SpeechError> {
        let mut json = ptr::null_mut();
        let mut err_msg = ptr::null_mut();
        let status = unsafe { ffi::sp_dictation_supported_locales_json(&mut json, &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status(status, err_msg) });
        }
        unsafe { parse_json_ptr::<Vec<String>>(json, "dictation supported locales") }
    }

    pub fn installed_locales() -> Result<Vec<String>, SpeechError> {
        let mut json = ptr::null_mut();
        let mut err_msg = ptr::null_mut();
        let status = unsafe { ffi::sp_dictation_installed_locales_json(&mut json, &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status(status, err_msg) });
        }
        unsafe { parse_json_ptr::<Vec<String>>(json, "dictation installed locales") }
    }

    pub fn supported_locale_equivalent_to(
        locale_identifier: &str,
    ) -> Result<Option<String>, SpeechError> {
        let locale_identifier = cstring_from_str(locale_identifier, "dictation locale identifier")?;
        let mut locale = ptr::null_mut();
        let mut err_msg = ptr::null_mut();
        let status = unsafe {
            ffi::sp_dictation_supported_locale_identifier(
                locale_identifier.as_ptr(),
                &mut locale,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status(status, err_msg) });
        }
        unsafe { Ok(take_string(locale)) }
    }

    pub fn selected_locales(&self) -> Result<Vec<String>, SpeechError> {
        let config_json = self.config_json()?;
        let mut json = ptr::null_mut();
        let mut err_msg = ptr::null_mut();
        let status = unsafe {
            ffi::sp_dictation_selected_locales_json(config_json.as_ptr(), &mut json, &mut err_msg)
        };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status(status, err_msg) });
        }
        unsafe { parse_json_ptr::<Vec<String>>(json, "dictation selected locales") }
    }

    pub fn available_compatible_audio_formats(&self) -> Result<Vec<AudioFormat>, SpeechError> {
        let config_json = self.config_json()?;
        let mut json = ptr::null_mut();
        let mut err_msg = ptr::null_mut();
        let status = unsafe {
            ffi::sp_dictation_available_audio_formats_json(
                config_json.as_ptr(),
                &mut json,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status(status, err_msg) });
        }
        let payload = unsafe {
            parse_json_ptr::<Vec<AudioFormatPayload>>(json, "dictation compatible audio formats")
        }?;
        Ok(payload.into_iter().map(AudioFormat::from).collect())
    }

    pub fn transcribe_in_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<DictationTranscriptionResult>, SpeechError> {
        let path = cstring_from_path(path.as_ref(), "dictation audio path")?;
        let config_json = self.config_json()?;
        let mut json = ptr::null_mut();
        let mut err_msg = ptr::null_mut();
        let status = unsafe {
            ffi::sp_dictation_transcribe_url_json(
                path.as_ptr(),
                config_json.as_ptr(),
                &mut json,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status(status, err_msg) });
        }
        unsafe {
            parse_json_ptr::<Vec<DictationTranscriptionResult>>(
                json,
                "dictation transcription results",
            )
        }
    }

    fn config_json(&self) -> Result<CString, SpeechError> {
        json_cstring(
            &DictationPayload::try_from(self)?,
            "dictation configuration",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationAudioTimeRange {
    pub start_seconds: f64,
    pub duration_seconds: f64,
}

/// One `DictationTranscriber.Result` collected from Speech.framework.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[doc(alias = "DictationExtensionTranscriptionResult")]
pub struct DictationTranscriptionResult {
    pub text: String,
    pub alternatives: Vec<String>,
    #[serde(rename = "range")]
    pub audio_time_range: DictationAudioTimeRange,
    pub results_finalization_time_seconds: f64,
    pub is_final: bool,
}

impl DictationTranscriptionResult {
    #[must_use]
    pub fn transcript(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DictationPayload {
    locale_identifier: String,
    preset: Option<String>,
    content_hints: Option<Vec<DictationContentHintPayload>>,
    transcription_options: Option<Vec<String>>,
    reporting_options: Option<Vec<String>>,
    attribute_options: Option<Vec<String>>,
}

impl TryFrom<&DictationTranscriber> for DictationPayload {
    type Error = SpeechError;

    fn try_from(value: &DictationTranscriber) -> Result<Self, Self::Error> {
        let (preset, content_hints, transcription_options, reporting_options, attribute_options) =
            match &value.configuration {
                DictationConfiguration::Preset(preset) => {
                    (Some((*preset).as_str().to_owned()), None, None, None, None)
                }
                DictationConfiguration::Custom(options) => (
                    None,
                    (!options.content_hints.is_empty())
                        .then(|| {
                            options
                                .content_hints
                                .iter()
                                .map(DictationContentHintPayload::try_from)
                                .collect()
                        })
                        .transpose()?,
                    (!options.transcription_options.is_empty()).then(|| {
                        options
                            .transcription_options
                            .iter()
                            .copied()
                            .map(DictationTranscriptionOption::as_str)
                            .map(str::to_owned)
                            .collect()
                    }),
                    (!options.reporting_options.is_empty()).then(|| {
                        options
                            .reporting_options
                            .iter()
                            .copied()
                            .map(DictationReportingOption::as_str)
                            .map(str::to_owned)
                            .collect()
                    }),
                    (!options.attribute_options.is_empty()).then(|| {
                        options
                            .attribute_options
                            .iter()
                            .copied()
                            .map(DictationResultAttributeOption::as_str)
                            .map(str::to_owned)
                            .collect()
                    }),
                ),
            };

        Ok(Self {
            locale_identifier: value.locale_identifier.clone(),
            preset,
            content_hints,
            transcription_options,
            reporting_options,
            attribute_options,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DictationContentHintPayload {
    kind: String,
    language_model: Option<LanguageModelConfigurationPayload>,
}

impl TryFrom<&DictationContentHint> for DictationContentHintPayload {
    type Error = SpeechError;

    fn try_from(value: &DictationContentHint) -> Result<Self, Self::Error> {
        match value {
            DictationContentHint::ShortForm => Ok(Self {
                kind: "shortForm".into(),
                language_model: None,
            }),
            DictationContentHint::FarField => Ok(Self {
                kind: "farField".into(),
                language_model: None,
            }),
            DictationContentHint::AtypicalSpeech => Ok(Self {
                kind: "atypicalSpeech".into(),
                language_model: None,
            }),
            DictationContentHint::CustomizedLanguageModel(configuration) => Ok(Self {
                kind: "customizedLanguage".into(),
                language_model: Some(configuration.to_payload()?),
            }),
        }
    }
}
