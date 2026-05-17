//! Async API smoke test — exercises all four `async_api` futures.
//!
//! Runs without a real audio file or authorized recognizer: the
//! `requestAuthorization` path exercises the async bridge unconditionally;
//! the recognition / analysis / language-model paths print a skip message
//! when authorization has not been granted or required OS version is absent.

use speech::analyzer::{SpeechAnalyzer, SpeechTranscriber, SpeechTranscriberPreset};
use speech::async_api::{AsyncSpeechAnalyzer, AsyncSpeechLanguageModel, AsyncSpeechRecognizer};
use speech::language_model::LanguageModelConfiguration;
use speech::recognizer::SpeechRecognizer;
use speech::request::UrlRecognitionRequest;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(async {
        // -----------------------------------------------------------------------
        // 1. SFSpeechRecognizer.requestAuthorization
        // -----------------------------------------------------------------------
        println!("[05_async] requesting authorization asynchronously …");
        let status = AsyncSpeechRecognizer::request_authorization().await?;
        println!("[05_async] authorization status: {status:?}");

        if !status.is_authorized() {
            println!(
                "[05_async] not authorized ({status:?}) — skipping recognition, \
                 analysis, and language-model tests (expected on CI / daemon context)"
            );
            return Ok(());
        }

        // -----------------------------------------------------------------------
        // 2. One-shot URL recognition
        //    Try a short audio file if it exists next to the binary; otherwise skip.
        // -----------------------------------------------------------------------
        let audio_fixture = std::path::Path::new("tests/fixtures/hello.m4a");
        if audio_fixture.exists() {
            println!("[05_async] recognizing {} …", audio_fixture.display());
            let recognizer = SpeechRecognizer::new();
            let request = UrlRecognitionRequest::new(audio_fixture);
            let future = AsyncSpeechRecognizer::recognize_url(&recognizer, &request)?;
            match future.await {
                Ok(result) => println!(
                    "[05_async] recognition: {}",
                    result.best_transcription.formatted_string
                ),
                Err(e) => println!("[05_async] recognition error (expected): {e}"),
            }
        } else {
            println!("[05_async] no audio fixture found — skipping recognition (OK on CI)");
        }

        // -----------------------------------------------------------------------
        // 3. SpeechAnalyzer (macOS 26+)
        //    If `SpeechTranscriber::is_available()` returns false, the future
        //    immediately resolves with RecognizerUnavailable.
        // -----------------------------------------------------------------------
        if SpeechTranscriber::is_available() && audio_fixture.exists() {
            println!("[05_async] analyzing {} with SpeechAnalyzer …", audio_fixture.display());
            let transcriber = SpeechTranscriber::new("en-US", SpeechTranscriberPreset::Transcription);
            let analyzer = SpeechAnalyzer::new([transcriber]);
            let future = AsyncSpeechAnalyzer::analyze_in_path(&analyzer, audio_fixture)?;
            match future.await {
                Ok(output) => println!("[05_async] analysis: {} module(s)", output.modules.len()),
                Err(e) => println!("[05_async] analysis error (expected on <macOS 26): {e}"),
            }
        } else {
            println!("[05_async] SpeechTranscriber unavailable or no fixture — skipping (OK on CI)");
        }

        // -----------------------------------------------------------------------
        // 4. SFSpeechLanguageModel.prepareCustomLanguageModel
        //    Skip silently when the asset doesn't exist.
        // -----------------------------------------------------------------------
        let model_fixture = std::path::Path::new("tests/fixtures/custom_model");
        if model_fixture.exists() {
            println!(
                "[05_async] preparing custom language model at {} …",
                model_fixture.display()
            );
            let config = LanguageModelConfiguration::new(model_fixture);
            let future =
                AsyncSpeechLanguageModel::prepare_custom_language_model(model_fixture, &config)?;
            match future.await {
                Ok(()) => println!("[05_async] language model prepared"),
                Err(e) => println!("[05_async] language model error (expected on CI): {e}"),
            }
        } else {
            println!("[05_async] no model fixture — skipping language model test (OK on CI)");
        }

        Ok::<(), Box<dyn std::error::Error>>(())
    })
}
