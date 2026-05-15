//! Smoke test: synthesise "the quick brown fox jumps over the lazy dog"
//! to an AIFF, then recognize it back. If authorization isn't granted (CLI
//! binaries usually aren't), gracefully skip.
//!
//! Run with: `cargo run --example 01_recognize_smoke`

use std::path::PathBuf;
use std::process::Command;
use speech::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let aiff_path: PathBuf = "/tmp/speech_smoke.aiff".into();
    let target_text = "the quick brown fox jumps over the lazy dog";

    println!("== Step 1: render speech audio via /usr/bin/say ==");
    let _ = std::fs::remove_file(&aiff_path);
    let status = Command::new("/usr/bin/say")
        .args(["-o", aiff_path.to_str().unwrap(), target_text])
        .status()?;
    if !status.success() {
        return Err(format!("`say` failed with status {status}").into());
    }
    let metadata = std::fs::metadata(&aiff_path)?;
    println!("synthesized {} ({} bytes)", aiff_path.display(), metadata.len());

    println!("\n== Step 2: check authorization ==");
    let status = SpeechRecognizer::authorization_status();
    println!("authorization status: {status:?}");
    if !status.is_authorized() {
        let new_status = SpeechRecognizer::request_authorization();
        println!("after request: {new_status:?}");
        if !new_status.is_authorized() {
            eprintln!(
                "\nSKIP: speech recognition not authorized.\n\
                 Add NSSpeechRecognitionUsageDescription to your Info.plist\n\
                 and run from a proper app bundle to grant authorization."
            );
            return Ok(());
        }
    }

    println!("\n== Step 3: locale + recognizer availability ==");
    let default_locale = SpeechRecognizer::default_locale_identifier();
    println!("default locale: {default_locale:?}");
    let recognizer = SpeechRecognizer::new();
    println!("recognizer is_available: {}", recognizer.is_available());

    println!("\n== Step 4: recognize the file ==");
    let result = recognizer.recognize_in_path(&aiff_path)?;
    println!("transcript: '{}'", result.transcript);
    println!("{} segment(s):", result.segments.len());
    for seg in &result.segments {
        println!(
            "  [{:.2}] '{}' t={:.2}s dur={:.2}s",
            seg.confidence, seg.text, seg.timestamp, seg.duration
        );
    }

    let combined = result.transcript.to_lowercase();
    assert!(
        combined.contains("fox") || combined.contains("brown"),
        "expected to recognize at least 'fox' or 'brown'; got {combined:?}"
    );
    println!("\nOK Speech recognition returned the expected transcript");
    Ok(())
}
