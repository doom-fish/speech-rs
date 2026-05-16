use serde::Deserialize;

/// A range inside a transcription's formatted string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct TextRange {
    pub location: usize,
    pub length: usize,
}

/// One acoustic feature value per audio frame.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AcousticFeature {
    #[serde(rename = "acousticFeatureValuePerFrame")]
    pub acoustic_feature_value_per_frame: Vec<f64>,
    #[serde(rename = "frameDuration")]
    pub frame_duration: f64,
}

/// Vocal analytics emitted by Speech.framework.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct VoiceAnalytics {
    pub jitter: AcousticFeature,
    pub shimmer: AcousticFeature,
    pub pitch: AcousticFeature,
    pub voicing: AcousticFeature,
}

/// A rich transcription segment with alternative hypotheses and optional voice analytics.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionSegmentDetails {
    pub substring: String,
    pub substring_range: TextRange,
    pub timestamp: f64,
    pub duration: f64,
    pub confidence: f32,
    pub alternative_substrings: Vec<String>,
    pub voice_analytics: Option<VoiceAnalytics>,
}

/// A full transcription hypothesis.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcription {
    pub formatted_string: String,
    pub segments: Vec<TranscriptionSegmentDetails>,
    pub speaking_rate: Option<f64>,
    pub average_pause_duration: Option<f64>,
}

/// Rich metadata associated with a recognition result.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailedRecognitionMetadata {
    pub speaking_rate: f64,
    pub average_pause_duration: f64,
    pub speech_start_timestamp: f64,
    pub speech_duration: f64,
    pub voice_analytics: Option<VoiceAnalytics>,
}

/// A full `SFSpeechRecognitionResult`, including alternative transcriptions.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailedRecognitionResult {
    pub best_transcription: Transcription,
    pub transcriptions: Vec<Transcription>,
    #[serde(rename = "isFinal")]
    pub is_final: bool,
    pub speech_recognition_metadata: Option<DetailedRecognitionMetadata>,
}

impl DetailedRecognitionResult {
    #[must_use]
    pub fn transcript(&self) -> &str {
        &self.best_transcription.formatted_string
    }

    #[must_use]
    pub fn segments(&self) -> &[TranscriptionSegmentDetails] {
        &self.best_transcription.segments
    }
}
