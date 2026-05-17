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
    use speech::async_api::{
        AsyncSpeechAnalyzer, AsyncSpeechLanguageModel, AsyncSpeechRecognizer,
    };
    use speech::analyzer::{SpeechAnalyzer, SpeechTranscriber, SpeechTranscriberPreset};
    use speech::language_model::LanguageModelConfiguration;
    use speech::recognizer::SpeechRecognizer;
    use speech::request::UrlRecognitionRequest;
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
        use speech::error::SpeechError;

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
}
