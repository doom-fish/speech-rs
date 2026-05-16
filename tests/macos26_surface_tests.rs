use std::collections::BTreeMap;
use std::ptr::NonNull;

use speech::prelude::*;

#[test]
fn speech_transcriber_builders_preserve_configuration() {
    let preset = SpeechTranscriber::new(
        "en-US",
        SpeechTranscriberPreset::TimeIndexedTranscriptionWithAlternatives,
    );
    assert_eq!(preset.locale_identifier(), "en-US");
    assert_eq!(
        preset.preset(),
        Some(SpeechTranscriberPreset::TimeIndexedTranscriptionWithAlternatives)
    );
    assert!(preset.options().is_none());

    let options = SpeechTranscriberOptions::new()
        .with_transcription_options([SpeechTranscriptionOption::EtiquetteReplacements])
        .with_reporting_options([
            SpeechTranscriberReportingOption::VolatileResults,
            SpeechTranscriberReportingOption::FastResults,
        ])
        .with_attribute_options([
            SpeechTranscriberResultAttributeOption::AudioTimeRange,
            SpeechTranscriberResultAttributeOption::TranscriptionConfidence,
        ]);
    let custom = SpeechTranscriber::with_options("en-GB", options.clone());
    assert_eq!(custom.locale_identifier(), "en-GB");
    assert_eq!(custom.preset(), None);
    assert_eq!(custom.options(), Some(&options));
}

#[test]
fn speech_analyzer_and_detector_builders_preserve_configuration() {
    let detector = SpeechDetector::new(
        SpeechDetectionOptions::new(SpeechDetectorSensitivityLevel::High),
        false,
    );
    assert_eq!(
        detector.detection_options().sensitivity_level(),
        SpeechDetectorSensitivityLevel::High
    );
    assert!(!detector.report_results());

    let context = AnalysisContext::new()
        .with_contextual_strings(ContextualStringsTag::general(), ["doom fish", "copilot"])
        .with_user_data(UserDataTag::new("session"), "demo");
    let options = SpeechAnalyzerOptions::new(
        SpeechAnalyzerPriority::UserInitiated,
        SpeechAnalyzerModelRetention::Lingering,
    );
    let analyzer = SpeechAnalyzer::new([
        SpeechModuleDescriptor::from(&SpeechTranscriber::new(
            "en-US",
            SpeechTranscriberPreset::Transcription,
        )),
        SpeechModuleDescriptor::from(&detector),
    ])
    .with_context(context.clone())
    .with_options(options);

    assert_eq!(analyzer.modules().len(), 2);
    assert_eq!(analyzer.options(), Some(options));
    assert_eq!(analyzer.context(), &context);
    assert_eq!(
        context
            .contextual_strings()
            .get(&ContextualStringsTag::general())
            .expect("general tag"),
        &["doom fish".to_string(), "copilot".to_string()]
    );
    assert_eq!(
        context
            .user_data()
            .get(&UserDataTag::new("session"))
            .expect("session tag"),
        "demo"
    );
}

#[test]
fn speech_transcription_result_decodes_attributed_text_and_time_ranges() {
    let result: SpeechTranscriptionResult = serde_json::from_str(
        r#"{
            "range": {"startSeconds": 0.0, "durationSeconds": 1.2},
            "resultsFinalizationTimeSeconds": 1.2,
            "text": {
                "text": "hello world",
                "spans": [
                    {
                        "start": 0,
                        "end": 5,
                        "transcriptionConfidence": 0.9,
                        "audioTimeRange": {"startSeconds": 0.0, "durationSeconds": 0.5}
                    },
                    {
                        "start": 6,
                        "end": 11,
                        "audioTimeRange": {"startSeconds": 0.5, "durationSeconds": 0.7}
                    }
                ]
            },
            "alternatives": [
                {"text": "hullo world", "spans": []}
            ],
            "isFinal": true
        }"#,
    )
    .expect("parse speech transcriber result JSON");

    assert_eq!(result.transcript(), "hello world");
    assert!(result.is_final);
    assert_eq!(
        result
            .text
            .range_of_audio_time_range_attributes_intersecting(AudioTimeRange::new(0.25, 0.1)),
        Some(0..5)
    );
    assert_eq!(
        result
            .text
            .range_of_audio_time_range_attributes_intersecting(AudioTimeRange::new(0.55, 0.1)),
        Some(6..11)
    );
    assert_eq!(result.alternatives[0].as_str(), "hullo world");
}

#[test]
fn analyzer_input_and_error_code_helpers_cover_macos26_surface() {
    let raw = NonNull::<u8>::dangling().cast();
    let input = unsafe { AnalyzerInput::from_audio_pcm_buffer_raw_with_start_time(raw, 1.25) };
    assert_eq!(input.raw_buffer(), raw);
    assert_eq!(input.buffer_start_time_seconds(), Some(1.25));

    assert_eq!(
        SpeechFrameworkErrorCode::from_domain_code_and_message(
            SPEECH_ERROR_DOMAIN,
            2,
            "audio frames arrived disordered"
        ),
        SpeechFrameworkErrorCode::AudioDisordered
    );
    assert_eq!(
        SpeechFrameworkErrorCode::from_domain_code_and_message(SPEECH_ERROR_DOMAIN, 3, ""),
        SpeechFrameworkErrorCode::UnexpectedAudioFormat
    );
    assert_eq!(
        SpeechFrameworkErrorCode::from_domain_code_and_message(SPEECH_ERROR_DOMAIN, 16, ""),
        SpeechFrameworkErrorCode::InsufficientResources
    );
}

#[test]
fn asset_inventory_and_custom_language_model_builders_work() {
    assert!(AssetInventoryStatus::Installed > AssetInventoryStatus::Downloading);

    let mut generator = PhraseCountGenerator::new();
    generator.push(PhraseCount::new("hello world", 3));
    assert_eq!(generator.iter().collect::<Vec<_>>(), vec![PhraseCount::new("hello world", 3)]);

    let mut template_generator = TemplatePhraseCountGenerator::new();
    template_generator.define_class("greeting", ["hello", "hi"]);
    template_generator.insert_template("<greeting> world", 2);
    assert_eq!(
        template_generator.iter().collect::<Vec<_>>(),
        vec![
            PhraseCount::new("hello world", 2),
            PhraseCount::new("hi world", 2),
        ]
    );

    let template_builder = TemplateInsertableBuilder::new().with(CompoundTemplate::new([
        TemplatePhraseCountGeneratorTemplate::new("<greeting> there", 1),
    ]));
    let phrase_counts = PhraseCountsFromTemplates::new(
        BTreeMap::from([(
            "greeting".to_string(),
            vec!["hey".to_string(), "hiya".to_string()],
        )]),
        template_builder,
    );
    assert_eq!(
        phrase_counts.expanded_phrase_counts(),
        vec![
            PhraseCount::new("hey there", 1),
            PhraseCount::new("hiya there", 1),
        ]
    );

    let data = SFCustomLanguageModelData::new("en-US", "demo", "1.0")
        .with_insertable(PhraseCount::new("copilot cli", 4))
        .with_insertable(CustomPronunciation::new(
            "copilot",
            ["K", "AA", "P", "AH", "L", "AH", "T"],
        ))
        .with_insertable(generator)
        .with_insertable(template_generator)
        .with_insertable(phrase_counts);

    assert_eq!(data.locale_identifier(), "en-US");
    assert_eq!(data.identifier(), "demo");
    assert_eq!(data.version(), "1.0");
}
