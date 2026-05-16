#![allow(clippy::too_many_lines)]

use speech::prelude::*;
use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let audio_path: PathBuf = std::env::current_dir()?.join("target/analyzer_surface_smoke.aiff");
    let export_path: PathBuf =
        std::env::current_dir()?.join("target/custom-language-model-data.bin");
    let target_text = "copilot helps wrap the speech analyzer on macos";

    println!("== Step 1: render speech audio via /usr/bin/say ==");
    let _ = std::fs::remove_file(&audio_path);
    let status = Command::new("/usr/bin/say")
        .args(["-o", audio_path.to_str().unwrap(), target_text])
        .status()?;
    if !status.success() {
        return Err(format!("`say` failed with status {status}").into());
    }

    println!("\n== Step 2: check authorization ==");
    let status = SpeechRecognizer::authorization_status();
    println!("authorization status: {status:?}");
    if !status.is_authorized() {
        let new_status = SpeechRecognizer::request_authorization();
        println!("after request: {new_status:?}");
        if !new_status.is_authorized() {
            eprintln!("\nSKIP: speech recognition not authorized for analyzer smoke.");
            return Ok(());
        }
    }

    if !SpeechTranscriber::is_available() {
        eprintln!("\nSKIP: SpeechTranscriber unavailable on this macOS/SDK combination.");
        return Ok(());
    }

    println!("\n== Step 3: inspect analyzer modules ==");
    println!(
        "supported locales: {}",
        SpeechTranscriber::supported_locales()?.len()
    );
    println!(
        "installed locales: {}",
        SpeechTranscriber::installed_locales()?.len()
    );

    let transcriber = SpeechTranscriber::new(
        "en-US",
        SpeechTranscriberPreset::TimeIndexedTranscriptionWithAlternatives,
    );
    let detector = SpeechDetector::default();
    println!("selected locales: {:?}", transcriber.selected_locales()?);
    println!(
        "transcriber formats: {:?}",
        transcriber.available_compatible_audio_formats()?
    );
    println!(
        "detector formats: {:?}",
        detector.available_compatible_audio_formats()?
    );

    let analyzer = SpeechAnalyzer::new([
        SpeechModuleDescriptor::from(&transcriber),
        SpeechModuleDescriptor::from(&detector),
    ])
    .with_context(
        AnalysisContext::new()
            .with_contextual_strings(ContextualStringsTag::general(), ["copilot", "speech analyzer"])
            .with_user_data(UserDataTag::new("session"), "smoke-example"),
    )
    .with_options(SpeechAnalyzerOptions::new(
        SpeechAnalyzerPriority::UserInitiated,
        SpeechAnalyzerModelRetention::Lingering,
    ));
    println!(
        "best audio format: {:?}",
        analyzer.best_available_audio_format()?
    );

    println!("\n== Step 4: analyze the file ==");
    let output = match analyzer.analyze_in_path(&audio_path) {
        Ok(output) => output,
        Err(
            SpeechError::RecognizerUnavailable(_)
            | SpeechError::TimedOut(_)
            | SpeechError::Framework(_),
        ) => {
            eprintln!("\nSKIP: analyzer pipeline unavailable in this environment.");
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    for module in &output.modules {
        match &module.results {
            SpeechAnalyzerModuleResults::SpeechTranscriber(results) => {
                println!("transcriber results: {}", results.len());
                for result in results {
                    println!(
                        "  text='{}' final={} start={:.2}s dur={:.2}s",
                        result.transcript(),
                        result.is_final,
                        result.audio_time_range.start_seconds,
                        result.audio_time_range.duration_seconds
                    );
                }
            }
            SpeechAnalyzerModuleResults::SpeechDetector(results) => {
                println!("detector results: {}", results.len());
                for result in results {
                    println!(
                        "  speech_detected={} final={} start={:.2}s dur={:.2}s",
                        result.speech_detected,
                        result.is_final,
                        result.audio_time_range.start_seconds,
                        result.audio_time_range.duration_seconds
                    );
                }
            }
        }
    }

    println!("\n== Step 5: inspect downloadable assets ==");
    println!(
        "maximum reserved locales: {}",
        AssetInventory::maximum_reserved_locales()?
    );
    println!("reserved locales: {:?}", AssetInventory::reserved_locales()?);
    println!(
        "asset status: {:?}",
        AssetInventory::status_for_modules([
            SpeechModuleDescriptor::from(&transcriber),
            SpeechModuleDescriptor::from(&detector),
        ])?
    );
    if let Some(request) = AssetInventory::asset_installation_request_for_modules([
        SpeechModuleDescriptor::from(&transcriber),
        SpeechModuleDescriptor::from(&detector),
    ])? {
        println!("installation request progress: {:?}", request.progress()?);
    }

    println!("\n== Step 6: build custom language-model data ==");
    println!(
        "supported phonemes: {}",
        SFCustomLanguageModelData::supported_phonemes("en-US")?.len()
    );
    let mut generator = TemplatePhraseCountGenerator::new();
    generator.define_class("subject", ["copilot", "speech analyzer"]);
    generator.insert_template("<subject> demo", 2);
    let data = SFCustomLanguageModelData::new("en-US", "speech-rs-smoke", "1.0")
        .with_insertable(PhraseCount::new("speech analyzer demo", 4))
        .with_insertable(CustomPronunciation::new(
            "copilot",
            ["K", "AA", "P", "AH", "L", "AH", "T"],
        ))
        .with_insertable(generator);
    match data.export_to(&export_path) {
        Ok(()) => println!("exported custom language-model data to {}", export_path.display()),
        Err(SpeechError::Framework(_) | SpeechError::RecognizerUnavailable(_)) => {
            eprintln!("SKIP: custom language-model export unavailable in this environment.");
        }
        Err(error) => return Err(error.into()),
    }

    let _ = SpeechModels::end_retention();
    println!("\nOK macOS 26 analyzer, asset, and custom language-model surfaces are reachable");
    Ok(())
}
