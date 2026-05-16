#![allow(clippy::too_many_lines)]

use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use speech::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("== Speech.framework surface smoke ==");

    let supported_locales = SpeechRecognizer::supported_locales()?;
    println!("supported locales: {}", supported_locales.len());
    println!(
        "sample locales: {:?}",
        supported_locales.iter().take(5).collect::<Vec<_>>()
    );

    let recognizer = SpeechRecognizer::new()
        .with_default_task_hint(TaskHint::Dictation)
        .with_callback_queue(CallbackQueue::named("speech-smoke-callbacks"));
    println!("resolved locale: {}", recognizer.locale_identifier()?);
    println!(
        "supports on-device recognition: {}",
        recognizer.supports_on_device_recognition()?
    );

    let _availability_observer = recognizer.observe_availability_changes(|available| {
        println!("availability changed: {available}");
    })?;

    let audio_buffer_request = AudioBufferRecognitionRequest::new();
    let native_audio_format = audio_buffer_request.native_audio_format()?;
    println!(
        "native audio format: {:?}, {}ch @ {} Hz interleaved={}",
        native_audio_format.common_format,
        native_audio_format.channel_count,
        native_audio_format.sample_rate,
        native_audio_format.is_interleaved
    );

    let custom_model = LanguageModelConfiguration::new("target/example-language-model.bin")
        .with_vocabulary("target/example-vocabulary.txt")
        .with_weight(0.5);
    println!(
        "language model config: model={}, vocab={:?}, weight={:?}",
        custom_model.language_model().display(),
        custom_model
            .vocabulary()
            .map(|path| path.display().to_string()),
        custom_model.weight()
    );

    let status = SpeechRecognizer::authorization_status();
    println!("authorization status: {status:?}");
    if !status.is_authorized() {
        println!(
            "authorization not granted; advanced object creation succeeded, skipping recognition."
        );
        return Ok(());
    }

    let audio_path = std::env::current_dir()?.join("target/speech_framework_smoke.aiff");
    let _ = std::fs::remove_file(&audio_path);
    let status = Command::new("/usr/bin/say")
        .args([
            "-o",
            audio_path.to_str().ok_or("non-UTF-8 output path")?,
            "the quick brown fox jumps over the lazy dog",
        ])
        .status()?;
    if !status.success() {
        return Err(format!("`say` failed with status {status}").into());
    }
    println!("synthesized audio: {}", audio_path.display());

    let request = UrlRecognitionRequest::new(&audio_path).with_options(
        RecognitionRequestOptions::new()
            .with_task_hint(TaskHint::Dictation)
            .with_contextual_strings(["quick brown fox", "lazy dog"])
            .with_should_report_partial_results(true)
            .with_requires_on_device_recognition(true)
            .with_adds_punctuation(true),
    );

    let detailed = recognizer.recognize_request(&request)?;
    println!("sync transcript: {}", detailed.transcript());
    println!("sync alternatives: {}", detailed.transcriptions.len());
    println!(
        "sync metadata available: {}",
        detailed.speech_recognition_metadata.is_some()
    );

    let (tx, rx) = mpsc::channel();
    let task = recognizer.start_url_task(&request, move |event| {
        println!("task event: {event:?}");
        let _ = tx.send(event);
    })?;
    println!("initial task state: {:?}", task.state());

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut saw_delegate_event = false;
    let mut saw_success = false;
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(event) => {
                saw_delegate_event = true;
                if matches!(event, RecognitionTaskEvent::DidFinishSuccessfully(true)) {
                    saw_success = true;
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if matches!(task.state(), TaskState::Completed) {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    println!("final task state: {:?}", task.state());
    assert!(
        saw_delegate_event,
        "expected the delegate pipeline to emit at least one event"
    );
    assert!(
        saw_success || matches!(task.state(), TaskState::Completed),
        "expected the task to complete successfully"
    );

    let _ = std::fs::remove_file(&audio_path);
    println!("OK full framework smoke finished successfully");
    Ok(())
}
