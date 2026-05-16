use speech::prelude::*;

#[test]
fn dictation_transcriber_builders_preserve_configuration() {
    let preset = DictationTranscriber::new("en-US", DictationPreset::ShortDictation);
    assert_eq!(preset.locale_identifier(), "en-US");
    assert_eq!(preset.preset(), Some(DictationPreset::ShortDictation));
    assert!(preset.options().is_none());

    let options = DictationTranscriberOptions::new()
        .with_content_hints([
            DictationContentHint::ShortForm,
            DictationContentHint::customized_language_model(
                LanguageModelConfiguration::new("target/dictation-language-model.bin")
                    .with_vocabulary("target/dictation-vocabulary.txt"),
            ),
        ])
        .with_transcription_options([
            DictationTranscriptionOption::Punctuation,
            DictationTranscriptionOption::Emoji,
        ])
        .with_reporting_options([
            DictationReportingOption::VolatileResults,
            DictationReportingOption::FrequentFinalization,
        ])
        .with_attribute_options([
            DictationResultAttributeOption::AudioTimeRange,
            DictationResultAttributeOption::TranscriptionConfidence,
        ]);
    let custom = DictationTranscriber::with_options("en-GB", options.clone());
    assert_eq!(custom.locale_identifier(), "en-GB");
    assert_eq!(custom.preset(), None);
    assert_eq!(custom.options(), Some(&options));
}

#[test]
fn dictation_transcription_result_helper_returns_plain_text() {
    let result: DictationTranscriptionResult = serde_json::from_str(
        r#"{
            "text": "hello world",
            "alternatives": ["hello world", "yellow world"],
            "range": {"startSeconds": 0.25, "durationSeconds": 1.5},
            "resultsFinalizationTimeSeconds": 1.75,
            "isFinal": true
        }"#,
    )
    .expect("parse dictation result JSON");

    assert_eq!(result.transcript(), "hello world");
    assert!(result.is_final);
    assert!((result.audio_time_range.start_seconds - 0.25).abs() < f64::EPSILON);
    assert!((result.audio_time_range.duration_seconds - 1.5).abs() < f64::EPSILON);
    assert_eq!(result.alternatives.len(), 2);
}
