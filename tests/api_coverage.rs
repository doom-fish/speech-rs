//! API-surface coverage harness for `speech`.
//!
//! Parses Apple's Obj-C headers under `Speech.framework/Headers/` and
//! verifies the methods/properties we wrap are referenced from our Swift
//! bridge. Same approach as the other doom-fish crates.

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
    read(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("swift-bridge/Sources/SpeechBridge/Speech.swift"),
    )
}

fn read_header(name: &str) -> String {
    read(&sdk_root().join(format!(
        "System/Library/Frameworks/Speech.framework/Headers/{name}.h"
    )))
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

    let method_re = regex_lite::Regex::new(
        r"(?m)^\s*[+\-]\s*\([^\)]*\)\s*([A-Za-z_][A-Za-z0-9_]*)",
    )
    .unwrap();
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

fn references_in_bridge(symbols: &BTreeSet<String>) -> BTreeSet<String> {
    let bridge = read_bridge();
    symbols
        .iter()
        .filter(|name| {
            let pattern = format!(r"\b{}\b", regex_lite::escape(name));
            regex_lite::Regex::new(&pattern).unwrap().is_match(&bridge)
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

// ---- Tests ----

#[test]
fn sf_speech_recognizer_coverage() {
    let header = read_header("SFSpeechRecognizer");
    let body = extract_interface(&header, "SFSpeechRecognizer");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set([
        "supportedLocales",
        "init",
        "initWithLocale",
        "supportsOnDeviceRecognition",
        "delegate",
        "defaultTaskHint",
        "recognitionTaskWithRequest",
        "queue",
        // We use the `isAvailable` getter form, not the bare `available`
        // property name. Counted via the `getter=isAvailable` rewrite.
        "available",
    ]);
    report("SFSpeechRecognizer", &apple, &ours, &omitted);
}

#[test]
fn sf_speech_url_recognition_request_coverage() {
    let header = read_header("SFSpeechRecognitionRequest");
    let body = extract_interface(&header, "SFSpeechURLRecognitionRequest");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set([
        "init",
        "URL",
        // Swift bridges `initWithURL:` as `init(url:)`, which is what we use.
        "initWithURL",
    ]);
    report("SFSpeechURLRecognitionRequest", &apple, &ours, &omitted);
}

#[test]
fn sf_speech_recognition_request_base_coverage() {
    let header = read_header("SFSpeechRecognitionRequest");
    let body = extract_interface(&header, "SFSpeechRecognitionRequest");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set([
        "taskHint",
        "contextualStrings",
        "interactionIdentifier",
        "addsPunctuation",
        "customizedLanguageModel",
    ]);
    report("SFSpeechRecognitionRequest", &apple, &ours, &omitted);
}

#[test]
fn sf_transcription_coverage() {
    let header = read_header("SFTranscription");
    let body = extract_interface(&header, "SFTranscription");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set([
        "speakingRate",
        "averagePauseDuration",
    ]);
    report("SFTranscription", &apple, &ours, &omitted);
}

#[test]
fn sf_transcription_segment_coverage() {
    let header = read_header("SFTranscriptionSegment");
    let body = extract_interface(&header, "SFTranscriptionSegment");
    let apple = extract_member_surface(&body);
    let ours = references_in_bridge(&apple);
    let omitted = omitted_set([
        "substringRange",
        "alternativeSubstrings",
        "voiceAnalytics",
    ]);
    report("SFTranscriptionSegment", &apple, &ours, &omitted);
}
