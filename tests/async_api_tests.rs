//! Tests for the `async_api` module.
//!
//! These tests run on macOS and exercise the Rust Future machinery without
//! requiring a real audio file or speech authorization:
//!
//! - `request_authorization` always fires the Swift callback → the Future
//!   must resolve without hanging.
//! - `recognize_url` / `analyze_in_path` / `prepare_custom_language_model`
//!   with a non-existent path are expected to produce errors — the important
//!   thing is that the callback fires and the Future resolves.

#[cfg(feature = "async")]
mod async_tests {
    use speech::analyzer::{SpeechAnalyzer, SpeechTranscriber, SpeechTranscriberPreset};
    use speech::async_api::{
        AsyncSpeechAnalyzer, AsyncSpeechLanguageModel, AsyncSpeechRecognizer,
    };
    use speech::error::SpeechError;
    use speech::language_model::LanguageModelConfiguration;
    use speech::recognizer::SpeechRecognizer;
    use speech::request::{RecognitionRequestOptions, UrlRecognitionRequest};
    use std::path::Path;

    // -----------------------------------------------------------------------
    // Happy path: request_authorization always resolves
    // -----------------------------------------------------------------------

    #[test]
    fn test_request_authorization_resolves() {
        let status = pollster::block_on(AsyncSpeechRecognizer::request_authorization());
        // The future itself must not panic and must return Ok.
        let status = status.expect("authorization future should resolve without error");
        // Any valid variant is acceptable (NotDetermined, Denied, Restricted, Authorized).
        let _ = format!("{status:?}");
        println!("test_request_authorization_resolves: status = {status:?}");
    }

    // -----------------------------------------------------------------------
    // Error path: recognize_url with a non-existent file
    // -----------------------------------------------------------------------

    #[test]
    fn test_recognize_url_nonexistent_returns_error() {
        let recognizer = SpeechRecognizer::new();
        let request = UrlRecognitionRequest::new(Path::new("/nonexistent/no_such_file.m4a"));
        let future = AsyncSpeechRecognizer::recognize_url(&recognizer, &request)
            .expect("building RecognizeUrlFuture should not fail for a valid (if missing) path");
        let result = pollster::block_on(future);
        assert!(
            result.is_err(),
            "recognition of a missing file must resolve with an error; got {result:?}"
        );
        println!(
            "test_recognize_url_nonexistent_returns_error: error = {:?}",
            result.unwrap_err()
        );
    }

    // -----------------------------------------------------------------------
    // Error path: analyze_in_path with a non-existent file
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_url_nonexistent_returns_error() {
        let transcriber = SpeechTranscriber::new("en-US", SpeechTranscriberPreset::Transcription);
        let analyzer = SpeechAnalyzer::new([transcriber]);
        let future = AsyncSpeechAnalyzer::analyze_in_path(
            &analyzer,
            Path::new("/nonexistent/no_such_file.m4a"),
        )
        .expect(
            "building AnalyzeUrlFuture should not fail for a valid (if missing) path",
        );
        let result = pollster::block_on(future);
        assert!(
            result.is_err(),
            "analysis of a missing file must resolve with an error; got {result:?}"
        );
        println!(
            "test_analyze_url_nonexistent_returns_error: error = {:?}",
            result.unwrap_err()
        );
    }

    // -----------------------------------------------------------------------
    // Error path: analyze_in_path with empty module list
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_url_no_modules_returns_invalid_argument() {
        let analyzer = SpeechAnalyzer::new::<SpeechTranscriber, _>([]);
        let result = AsyncSpeechAnalyzer::analyze_in_path(
            &analyzer,
            Path::new("/nonexistent/no_such_file.m4a"),
        );
        assert!(
            matches!(result, Err(SpeechError::InvalidArgument(_))),
            "analyzer with no modules must return InvalidArgument immediately; got {result:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Error path: prepare_custom_language_model with a non-existent asset
    // -----------------------------------------------------------------------

    #[test]
    fn test_prepare_language_model_nonexistent_returns_error() {
        let config = LanguageModelConfiguration::new("/nonexistent/no_such_model");
        let future = AsyncSpeechLanguageModel::prepare_custom_language_model(
            Path::new("/nonexistent/no_such_model"),
            &config,
        )
        .expect("building PrepareLanguageModelFuture should not fail for a valid path");
        let result = pollster::block_on(future);
        assert!(
            result.is_err(),
            "preparing a missing model must resolve with an error; got {result:?}"
        );
        println!(
            "test_prepare_language_model_nonexistent_returns_error: error = {:?}",
            result.unwrap_err()
        );
    }

    const ROUNDS: usize = 32;

    fn missing_path(kind: &str, round: usize) -> String {
        format!(
            "/nonexistent/speech-rs-{kind}-{}-{round}-{}.m4a",
            std::process::id(),
            "x".repeat(40)
        )
    }

    fn scribble_freed_allocations() -> Vec<Vec<u8>> {
        (1..=32)
            .flat_map(|size| (0..8).map(move |_| vec![b'#'; size * 16]))
            .collect()
    }

    fn assert_each_error_names_its_path<T, F>(pending: Vec<(String, F, Vec<Vec<u8>>)>)
    where
        T: std::fmt::Debug,
        F: std::future::Future<Output = Result<T, SpeechError>>,
    {
        for (path, future, _scribble) in pending {
            let error = pollster::block_on(future).expect_err("a missing input must fail");
            if matches!(error, SpeechError::RecognizerUnavailable(_)) {
                println!("skipping: the API is unavailable on this system: {error}");
                return;
            }
            assert!(
                error.to_string().contains(&path),
                "the error does not name the path that was passed in ({path}): {error}"
            );
            assert!(
                matches!(error, SpeechError::AudioLoadFailed(_)),
                "unexpected error kind for a missing input: {error:?}"
            );
        }
    }

    #[test]
    fn recognize_url_copies_its_inputs_before_returning() {
        let pending = (0..ROUNDS)
            .map(|round| {
                let path = missing_path("recognize", round);
                let recognizer = SpeechRecognizer::with_locale("en-US")
                    .expect("en-US is a valid locale identifier");
                let request = UrlRecognitionRequest::new(&path).with_options(
                    RecognitionRequestOptions::new().with_contextual_strings(["doom fish"]),
                );
                let future = AsyncSpeechRecognizer::recognize_url(&recognizer, &request)
                    .expect("valid inputs build a future");
                drop(request);
                drop(recognizer);
                (path, future, scribble_freed_allocations())
            })
            .collect();
        assert_each_error_names_its_path(pending);
    }

    #[test]
    fn analyze_in_path_copies_its_inputs_before_returning() {
        let pending = (0..ROUNDS)
            .map(|round| {
                let path = missing_path("analyze", round);
                let analyzer = SpeechAnalyzer::new([SpeechTranscriber::new(
                    "en-US",
                    SpeechTranscriberPreset::Transcription,
                )]);
                let future = AsyncSpeechAnalyzer::analyze_in_path(&analyzer, Path::new(&path))
                    .expect("valid inputs build a future");
                drop(analyzer);
                (path, future, scribble_freed_allocations())
            })
            .collect();
        assert_each_error_names_its_path(pending);
    }

    #[test]
    fn prepare_custom_language_model_copies_its_inputs_before_returning() {
        let pending = (0..ROUNDS)
            .map(|round| {
                let path = missing_path("language-model", round);
                let configuration = LanguageModelConfiguration::new(&path)
                    .with_vocabulary(format!("{path}.vocabulary"));
                let future = AsyncSpeechLanguageModel::prepare_custom_language_model(
                    Path::new(&path),
                    &configuration,
                )
                .expect("valid inputs build a future");
                drop(configuration);
                (path, future, scribble_freed_allocations())
            })
            .collect();
        assert_each_error_names_its_path(pending);
    }

    #[test]
    fn recognize_url_resolves_with_the_final_result_after_partial_results() {
        if !SpeechRecognizer::authorization_status().is_authorized() {
            println!("skipping: speech recognition is not authorized for this process");
            return;
        }
        let recognizer =
            SpeechRecognizer::with_locale("en-US").expect("en-US is a valid locale identifier");
        if !recognizer.is_available()
            || !recognizer.supports_on_device_recognition().unwrap_or(false)
        {
            println!("skipping: on-device en-US recognition is unavailable");
            return;
        }
        let audio = Path::new(env!("CARGO_TARGET_TMPDIR")).join("speech-rs-one-shot.aiff");
        let spoken = std::process::Command::new("/usr/bin/say")
            .arg("-o")
            .arg(&audio)
            .arg("the quick brown fox jumps over the lazy dog")
            .status();
        if !spoken.is_ok_and(|status| status.success()) {
            println!("skipping: `say` could not synthesize a test utterance");
            return;
        }

        let request = UrlRecognitionRequest::new(&audio).with_options(
            RecognitionRequestOptions::new().with_should_report_partial_results(true),
        );
        let future = AsyncSpeechRecognizer::recognize_url(&recognizer, &request)
            .expect("valid inputs build a future");
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(pollster::block_on(future));
        });
        let result = receiver
            .recv_timeout(std::time::Duration::from_secs(120))
            .expect("recognize_url must resolve once the final result arrives")
            .expect("on-device recognition of synthesized speech succeeds");

        assert!(result.is_final, "the future resolved with a partial result");
        assert!(
            result.transcript().to_lowercase().contains("fox"),
            "unexpected transcript: {}",
            result.transcript()
        );
    }
}
