use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

use speech::{LiveRecognition, SpeechRecognizer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize German speech recognition
    let recognizer = SpeechRecognizer::new();

    println!("Recognizer available: {}", recognizer.is_available());

    if !recognizer.is_available() {
        return Err("Speech recognizer is unavailable".into());
    }

    println!("🎙️ Listening ...");
    println!("Press Enter to stop.\n");

    // Start microphone capture and live transcription
    let session = LiveRecognition::start_accumulated(&recognizer, |update| {
        if update.is_final {
            println!("\n[FINAL] {}", update.transcript);
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

    // Give Apple time to deliver the final result.
    thread::sleep(Duration::from_secs(3));

    Ok(())
}
