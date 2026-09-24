use std::path::{Path, PathBuf};
use std::process::Command;

use speech::prelude::*;

fn synthesized_utterance(name: &str) -> Option<PathBuf> {
    let audio = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let spoken = Command::new("/usr/bin/say")
        .arg("-o")
        .arg(&audio)
        .arg("the quick brown fox jumps over the lazy dog")
        .status();
    spoken.is_ok_and(|status| status.success()).then_some(audio)
}

fn on_device_en_us_recognizer() -> Option<SpeechRecognizer> {
    if !SpeechRecognizer::authorization_status().is_authorized() {
        println!("skipping: speech recognition is not authorized for this process");
        return None;
    }
    let recognizer = SpeechRecognizer::with_locale("en-US");
    if !recognizer.is_available() || !recognizer.supports_on_device_recognition().unwrap_or(false)
    {
        println!("skipping: on-device en-US recognition is unavailable");
        return None;
    }
    Some(recognizer)
}

#[test]
fn recognize_in_path_completes_on_the_default_callback_queue() {
    let Some(recognizer) = on_device_en_us_recognizer() else {
        return;
    };
    let Some(audio) = synthesized_utterance("speech-rs-default-queue.aiff") else {
        println!("skipping: `say` could not synthesize a test utterance");
        return;
    };

    let result = recognizer
        .recognize_in_path(&audio)
        .expect("on-device recognition of synthesized speech succeeds");
    assert!(
        result.transcript.to_lowercase().contains("fox"),
        "unexpected transcript: {}",
        result.transcript
    );
}
