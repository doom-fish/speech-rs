use speech::prelude::*;

#[test]
fn a_missing_audio_file_is_reported_as_an_audio_load_failure() {
    let recognizer =
        SpeechRecognizer::with_locale("en-US").expect("en-US is a valid locale identifier");
    let request = UrlRecognitionRequest::new("/nonexistent/speech-rs-task-start.m4a");
    let result = recognizer.start_url_task(&request, |_| {});
    let error = result.err();
    assert!(
        matches!(error, Some(SpeechError::AudioLoadFailed(_))),
        "unexpected result: {error:?}"
    );
}

#[test]
fn an_audio_buffer_task_requires_authorization() {
    let recognizer = SpeechRecognizer::with_locale("en-US")
        .expect("en-US is a valid locale identifier")
        .with_callback_queue(CallbackQueue::background());
    let result = recognizer.start_audio_buffer_task(&AudioBufferRecognitionRequest::new(), |_| {});
    let error = result.err();
    if SpeechRecognizer::authorization_status().is_authorized() {
        assert!(
            matches!(error, None | Some(SpeechError::RecognizerUnavailable(_))),
            "unexpected result: {error:?}"
        );
    } else {
        assert!(
            matches!(error, Some(SpeechError::NotAuthorized(_))),
            "unexpected result: {error:?}"
        );
    }
}

#[test]
fn live_recognition_requires_authorization_before_opening_the_microphone() {
    if SpeechRecognizer::authorization_status().is_authorized() {
        println!("skipping: starting live recognition here would open the microphone");
        return;
    }
    let recognizer =
        SpeechRecognizer::with_locale("en-US").expect("en-US is a valid locale identifier");
    let error = LiveRecognition::start(&recognizer, |_| {}).err();
    assert!(
        matches!(error, Some(SpeechError::NotAuthorized(_))),
        "unexpected result: {error:?}"
    );
}

#[test]
fn live_recognition_honors_the_recognizer_callback_queue() {
    let recognizer = SpeechRecognizer::new()
        .with_callback_queue(CallbackQueue::background().with_max_concurrent_operations(0));
    let error = LiveRecognition::start(&recognizer, |_| {}).err();
    assert!(
        matches!(error, Some(SpeechError::InvalidArgument(_))),
        "unexpected result: {error:?}"
    );
}

#[test]
fn audio_buffer_task_controls_can_run_on_several_threads_at_once() {
    let recognizer =
        SpeechRecognizer::with_locale("en-US").expect("en-US is a valid locale identifier");
    let Ok(task) =
        recognizer.start_audio_buffer_task(&AudioBufferRecognitionRequest::new(), |_| {})
    else {
        println!("skipping: an audio-buffer task cannot start in this environment");
        return;
    };

    std::thread::scope(|scope| {
        for round in 0..8 {
            let task = &task;
            scope.spawn(move || match round % 3 {
                0 => task.end_audio(),
                1 => task.finish(),
                _ => task.cancel(),
            });
        }
    });

    let state = task.state();
    assert!(
        matches!(
            state,
            TaskState::Finishing | TaskState::Canceling | TaskState::Completed
        ),
        "the task ignored finish and cancel: {state:?}"
    );
}
