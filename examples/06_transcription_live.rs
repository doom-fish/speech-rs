use std::{
    io::{self, Write},
    sync::mpsc,
    time::Duration,
};

use speech::{LiveRecognition, SpeechRecognizer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let status = SpeechRecognizer::authorization_status();
    println!("authorization status: {status:?}");
    if !status.is_authorized() {
        let new_status = SpeechRecognizer::request_authorization()?;
        println!("after request: {new_status:?}");
        if !new_status.is_authorized() {
            eprintln!("\nSKIP: speech recognition not authorized for live transcription.");
            return Ok(());
        }
    }

    let recognizer = SpeechRecognizer::new();

    println!("Recognizer available: {}", recognizer.is_available());

    if !recognizer.is_available() {
        return Err("Speech recognizer is unavailable".into());
    }

    println!("🎙️ Listening ...");
    println!("Press Enter to stop.\n");

    let (final_sender, final_receiver) = mpsc::channel();

    // Start microphone capture and live transcription
    let session = LiveRecognition::start_accumulated(&recognizer, move |update| {
        if update.is_final {
            println!("\n[FINAL] {}", update.transcript);
            let _ = final_sender.send(());
        } else {
            println!("\r[PARTIAL] {}", update.transcript);
            let _ = io::stdout().flush();
        }
    })?;

    // Wait until the user presses Enter.
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    println!("\nFinishing audio...");
    session.end_audio();

    let final_update = final_receiver.recv_timeout(Duration::from_secs(10));
    if final_update.is_err() {
        eprintln!("No final result within 10 seconds.");
    }

    Ok(())
}
