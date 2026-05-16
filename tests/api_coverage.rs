//! API-surface coverage harness for `speech`.
//!
//! Parses Apple's Obj-C headers under `Speech.framework/Headers/` and
//! verifies the public Speech symbols we claim to support are referenced from
//! our Swift bridge sources. Constructor spelling differences (`initWithURL:`
//! vs `init(url:)`) and purely Rust-side stored configuration fields are
//! handled via alias mappings or targeted omissions.

#![allow(clippy::cast_precision_loss)]

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn sdk_root() -> PathBuf {
    let out = Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
        .expect("xcrun");
    assert!(out.status.success());
    PathBuf::from(String::from_utf8(out.stdout).unwrap().trim().to_string())
}

fn read(path: &PathBuf) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn read_bridge() -> String {
    let bridge_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("swift-bridge/Sources/SpeechBridge");
    let mut files = std::fs::read_dir(&bridge_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "swift"))
        .collect::<Vec<_>>();
    files.sort();
    files
        .into_iter()
        .map(|path| read(&path))
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_header(name: &str) -> String {
    read(&sdk_root().join(format!(
        "System/Library/Frameworks/Speech.framework/Headers/{name}.h"
    )))
}

fn read_swiftinterface() -> String {
    let module_dir = sdk_root()
        .join("System/Library/Frameworks/Speech.framework/Versions/A/Modules/Speech.swiftmodule");
    let mut files = std::fs::read_dir(&module_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "swiftinterface"))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains("apple-macos"))
        })
        .collect::<Vec<_>>();
    files.sort();
    read(files.first().expect("Speech macOS swiftinterface"))
}

/// Extract the `@interface TypeName ... @end` block (no protocol blocks).
fn extract_interface(header: &str, type_name: &str) -> String {
    let needle = regex_lite::Regex::new(&format!(r"@interface\s+{type_name}\b")).unwrap();
    let Some(start) = needle.find(header) else {
        return String::new();
    };
    let rest = &header[start.start()..];
    let Some(end_off) = rest.find("@end") else {
        return rest.to_string();
    };
    rest[..end_off].to_string()
}

/// Extract method + property names from an Obj-C interface body.
fn extract_member_surface(interface_body: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();

    let method_re =
        regex_lite::Regex::new(r"(?m)^\s*[+\-]\s*\([^\)]*\)\s*([A-Za-z_][A-Za-z0-9_]*)").unwrap();
    for c in method_re.captures_iter(interface_body) {
        out.insert(c[1].to_string());
    }

    let prop_re = regex_lite::Regex::new(
        r"(?m)^\s*@property\s*(?:\([^\)]*\))?\s*[^;]*?\b([A-Za-z_][A-Za-z0-9_]*)\s*(?:NS_|API_|;)",
    )
    .unwrap();
    for c in prop_re.captures_iter(interface_body) {
        out.insert(c[1].to_string());
    }

    let getter_re = regex_lite::Regex::new(r"getter\s*=\s*([A-Za-z_][A-Za-z0-9_]*)").unwrap();
    for c in getter_re.captures_iter(interface_body) {
        out.insert(c[1].to_string());
    }

    out
}

fn extra_patterns(name: &str) -> Vec<String> {
    match name {
        "available" => vec![r"\bisAvailable\b".into()],
        "final" => vec![r"\bisFinal\b".into()],
        "cancelled" => vec![r"\bisCancelled\b".into()],
        "finishing" => vec![r"\bisFinishing\b".into()],
        "initWithLocale" => vec![r"SFSpeechRecognizer\(locale:".into()],
        "initWithURL" => vec![r"SFSpeechURLRecognitionRequest\(url:".into()],
        "initWithLanguageModel" => vec![r"Configuration\(languageModel:".into()],
        "appendAudioPCMBuffer" => vec![r"\bappend\(".into()],
        "prepareCustomLanguageModelForUrl" => vec![r"\bprepareCustomLanguageModel\(".into()],
        "recognitionTaskWithRequest" => vec![r"\brecognitionTask\(".into()],
        _ => Vec::new(),
    }
}

fn references_in_bridge(symbols: &BTreeSet<String>) -> BTreeSet<String> {
    let bridge = read_bridge();
    symbols
        .iter()
        .filter(|name| {
            let default_pattern = format!(r"\b{}\b", regex_lite::escape(name));
            std::iter::once(default_pattern)
                .chain(extra_patterns(name))
                .any(|pattern| regex_lite::Regex::new(&pattern).unwrap().is_match(&bridge))
        })
        .cloned()
        .collect()
}

fn report(
    name: &str,
    apple: &BTreeSet<String>,
    ours: &BTreeSet<String>,
    omitted: &BTreeSet<String>,
) {
    let wrapped: BTreeSet<&String> = apple.intersection(ours).collect();
    let missing: BTreeSet<&String> = apple
        .difference(ours)
        .filter(|s| !omitted.contains(*s))
        .collect();
    let coverable = wrapped.len() + missing.len();
    let pct = if coverable == 0 {
        100.0
    } else {
        wrapped.len() as f64 / coverable as f64 * 100.0
    };
    println!(
        "\n=== {name} ===\n  apple={}, omitted={}, coverable={coverable}, wrapped={}, missing={}, pct={pct:.1}%",
        apple.len(),
        omitted.len(),
        wrapped.len(),
        missing.len(),
    );
    if !missing.is_empty() {
        for s in &missing {
            println!("  - {s}");
        }
    }
    assert!(pct >= 100.0, "{name}: {pct:.1}%");
}

fn omitted_set<const N: usize>(items: [&str; N]) -> BTreeSet<String> {
    items.into_iter().map(String::from).collect()
}

#[test]
fn sf_speech_recognizer_coverage() {
    let header = read_header("SFSpeechRecognizer");
    let body = extract_interface(&header, "SFSpeechRecognizer");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set(["init"]);
    report("SFSpeechRecognizer", &apple, &ours, &omitted);
}

#[test]
fn sf_speech_recognition_request_base_coverage() {
    let header = read_header("SFSpeechRecognitionRequest");
    let body = extract_interface(&header, "SFSpeechRecognitionRequest");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set::<0>([]);
    report("SFSpeechRecognitionRequest", &apple, &ours, &omitted);
}

#[test]
fn sf_speech_url_recognition_request_coverage() {
    let header = read_header("SFSpeechRecognitionRequest");
    let body = extract_interface(&header, "SFSpeechURLRecognitionRequest");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set(["init", "URL"]);
    report("SFSpeechURLRecognitionRequest", &apple, &ours, &omitted);
}

#[test]
fn sf_speech_audio_buffer_recognition_request_coverage() {
    let header = read_header("SFSpeechRecognitionRequest");
    let body = extract_interface(&header, "SFSpeechAudioBufferRecognitionRequest");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set::<0>([]);
    report(
        "SFSpeechAudioBufferRecognitionRequest",
        &apple,
        &ours,
        &omitted,
    );
}

#[test]
fn sf_speech_recognition_result_coverage() {
    let header = read_header("SFSpeechRecognitionResult");
    let body = extract_interface(&header, "SFSpeechRecognitionResult");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set::<0>([]);
    report("SFSpeechRecognitionResult", &apple, &ours, &omitted);
}

#[test]
fn sf_speech_recognition_metadata_coverage() {
    let header = read_header("SFSpeechRecognitionMetadata");
    let body = extract_interface(&header, "SFSpeechRecognitionMetadata");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set::<0>([]);
    report("SFSpeechRecognitionMetadata", &apple, &ours, &omitted);
}

#[test]
fn sf_transcription_coverage() {
    let header = read_header("SFTranscription");
    let body = extract_interface(&header, "SFTranscription");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set::<0>([]);
    report("SFTranscription", &apple, &ours, &omitted);
}

#[test]
fn sf_transcription_segment_coverage() {
    let header = read_header("SFTranscriptionSegment");
    let body = extract_interface(&header, "SFTranscriptionSegment");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set::<0>([]);
    report("SFTranscriptionSegment", &apple, &ours, &omitted);
}

#[test]
fn sf_speech_recognition_task_coverage() {
    let header = read_header("SFSpeechRecognitionTask");
    let body = extract_interface(&header, "SFSpeechRecognitionTask");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set::<0>([]);
    report("SFSpeechRecognitionTask", &apple, &ours, &omitted);
}

#[test]
fn sf_voice_analytics_coverage() {
    let header = read_header("SFVoiceAnalytics");
    let body = extract_interface(&header, "SFVoiceAnalytics");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set::<0>([]);
    report("SFVoiceAnalytics", &apple, &ours, &omitted);
}

#[test]
fn sf_acoustic_feature_coverage() {
    let header = read_header("SFVoiceAnalytics");
    let body = extract_interface(&header, "SFAcousticFeature");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set::<0>([]);
    report("SFAcousticFeature", &apple, &ours, &omitted);
}

#[test]
fn sf_speech_language_model_configuration_coverage() {
    let header = read_header("SFSpeechLanguageModel");
    let body = extract_interface(&header, "SFSpeechLanguageModelConfiguration");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set(["languageModel", "vocabulary", "weight"]);
    report(
        "SFSpeechLanguageModelConfiguration",
        &apple,
        &ours,
        &omitted,
    );
}

#[test]
fn sf_speech_language_model_coverage() {
    let header = read_header("SFSpeechLanguageModel");
    let body = extract_interface(&header, "SFSpeechLanguageModel");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set::<0>([]);
    report("SFSpeechLanguageModel", &apple, &ours, &omitted);
}

#[test]
fn sf_speech_recognition_task_delegate_coverage() {
    let header = read_header("SFSpeechRecognitionTask");
    let bridge = read_bridge();
    for selector in [
        "speechRecognitionDidDetectSpeech",
        "didHypothesizeTranscription",
        "didFinishRecognition",
        "speechRecognitionTaskFinishedReadingAudio",
        "speechRecognitionTaskWasCancelled",
        "didFinishSuccessfully",
        "didProcessAudioDuration",
    ] {
        assert!(
            header.contains(selector),
            "missing selector in header: {selector}"
        );
    }
    for bridge_token in [
        "didDetectSpeech",
        "didHypothesizeTranscription",
        "didFinishRecognition",
        "finishedReadingAudio",
        "wasCancelled",
        "didFinishSuccessfully",
        "didProcessAudioDuration",
    ] {
        assert!(
            bridge.contains(bridge_token),
            "missing delegate bridge token: {bridge_token}"
        );
    }
}

#[test]
fn dictation_transcriber_coverage() {
    let interface = read_swiftinterface();
    let bridge = read_bridge();

    for sdk_symbol in [
        "final public class DictationTranscriber",
        "public static var supportedLocales",
        "public static var installedLocales",
        "supportedLocale(equivalentTo",
        "final public var selectedLocales",
        "final public var availableCompatibleAudioFormats",
        "final public var results",
        "public struct Result",
        "public let alternatives",
        "public let resultsFinalizationTime",
    ] {
        assert!(
            interface.contains(sdk_symbol),
            "missing dictation SDK symbol: {sdk_symbol}"
        );
    }

    for bridge_symbol in [
        "DictationTranscriber",
        "shortDictation",
        "progressiveShortDictation",
        "longDictation",
        "progressiveLongDictation",
        "timeIndexedLongDictation",
        "supportedLocales",
        "installedLocales",
        "supportedLocale(equivalentTo",
        "selectedLocales",
        "availableCompatibleAudioFormats",
        "resultsFinalizationTime",
        "SPXDictationResultPayload",
        "sp_dictation_transcribe_url_json",
    ] {
        assert!(
            bridge.contains(bridge_symbol),
            "missing dictation bridge symbol: {bridge_symbol}"
        );
    }
}
