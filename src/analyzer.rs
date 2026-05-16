#![allow(
    clippy::missing_const_for_fn,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::struct_field_names
)]

use core::ffi::c_void;
use std::collections::BTreeMap;
use std::ffi::CString;
use std::ops::Range;
use std::path::Path;
use std::ptr::{self, NonNull};

use serde::{Deserialize, Serialize};

use crate::error::SpeechError;
use crate::ffi;
use crate::private::{
    cstring_from_path, cstring_from_str, error_from_status_or_json, json_cstring,
    parse_json_ptr, take_string,
};
use crate::request::{AudioFormat, AudioFormatPayload};

/// A time range in analyzed audio.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTimeRange {
    pub start_seconds: f64,
    pub duration_seconds: f64,
}

impl AudioTimeRange {
    #[must_use]
    pub const fn new(start_seconds: f64, duration_seconds: f64) -> Self {
        Self {
            start_seconds,
            duration_seconds,
        }
    }

    #[must_use]
    pub const fn end_seconds(self) -> f64 {
        self.start_seconds + self.duration_seconds
    }

    #[must_use]
    pub fn intersects(self, other: Self) -> bool {
        self.start_seconds < other.end_seconds() && other.start_seconds < self.end_seconds()
    }
}

/// Namespace for Speech attributed-text metadata.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SpeechAttributes;

/// Confidence attached to a span in `SpeechTranscriber.Result.text`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SpeechConfidenceAttribute(pub f64);

impl SpeechConfidenceAttribute {
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0
    }
}

/// Audio time range attached to a span in `SpeechTranscriber.Result.text`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SpeechTimeRangeAttribute(pub AudioTimeRange);

impl SpeechTimeRangeAttribute {
    #[must_use]
    pub const fn value(self) -> AudioTimeRange {
        self.0
    }
}

/// One span of Speech attributes inside a flattened transcript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechAttributeSpan {
    pub start: usize,
    pub end: usize,
    pub transcription_confidence: Option<SpeechConfidenceAttribute>,
    pub audio_time_range: Option<SpeechTimeRangeAttribute>,
}

impl SpeechAttributeSpan {
    #[must_use]
    pub fn byte_range(&self) -> Range<usize> {
        self.start..self.end
    }

    #[must_use]
    pub fn intersects_audio_time_range(&self, time_range: AudioTimeRange) -> bool {
        self.audio_time_range
            .map(SpeechTimeRangeAttribute::value)
            .is_some_and(|value| value.intersects(time_range))
    }
}

/// Flattened `AttributedString` text plus any Speech-specific attributes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechAttributedText {
    pub text: String,
    #[serde(default)]
    pub spans: Vec<SpeechAttributeSpan>,
}

impl SpeechAttributedText {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn range_of_audio_time_range_attributes_intersecting(
        &self,
        time_range: AudioTimeRange,
    ) -> Option<Range<usize>> {
        let mut matching = self
            .spans
            .iter()
            .filter(|span| span.intersects_audio_time_range(time_range))
            .map(SpeechAttributeSpan::byte_range);
        let first = matching.next()?;
        let start = first.start;
        let mut end = first.end;
        for range in matching {
            end = end.max(range.end);
        }
        Some(start..end)
    }
}

/// Shared behavior for analyzer-module results.
pub trait SpeechModuleResult {
    fn audio_time_range(&self) -> AudioTimeRange;
    fn results_finalization_time_seconds(&self) -> f64;

    #[must_use]
    fn is_final(&self) -> bool {
        let range = self.audio_time_range();
        range.end_seconds() <= self.results_finalization_time_seconds() + f64::EPSILON
    }
}

/// Shared behavior for analyzer-compatible modules.
pub trait SpeechModule {
    fn descriptor(&self) -> SpeechModuleDescriptor;
}

/// Shared locale helpers for locale-aware analyzer modules.
pub trait LocaleDependentSpeechModule: SpeechModule {
    fn selected_locales(&self) -> Result<Vec<String>, SpeechError>;

    fn supported_locales() -> Result<Vec<String>, SpeechError>
    where
        Self: Sized;

    fn supported_locale_equivalent_to(
        locale_identifier: &str,
    ) -> Result<Option<String>, SpeechError>
    where
        Self: Sized;
}

/// `SpeechTranscriber.Preset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpeechTranscriberPreset {
    Transcription,
    TranscriptionWithAlternatives,
    TimeIndexedTranscriptionWithAlternatives,
    ProgressiveTranscription,
    TimeIndexedProgressiveTranscription,
}

impl SpeechTranscriberPreset {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Transcription => "transcription",
            Self::TranscriptionWithAlternatives => "transcriptionWithAlternatives",
            Self::TimeIndexedTranscriptionWithAlternatives => {
                "timeIndexedTranscriptionWithAlternatives"
            }
            Self::ProgressiveTranscription => "progressiveTranscription",
            Self::TimeIndexedProgressiveTranscription => "timeIndexedProgressiveTranscription",
        }
    }
}

/// `SpeechTranscriber.TranscriptionOption`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpeechTranscriptionOption {
    EtiquetteReplacements,
}

impl SpeechTranscriptionOption {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EtiquetteReplacements => "etiquetteReplacements",
        }
    }
}

/// `SpeechTranscriber.ReportingOption`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpeechTranscriberReportingOption {
    VolatileResults,
    AlternativeTranscriptions,
    FastResults,
}

impl SpeechTranscriberReportingOption {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VolatileResults => "volatileResults",
            Self::AlternativeTranscriptions => "alternativeTranscriptions",
            Self::FastResults => "fastResults",
        }
    }
}

/// `SpeechTranscriber.ResultAttributeOption`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpeechTranscriberResultAttributeOption {
    AudioTimeRange,
    TranscriptionConfidence,
}

impl SpeechTranscriberResultAttributeOption {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AudioTimeRange => "audioTimeRange",
            Self::TranscriptionConfidence => "transcriptionConfidence",
        }
    }
}

/// Explicit `SpeechTranscriber` configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpeechTranscriberOptions {
    transcription: Vec<SpeechTranscriptionOption>,
    reporting: Vec<SpeechTranscriberReportingOption>,
    attributes: Vec<SpeechTranscriberResultAttributeOption>,
}

impl SpeechTranscriberOptions {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn transcription_options(&self) -> &[SpeechTranscriptionOption] {
        &self.transcription
    }

    #[must_use]
    pub fn with_transcription_options<I>(mut self, transcription_options: I) -> Self
    where
        I: IntoIterator<Item = SpeechTranscriptionOption>,
    {
        self.transcription = transcription_options.into_iter().collect();
        self
    }

    pub fn set_transcription_options<I>(&mut self, transcription_options: I)
    where
        I: IntoIterator<Item = SpeechTranscriptionOption>,
    {
        self.transcription = transcription_options.into_iter().collect();
    }

    #[must_use]
    pub fn reporting_options(&self) -> &[SpeechTranscriberReportingOption] {
        &self.reporting
    }

    #[must_use]
    pub fn with_reporting_options<I>(mut self, reporting_options: I) -> Self
    where
        I: IntoIterator<Item = SpeechTranscriberReportingOption>,
    {
        self.reporting = reporting_options.into_iter().collect();
        self
    }

    pub fn set_reporting_options<I>(&mut self, reporting_options: I)
    where
        I: IntoIterator<Item = SpeechTranscriberReportingOption>,
    {
        self.reporting = reporting_options.into_iter().collect();
    }

    #[must_use]
    pub fn attribute_options(&self) -> &[SpeechTranscriberResultAttributeOption] {
        &self.attributes
    }

    #[must_use]
    pub fn with_attribute_options<I>(mut self, attribute_options: I) -> Self
    where
        I: IntoIterator<Item = SpeechTranscriberResultAttributeOption>,
    {
        self.attributes = attribute_options.into_iter().collect();
        self
    }

    pub fn set_attribute_options<I>(&mut self, attribute_options: I)
    where
        I: IntoIterator<Item = SpeechTranscriberResultAttributeOption>,
    {
        self.attributes = attribute_options.into_iter().collect();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SpeechTranscriberConfiguration {
    Preset(SpeechTranscriberPreset),
    Custom(SpeechTranscriberOptions),
}

/// Safe wrapper around Speech.framework's analyzer-based `SpeechTranscriber`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeechTranscriber {
    locale_identifier: String,
    configuration: SpeechTranscriberConfiguration,
}

impl SpeechTranscriber {
    #[must_use]
    pub fn new(locale_identifier: impl Into<String>, preset: SpeechTranscriberPreset) -> Self {
        Self {
            locale_identifier: locale_identifier.into(),
            configuration: SpeechTranscriberConfiguration::Preset(preset),
        }
    }

    #[must_use]
    pub fn with_options(
        locale_identifier: impl Into<String>,
        options: SpeechTranscriberOptions,
    ) -> Self {
        Self {
            locale_identifier: locale_identifier.into(),
            configuration: SpeechTranscriberConfiguration::Custom(options),
        }
    }

    #[must_use]
    pub fn locale_identifier(&self) -> &str {
        &self.locale_identifier
    }

    #[must_use]
    pub fn preset(&self) -> Option<SpeechTranscriberPreset> {
        match self.configuration {
            SpeechTranscriberConfiguration::Preset(preset) => Some(preset),
            SpeechTranscriberConfiguration::Custom(_) => None,
        }
    }

    #[must_use]
    pub fn options(&self) -> Option<&SpeechTranscriberOptions> {
        match &self.configuration {
            SpeechTranscriberConfiguration::Preset(_) => None,
            SpeechTranscriberConfiguration::Custom(options) => Some(options),
        }
    }

    #[must_use]
    pub fn is_available() -> bool {
        unsafe { ffi::sp_speech_transcriber_is_available() }
    }

    pub fn supported_locales() -> Result<Vec<String>, SpeechError> {
        let mut json = ptr::null_mut();
        let mut err_msg = ptr::null_mut();
        let status = unsafe { ffi::sp_speech_transcriber_supported_locales_json(&mut json, &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status_or_json(status, err_msg) });
        }
        unsafe { parse_json_ptr::<Vec<String>>(json, "speech transcriber supported locales") }
    }

    pub fn installed_locales() -> Result<Vec<String>, SpeechError> {
        let mut json = ptr::null_mut();
        let mut err_msg = ptr::null_mut();
        let status = unsafe { ffi::sp_speech_transcriber_installed_locales_json(&mut json, &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status_or_json(status, err_msg) });
        }
        unsafe { parse_json_ptr::<Vec<String>>(json, "speech transcriber installed locales") }
    }

    pub fn supported_locale_equivalent_to(
        locale_identifier: &str,
    ) -> Result<Option<String>, SpeechError> {
        let locale_identifier =
            cstring_from_str(locale_identifier, "speech transcriber locale identifier")?;
        let mut locale = ptr::null_mut();
        let mut err_msg = ptr::null_mut();
        let status = unsafe {
            ffi::sp_speech_transcriber_supported_locale_identifier(
                locale_identifier.as_ptr(),
                &mut locale,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status_or_json(status, err_msg) });
        }
        unsafe { Ok(take_string(locale)) }
    }

    pub fn selected_locales(&self) -> Result<Vec<String>, SpeechError> {
        let config_json = self.config_json()?;
        let mut json = ptr::null_mut();
        let mut err_msg = ptr::null_mut();
        let status = unsafe {
            ffi::sp_speech_transcriber_selected_locales_json(
                config_json.as_ptr(),
                &mut json,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status_or_json(status, err_msg) });
        }
        unsafe { parse_json_ptr::<Vec<String>>(json, "speech transcriber selected locales") }
    }

    pub fn available_compatible_audio_formats(&self) -> Result<Vec<AudioFormat>, SpeechError> {
        let config_json = self.config_json()?;
        let mut json = ptr::null_mut();
        let mut err_msg = ptr::null_mut();
        let status = unsafe {
            ffi::sp_speech_transcriber_available_audio_formats_json(
                config_json.as_ptr(),
                &mut json,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status_or_json(status, err_msg) });
        }
        let payload = unsafe {
            parse_json_ptr::<Vec<AudioFormatPayload>>(
                json,
                "speech transcriber compatible audio formats",
            )
        }?;
        Ok(payload.into_iter().map(AudioFormat::from).collect())
    }

    pub fn transcribe_in_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<SpeechTranscriptionResult>, SpeechError> {
        self.transcribe_in_path_with_context(path, &AnalysisContext::default(), None)
    }

    pub fn transcribe_in_path_with_context(
        &self,
        path: impl AsRef<Path>,
        context: &AnalysisContext,
        options: Option<SpeechAnalyzerOptions>,
    ) -> Result<Vec<SpeechTranscriptionResult>, SpeechError> {
        let mut analyzer = SpeechAnalyzer::new([self.clone()]);
        analyzer.set_context(context.clone());
        if let Some(options) = options {
            analyzer.set_options(options);
        }
        let output = analyzer.analyze_in_path(path)?;
        let Some(module_output) = output.modules.first() else {
            return Err(SpeechError::RecognitionFailed(
                "speech analyzer produced no transcriber output".into(),
            ));
        };
        match &module_output.results {
            SpeechAnalyzerModuleResults::SpeechTranscriber(results) => Ok(results.clone()),
            SpeechAnalyzerModuleResults::SpeechDetector(_) => Err(SpeechError::RecognitionFailed(
                "speech analyzer returned detector results for a speech transcriber".into(),
            )),
        }
    }

    fn config_json(&self) -> Result<CString, SpeechError> {
        json_cstring(
            &SpeechTranscriberPayload::from(self),
            "speech transcriber configuration",
        )
    }
}

impl SpeechModule for SpeechTranscriber {
    fn descriptor(&self) -> SpeechModuleDescriptor {
        SpeechModuleDescriptor::SpeechTranscriber(self.clone())
    }
}

impl LocaleDependentSpeechModule for SpeechTranscriber {
    fn selected_locales(&self) -> Result<Vec<String>, SpeechError> {
        Self::selected_locales(self)
    }

    fn supported_locales() -> Result<Vec<String>, SpeechError> {
        Self::supported_locales()
    }

    fn supported_locale_equivalent_to(
        locale_identifier: &str,
    ) -> Result<Option<String>, SpeechError> {
        Self::supported_locale_equivalent_to(locale_identifier)
    }
}

impl From<SpeechTranscriber> for SpeechModuleDescriptor {
    fn from(value: SpeechTranscriber) -> Self {
        Self::SpeechTranscriber(value)
    }
}

impl From<&SpeechTranscriber> for SpeechModuleDescriptor {
    fn from(value: &SpeechTranscriber) -> Self {
        Self::SpeechTranscriber(value.clone())
    }
}

/// One `SpeechTranscriber.Result` collected from `SpeechAnalyzer`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechTranscriptionResult {
    #[serde(rename = "range")]
    pub audio_time_range: AudioTimeRange,
    pub results_finalization_time_seconds: f64,
    pub text: SpeechAttributedText,
    #[serde(default)]
    pub alternatives: Vec<SpeechAttributedText>,
    pub is_final: bool,
}

impl SpeechTranscriptionResult {
    #[must_use]
    pub fn transcript(&self) -> &str {
        self.text.as_str()
    }
}

impl SpeechModuleResult for SpeechTranscriptionResult {
    fn audio_time_range(&self) -> AudioTimeRange {
        self.audio_time_range
    }

    fn results_finalization_time_seconds(&self) -> f64 {
        self.results_finalization_time_seconds
    }

    fn is_final(&self) -> bool {
        self.is_final
    }
}

/// `SpeechDetector.SensitivityLevel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpeechDetectorSensitivityLevel {
    Low,
    Medium,
    High,
}

impl SpeechDetectorSensitivityLevel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

/// `SpeechDetector.DetectionOptions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpeechDetectionOptions {
    sensitivity_level: SpeechDetectorSensitivityLevel,
}

impl SpeechDetectionOptions {
    #[must_use]
    pub const fn new(sensitivity_level: SpeechDetectorSensitivityLevel) -> Self {
        Self { sensitivity_level }
    }

    #[must_use]
    pub const fn sensitivity_level(self) -> SpeechDetectorSensitivityLevel {
        self.sensitivity_level
    }
}

impl Default for SpeechDetectionOptions {
    fn default() -> Self {
        Self::new(SpeechDetectorSensitivityLevel::Medium)
    }
}

/// Safe wrapper around the analyzer-compatible `SpeechDetector` module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeechDetector {
    detection_options: SpeechDetectionOptions,
    report_results: bool,
}

impl SpeechDetector {
    #[must_use]
    pub const fn new(detection_options: SpeechDetectionOptions, report_results: bool) -> Self {
        Self {
            detection_options,
            report_results,
        }
    }

    #[must_use]
    pub const fn detection_options(&self) -> SpeechDetectionOptions {
        self.detection_options
    }

    #[must_use]
    pub const fn report_results(&self) -> bool {
        self.report_results
    }

    pub fn available_compatible_audio_formats(&self) -> Result<Vec<AudioFormat>, SpeechError> {
        let config_json = self.config_json()?;
        let mut json = ptr::null_mut();
        let mut err_msg = ptr::null_mut();
        let status = unsafe {
            ffi::sp_speech_detector_available_audio_formats_json(
                config_json.as_ptr(),
                &mut json,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status_or_json(status, err_msg) });
        }
        let payload = unsafe {
            parse_json_ptr::<Vec<AudioFormatPayload>>(
                json,
                "speech detector compatible audio formats",
            )
        }?;
        Ok(payload.into_iter().map(AudioFormat::from).collect())
    }

    pub fn detect_in_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<SpeechDetectionResult>, SpeechError> {
        self.detect_in_path_with_context(path, &AnalysisContext::default(), None)
    }

    pub fn detect_in_path_with_context(
        &self,
        path: impl AsRef<Path>,
        context: &AnalysisContext,
        options: Option<SpeechAnalyzerOptions>,
    ) -> Result<Vec<SpeechDetectionResult>, SpeechError> {
        let mut analyzer = SpeechAnalyzer::new([self.clone()]);
        analyzer.set_context(context.clone());
        if let Some(options) = options {
            analyzer.set_options(options);
        }
        let output = analyzer.analyze_in_path(path)?;
        let Some(module_output) = output.modules.first() else {
            return Err(SpeechError::RecognitionFailed(
                "speech analyzer produced no detector output".into(),
            ));
        };
        match &module_output.results {
            SpeechAnalyzerModuleResults::SpeechDetector(results) => Ok(results.clone()),
            SpeechAnalyzerModuleResults::SpeechTranscriber(_) => Err(SpeechError::RecognitionFailed(
                "speech analyzer returned transcriber results for a speech detector".into(),
            )),
        }
    }

    fn config_json(&self) -> Result<CString, SpeechError> {
        json_cstring(&SpeechDetectorPayload::from(self), "speech detector configuration")
    }
}

impl Default for SpeechDetector {
    fn default() -> Self {
        Self::new(SpeechDetectionOptions::default(), true)
    }
}

impl SpeechModule for SpeechDetector {
    fn descriptor(&self) -> SpeechModuleDescriptor {
        SpeechModuleDescriptor::SpeechDetector(self.clone())
    }
}

impl From<SpeechDetector> for SpeechModuleDescriptor {
    fn from(value: SpeechDetector) -> Self {
        Self::SpeechDetector(value)
    }
}

impl From<&SpeechDetector> for SpeechModuleDescriptor {
    fn from(value: &SpeechDetector) -> Self {
        Self::SpeechDetector(value.clone())
    }
}

/// One `SpeechDetector.Result` collected from `SpeechAnalyzer`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechDetectionResult {
    #[serde(rename = "range")]
    pub audio_time_range: AudioTimeRange,
    pub results_finalization_time_seconds: f64,
    pub speech_detected: bool,
    pub is_final: bool,
}

impl SpeechModuleResult for SpeechDetectionResult {
    fn audio_time_range(&self) -> AudioTimeRange {
        self.audio_time_range
    }

    fn results_finalization_time_seconds(&self) -> f64 {
        self.results_finalization_time_seconds
    }

    fn is_final(&self) -> bool {
        self.is_final
    }
}

/// `SpeechAnalyzer.Options` priority mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpeechAnalyzerPriority {
    Background,
    Utility,
    Low,
    Medium,
    High,
    UserInitiated,
}

impl SpeechAnalyzerPriority {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Utility => "utility",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::UserInitiated => "userInitiated",
        }
    }
}

/// `SpeechAnalyzer.Options.ModelRetention`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpeechAnalyzerModelRetention {
    WhileInUse,
    Lingering,
    ProcessLifetime,
}

impl SpeechAnalyzerModelRetention {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WhileInUse => "whileInUse",
            Self::Lingering => "lingering",
            Self::ProcessLifetime => "processLifetime",
        }
    }
}

/// `SpeechAnalyzer.Options`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpeechAnalyzerOptions {
    priority: SpeechAnalyzerPriority,
    model_retention: SpeechAnalyzerModelRetention,
}

impl SpeechAnalyzerOptions {
    #[must_use]
    pub const fn new(
        priority: SpeechAnalyzerPriority,
        model_retention: SpeechAnalyzerModelRetention,
    ) -> Self {
        Self {
            priority,
            model_retention,
        }
    }

    #[must_use]
    pub const fn priority(self) -> SpeechAnalyzerPriority {
        self.priority
    }

    #[must_use]
    pub const fn model_retention(self) -> SpeechAnalyzerModelRetention {
        self.model_retention
    }
}

impl Default for SpeechAnalyzerOptions {
    fn default() -> Self {
        Self::new(
            SpeechAnalyzerPriority::Medium,
            SpeechAnalyzerModelRetention::WhileInUse,
        )
    }
}

/// `AnalysisContext.ContextualStringsTag`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContextualStringsTag(String);

impl ContextualStringsTag {
    #[must_use]
    pub fn new(raw_value: impl Into<String>) -> Self {
        Self(raw_value.into())
    }

    #[must_use]
    pub fn general() -> Self {
        Self::new("general")
    }

    #[must_use]
    pub fn raw_value(&self) -> &str {
        &self.0
    }
}

/// `AnalysisContext.UserDataTag`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserDataTag(String);

impl UserDataTag {
    #[must_use]
    pub fn new(raw_value: impl Into<String>) -> Self {
        Self(raw_value.into())
    }

    #[must_use]
    pub fn raw_value(&self) -> &str {
        &self.0
    }
}

/// Analyzer context for contextual strings and string-backed user-data values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnalysisContext {
    contextual_strings: BTreeMap<ContextualStringsTag, Vec<String>>,
    user_data: BTreeMap<UserDataTag, String>,
}

impl AnalysisContext {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn contextual_strings(&self) -> &BTreeMap<ContextualStringsTag, Vec<String>> {
        &self.contextual_strings
    }

    pub fn set_contextual_strings<I, S>(&mut self, tag: ContextualStringsTag, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.contextual_strings
            .insert(tag, values.into_iter().map(Into::into).collect());
    }

    #[must_use]
    pub fn with_contextual_strings<I, S>(
        mut self,
        tag: ContextualStringsTag,
        values: I,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.set_contextual_strings(tag, values);
        self
    }

    #[must_use]
    pub fn user_data(&self) -> &BTreeMap<UserDataTag, String> {
        &self.user_data
    }

    pub fn set_user_data(&mut self, tag: UserDataTag, value: impl Into<String>) {
        self.user_data.insert(tag, value.into());
    }

    #[must_use]
    pub fn with_user_data(mut self, tag: UserDataTag, value: impl Into<String>) -> Self {
        self.set_user_data(tag, value);
        self
    }
}

/// A module descriptor accepted by `SpeechAnalyzer` and `AssetInventory`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeechModuleDescriptor {
    SpeechTranscriber(SpeechTranscriber),
    SpeechDetector(SpeechDetector),
}

impl SpeechModuleDescriptor {
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::SpeechTranscriber(_) => "speechTranscriber",
            Self::SpeechDetector(_) => "speechDetector",
        }
    }
}

/// Safe wrapper around the analyzer pipeline introduced in macOS 26.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeechAnalyzer {
    modules: Vec<SpeechModuleDescriptor>,
    options: Option<SpeechAnalyzerOptions>,
    context: AnalysisContext,
}

impl SpeechAnalyzer {
    #[must_use]
    pub fn new<M, I>(modules: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Into<SpeechModuleDescriptor>,
    {
        Self {
            modules: modules.into_iter().map(Into::into).collect(),
            options: None,
            context: AnalysisContext::default(),
        }
    }

    #[must_use]
    pub fn modules(&self) -> &[SpeechModuleDescriptor] {
        &self.modules
    }

    pub fn set_modules<M, I>(&mut self, modules: I)
    where
        I: IntoIterator<Item = M>,
        M: Into<SpeechModuleDescriptor>,
    {
        self.modules = modules.into_iter().map(Into::into).collect();
    }

    #[must_use]
    pub const fn options(&self) -> Option<SpeechAnalyzerOptions> {
        self.options
    }

    #[must_use]
    pub fn with_options(mut self, options: SpeechAnalyzerOptions) -> Self {
        self.options = Some(options);
        self
    }

    pub fn set_options(&mut self, options: SpeechAnalyzerOptions) {
        self.options = Some(options);
    }

    #[must_use]
    pub fn context(&self) -> &AnalysisContext {
        &self.context
    }

    #[must_use]
    pub fn with_context(mut self, context: AnalysisContext) -> Self {
        self.context = context;
        self
    }

    pub fn set_context(&mut self, context: AnalysisContext) {
        self.context = context;
    }

    pub fn best_available_audio_format(&self) -> Result<Option<AudioFormat>, SpeechError> {
        let modules_json = modules_json_cstring(&self.modules, "speech analyzer modules")?;
        let mut json = ptr::null_mut();
        let mut err_msg = ptr::null_mut();
        let status = unsafe {
            ffi::sp_speech_analyzer_best_audio_format_json(
                modules_json.as_ptr(),
                &mut json,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status_or_json(status, err_msg) });
        }
        let payload = unsafe {
            parse_json_ptr::<Option<AudioFormatPayload>>(json, "speech analyzer best audio format")
        }?;
        Ok(payload.map(AudioFormat::from))
    }

    pub fn analyze_in_path(&self, path: impl AsRef<Path>) -> Result<SpeechAnalyzerOutput, SpeechError> {
        if self.modules.is_empty() {
            return Err(SpeechError::InvalidArgument(
                "speech analyzer requires at least one module".into(),
            ));
        }

        let audio_path = cstring_from_path(path.as_ref(), "speech analyzer audio path")?;
        let analyzer_json = json_cstring(
            &SpeechAnalyzerPayload::from(self),
            "speech analyzer configuration",
        )?;
        let mut json = ptr::null_mut();
        let mut err_msg = ptr::null_mut();
        let status = unsafe {
            ffi::sp_speech_analyzer_analyze_url_json(
                audio_path.as_ptr(),
                analyzer_json.as_ptr(),
                &mut json,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status_or_json(status, err_msg) });
        }

        let payload = unsafe {
            parse_json_ptr::<SpeechAnalyzerOutputPayload>(json, "speech analyzer output")
        }?;
        let modules = payload
            .modules
            .into_iter()
            .map(|module_payload| {
                let module = self.modules.get(module_payload.module_index).cloned().ok_or_else(|| {
                    SpeechError::InvalidArgument(format!(
                        "speech analyzer output referenced unknown module index {}",
                        module_payload.module_index
                    ))
                })?;
                let results = match module_payload.kind.as_str() {
                    "speechTranscriber" => SpeechAnalyzerModuleResults::SpeechTranscriber(
                        module_payload.transcriber_results.unwrap_or_default(),
                    ),
                    "speechDetector" => {
                        SpeechAnalyzerModuleResults::SpeechDetector(
                            module_payload.detector_results.unwrap_or_default(),
                        )
                    }
                    other => {
                        return Err(SpeechError::InvalidArgument(format!(
                            "speech analyzer returned unsupported module kind {other}"
                        )))
                    }
                };
                Ok(SpeechAnalyzerModuleOutput {
                    module_index: module_payload.module_index,
                    module,
                    results,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(SpeechAnalyzerOutput {
            modules,
            volatile_range: payload.volatile_range,
        })
    }
}

/// Output collected from a file-backed `SpeechAnalyzer` run.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeechAnalyzerOutput {
    pub modules: Vec<SpeechAnalyzerModuleOutput>,
    pub volatile_range: Option<AudioTimeRange>,
}

/// One module's results from a `SpeechAnalyzer` run.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeechAnalyzerModuleOutput {
    pub module_index: usize,
    pub module: SpeechModuleDescriptor,
    pub results: SpeechAnalyzerModuleResults,
}

/// Results emitted by a specific analyzer module.
#[derive(Debug, Clone, PartialEq)]
pub enum SpeechAnalyzerModuleResults {
    SpeechTranscriber(Vec<SpeechTranscriptionResult>),
    SpeechDetector(Vec<SpeechDetectionResult>),
}

/// Safe wrapper for `AnalyzerInput` raw `AVAudioPCMBuffer *` values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnalyzerInput {
    buffer: NonNull<c_void>,
    buffer_start_time_seconds: Option<f64>,
}

unsafe impl Send for AnalyzerInput {}

impl AnalyzerInput {
    /// # Safety
    ///
    /// `buffer` must be a valid `AVAudioPCMBuffer *` allocated by `AVFoundation`.
    #[must_use]
    pub const unsafe fn from_audio_pcm_buffer_raw(buffer: NonNull<c_void>) -> Self {
        Self {
            buffer,
            buffer_start_time_seconds: None,
        }
    }

    /// # Safety
    ///
    /// `buffer` must be a valid `AVAudioPCMBuffer *` allocated by `AVFoundation`.
    #[must_use]
    pub const unsafe fn from_audio_pcm_buffer_raw_with_start_time(
        buffer: NonNull<c_void>,
        buffer_start_time_seconds: f64,
    ) -> Self {
        Self {
            buffer,
            buffer_start_time_seconds: Some(buffer_start_time_seconds),
        }
    }

    #[must_use]
    pub const fn raw_buffer(self) -> NonNull<c_void> {
        self.buffer
    }

    #[must_use]
    pub const fn buffer_start_time_seconds(self) -> Option<f64> {
        self.buffer_start_time_seconds
    }
}

/// `SpeechModels` lifecycle helpers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SpeechModels;

impl SpeechModels {
    pub fn end_retention() -> Result<(), SpeechError> {
        let mut err_msg = ptr::null_mut();
        let status = unsafe { ffi::sp_speech_models_end_retention(&mut err_msg) };
        if status == ffi::status::OK {
            Ok(())
        } else {
            Err(unsafe { error_from_status_or_json(status, err_msg) })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpeechAnalyzerPayload {
    modules: Vec<SpeechModulePayload>,
    options: Option<SpeechAnalyzerOptionsPayload>,
    context: AnalysisContextPayload,
}

impl From<&SpeechAnalyzer> for SpeechAnalyzerPayload {
    fn from(value: &SpeechAnalyzer) -> Self {
        Self {
            modules: value.modules.iter().map(SpeechModulePayload::from).collect(),
            options: value.options.map(Into::into),
            context: AnalysisContextPayload::from(&value.context),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpeechModulePayload {
    kind: String,
    transcriber: Option<SpeechTranscriberPayload>,
    detector: Option<SpeechDetectorPayload>,
}

impl From<&SpeechModuleDescriptor> for SpeechModulePayload {
    fn from(value: &SpeechModuleDescriptor) -> Self {
        match value {
            SpeechModuleDescriptor::SpeechTranscriber(transcriber) => Self {
                kind: "speechTranscriber".into(),
                transcriber: Some(SpeechTranscriberPayload::from(transcriber)),
                detector: None,
            },
            SpeechModuleDescriptor::SpeechDetector(detector) => Self {
                kind: "speechDetector".into(),
                transcriber: None,
                detector: Some(SpeechDetectorPayload::from(detector)),
            },
        }
    }
}

pub(crate) fn modules_json_cstring(
    modules: &[SpeechModuleDescriptor],
    context: &str,
) -> Result<CString, SpeechError> {
    let payload: Vec<_> = modules.iter().map(SpeechModulePayload::from).collect();
    json_cstring(&payload, context)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpeechTranscriberPayload {
    locale_identifier: String,
    preset: Option<String>,
    transcription_options: Option<Vec<String>>,
    reporting_options: Option<Vec<String>>,
    attribute_options: Option<Vec<String>>,
}

impl From<&SpeechTranscriber> for SpeechTranscriberPayload {
    fn from(value: &SpeechTranscriber) -> Self {
        let (preset, transcription_options, reporting_options, attribute_options) =
            match &value.configuration {
                SpeechTranscriberConfiguration::Preset(preset) => {
                    (Some((*preset).as_str().to_owned()), None, None, None)
                }
                SpeechTranscriberConfiguration::Custom(options) => (
                    None,
                    (!options.transcription.is_empty()).then(|| {
                        options
                            .transcription
                            .iter()
                            .copied()
                            .map(SpeechTranscriptionOption::as_str)
                            .map(str::to_owned)
                            .collect()
                    }),
                    (!options.reporting.is_empty()).then(|| {
                        options
                            .reporting
                            .iter()
                            .copied()
                            .map(SpeechTranscriberReportingOption::as_str)
                            .map(str::to_owned)
                            .collect()
                    }),
                    (!options.attributes.is_empty()).then(|| {
                        options
                            .attributes
                            .iter()
                            .copied()
                            .map(SpeechTranscriberResultAttributeOption::as_str)
                            .map(str::to_owned)
                            .collect()
                    }),
                ),
            };

        Self {
            locale_identifier: value.locale_identifier.clone(),
            preset,
            transcription_options,
            reporting_options,
            attribute_options,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpeechDetectorPayload {
    sensitivity_level: String,
    report_results: bool,
}

impl From<&SpeechDetector> for SpeechDetectorPayload {
    fn from(value: &SpeechDetector) -> Self {
        Self {
            sensitivity_level: value.detection_options.sensitivity_level.as_str().into(),
            report_results: value.report_results,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpeechAnalyzerOptionsPayload {
    priority: &'static str,
    model_retention: &'static str,
}

impl From<SpeechAnalyzerOptions> for SpeechAnalyzerOptionsPayload {
    fn from(value: SpeechAnalyzerOptions) -> Self {
        Self {
            priority: value.priority.as_str(),
            model_retention: value.model_retention.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AnalysisContextPayload {
    contextual_strings: Option<BTreeMap<String, Vec<String>>>,
    user_data: Option<BTreeMap<String, String>>,
}

impl From<&AnalysisContext> for AnalysisContextPayload {
    fn from(value: &AnalysisContext) -> Self {
        Self {
            contextual_strings: (!value.contextual_strings.is_empty()).then(|| {
                value
                    .contextual_strings
                    .iter()
                    .map(|(tag, strings)| (tag.raw_value().to_owned(), strings.clone()))
                    .collect()
            }),
            user_data: (!value.user_data.is_empty()).then(|| {
                value
                    .user_data
                    .iter()
                    .map(|(tag, value)| (tag.raw_value().to_owned(), value.clone()))
                    .collect()
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpeechAnalyzerOutputPayload {
    modules: Vec<SpeechAnalyzerModuleOutputPayload>,
    volatile_range: Option<AudioTimeRange>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpeechAnalyzerModuleOutputPayload {
    module_index: usize,
    kind: String,
    transcriber_results: Option<Vec<SpeechTranscriptionResult>>,
    detector_results: Option<Vec<SpeechDetectionResult>>,
}
