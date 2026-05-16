//! Smoke test for the macOS 26 `DictationTranscriber` bridge.
//!
//! Run with: `cargo run --example 03_dictation_smoke`

use speech::prelude::*;
use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let aiff_path: PathBuf = std::env::current_dir()?.join("target/dictation_speech_smoke.aiff");
    let target_text = "the quick brown fox jumps over the lazy dog";

    println!("== Step 1: render speech audio via /usr/bin/say ==");
    let _ = std::fs::remove_file(&aiff_path);
    let status = Command::new("/usr/bin/say")
        .args(["-o", aiff_path.to_str().unwrap(), target_text])
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
            eprintln!("\nSKIP: speech recognition not authorized for dictation smoke.");
            return Ok(());
        }
    }

    println!("\n== Step 3: inspect dictation surface ==");
    let supported_locales = match DictationTranscriber::supported_locales() {
        Ok(locales) => locales,
        Err(SpeechError::RecognizerUnavailable(message)) => {
            eprintln!("\nSKIP: dictation transcriber unavailable: {message}");
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    println!("supported locales: {}", supported_locales.len());
    println!(
        "installed locales: {}",
        DictationTranscriber::installed_locales()?.len()
    );

    let transcriber = DictationTranscriber::new("en-US", DictationPreset::ShortDictation);
    println!("selected locales: {:?}", transcriber.selected_locales()?);
    println!(
        "compatible formats: {:?}",
        transcriber.available_compatible_audio_formats()?
    );

    println!("\n== Step 4: transcribe the file ==");
    let results = match transcriber.transcribe_in_path(&aiff_path) {
        Ok(results) => results,
        Err(
            SpeechError::RecognizerUnavailable(_)
            | SpeechError::TimedOut(_)
            | SpeechError::Framework(_),
        ) => {
            eprintln!("\nSKIP: dictation transcription unavailable in this environment.");
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };

    println!("result count: {}", results.len());
    for (index, result) in results.iter().enumerate() {
        println!(
            "  #{index}: '{}' final={} start={:.2}s dur={:.2}s",
            result.text,
            result.is_final,
            result.audio_time_range.start_seconds,
            result.audio_time_range.duration_seconds
        );
    }

    let combined = results
        .iter()
        .map(DictationTranscriptionResult::transcript)
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    assert!(
        combined.contains("fox") || combined.contains("brown"),
        "expected dictation transcriber to recognize at least 'fox' or 'brown'; got {combined:?}"
    );
    println!("\nOK DictationTranscriber returned the expected transcript");
    Ok(())
}
