// Speech framework bridge — SFSpeechRecognizer for on-device transcription.
//
// Apple's Speech framework requires user authorization
// (NSSpeechRecognitionUsageDescription in Info.plist + an authorization
// request). Daemons / CLI binaries without a proper bundle typically get
// .denied. We expose authorization status + a synchronous file-recognition
// path so consumers can degrade gracefully when authorization fails.

import AVFoundation
import Foundation
import Speech

// MARK: - Status codes (mirrored in src/error.rs)

private let SP_OK: Int32 = 0
private let SP_INVALID_ARGUMENT: Int32 = -1
private let SP_NOT_AUTHORIZED: Int32 = -2
private let SP_RECOGNIZER_UNAVAILABLE: Int32 = -3
private let SP_AUDIO_LOAD_FAILED: Int32 = -4
private let SP_RECOGNITION_FAILED: Int32 = -5
private let SP_TIMED_OUT: Int32 = -6
private let SP_UNKNOWN: Int32 = -99

// MARK: - String helpers

@_cdecl("sp_string_free")
public func sp_string_free(_ str: UnsafeMutablePointer<CChar>?) {
    guard let str = str else { return }
    free(str)
}

private func ffiString(_ s: String) -> UnsafeMutablePointer<CChar>? {
    return s.withCString { strdup($0) }
}

// MARK: - FFI Result Types

/// One transcription segment. Layout-compatible with `TranscriptionSegmentRaw`
/// in src/ffi/mod.rs.
@frozen
public struct SPTranscriptionSegmentRaw {
    public var text: UnsafeMutablePointer<CChar>?
    public var confidence: Float
    /// Timestamp (seconds) in the audio.
    public var timestamp: Double
    /// Duration (seconds).
    public var duration: Double
}

// MARK: - Authorization

/// Returns the current authorisation status:
///   0 = not determined, 1 = denied, 2 = restricted, 3 = authorized.
@_cdecl("sp_authorization_status")
public func sp_authorization_status() -> Int32 {
    switch SFSpeechRecognizer.authorizationStatus() {
    case .notDetermined: return 0
    case .denied: return 1
    case .restricted: return 2
    case .authorized: return 3
    @unknown default: return -1
    }
}

/// Synchronously requests authorisation (blocks until the user responds or
/// the system grants automatically). Returns the resulting status code.
@_cdecl("sp_request_authorization")
public func sp_request_authorization() -> Int32 {
    let semaphore = DispatchSemaphore(value: 0)
    var result: SFSpeechRecognizerAuthorizationStatus = .notDetermined
    SFSpeechRecognizer.requestAuthorization { status in
        result = status
        semaphore.signal()
    }
    // 30s timeout — if the system doesn't respond, return what we have.
    _ = semaphore.wait(timeout: .now() + .seconds(30))
    switch result {
    case .notDetermined: return 0
    case .denied: return 1
    case .restricted: return 2
    case .authorized: return 3
    @unknown default: return -1
    }
}

// MARK: - Recognizer availability

@_cdecl("sp_recognizer_is_available")
public func sp_recognizer_is_available(_ localeId: UnsafePointer<CChar>?) -> Bool {
    let recognizer: SFSpeechRecognizer?
    if let localeId = localeId {
        let str = String(cString: localeId)
        recognizer = SFSpeechRecognizer(locale: Locale(identifier: str))
    } else {
        recognizer = SFSpeechRecognizer()
    }
    return recognizer?.isAvailable ?? false
}

@_cdecl("sp_recognizer_default_locale_identifier")
public func sp_recognizer_default_locale_identifier() -> UnsafeMutablePointer<CChar>? {
    guard let recognizer = SFSpeechRecognizer() else { return nil }
    return ffiString(recognizer.locale.identifier)
}

// MARK: - File recognition (synchronous)

/// Recognise speech in the audio file at `audioPath` using the recogniser
/// for `localeId` (or the default if NULL).
///
/// Writes the full transcript to `outTranscript` (heap-allocated, free with
/// `sp_string_free`) and the per-segment array to `outSegments` /
/// `outSegmentCount` (free segments with `sp_transcription_segments_free`).
///
/// Blocks until the recogniser fires its final result or 60s elapses.
/// Returns 0 on success or a negative status code on failure with an
/// optional `outErrorMessage`.
@_cdecl("sp_recognize_url")
public func sp_recognize_url(
    _ audioPath: UnsafePointer<CChar>,
    _ localeId: UnsafePointer<CChar>?,
    _ outTranscript: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
    _ outSegments: UnsafeMutablePointer<UnsafeMutableRawPointer?>,
    _ outSegmentCount: UnsafeMutablePointer<Int>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let pathStr = String(cString: audioPath)

    let authStatus = SFSpeechRecognizer.authorizationStatus()
    if authStatus != .authorized {
        outErrorMessage?.pointee = ffiString(
            "Speech recognition not authorized (status=\(authStatus.rawValue)). " +
            "Add NSSpeechRecognitionUsageDescription to your Info.plist and " +
            "call requestAuthorization()."
        )
        return SP_NOT_AUTHORIZED
    }

    let recognizer: SFSpeechRecognizer?
    if let localeId = localeId {
        let str = String(cString: localeId)
        recognizer = SFSpeechRecognizer(locale: Locale(identifier: str))
    } else {
        recognizer = SFSpeechRecognizer()
    }
    guard let recognizer = recognizer, recognizer.isAvailable else {
        outErrorMessage?.pointee = ffiString("recognizer is unavailable for this locale")
        return SP_RECOGNIZER_UNAVAILABLE
    }

    let url = URL(fileURLWithPath: pathStr)
    if !FileManager.default.fileExists(atPath: pathStr) {
        outErrorMessage?.pointee = ffiString("audio file does not exist: \(pathStr)")
        return SP_AUDIO_LOAD_FAILED
    }

    let request = SFSpeechURLRecognitionRequest(url: url)
    request.shouldReportPartialResults = false
    request.requiresOnDeviceRecognition = true

    let semaphore = DispatchSemaphore(value: 0)
    var finalResult: SFSpeechRecognitionResult?
    var finalError: Error?

    let task = recognizer.recognitionTask(with: request) { result, error in
        if let error = error {
            finalError = error
            semaphore.signal()
            return
        }
        if let result = result, result.isFinal {
            finalResult = result
            semaphore.signal()
        }
    }

    let waited = semaphore.wait(timeout: .now() + .seconds(60))
    if waited == .timedOut {
        task.cancel()
        outErrorMessage?.pointee = ffiString("recognition timed out after 60s")
        return SP_TIMED_OUT
    }

    if let error = finalError {
        outErrorMessage?.pointee = ffiString("recognition failed: \(error.localizedDescription)")
        return SP_RECOGNITION_FAILED
    }
    guard let result = finalResult else {
        outErrorMessage?.pointee = ffiString("recognition produced no result")
        return SP_RECOGNITION_FAILED
    }

    let transcription = result.bestTranscription
    outTranscript.pointee = ffiString(transcription.formattedString)

    let segments = transcription.segments
    let count = segments.count
    if count == 0 {
        outSegments.pointee = nil
        outSegmentCount.pointee = 0
        return SP_OK
    }
    let buffer = UnsafeMutablePointer<SPTranscriptionSegmentRaw>.allocate(capacity: count)
    for (i, seg) in segments.enumerated() {
        buffer.advanced(by: i).initialize(to: SPTranscriptionSegmentRaw(
            text: ffiString(seg.substring),
            confidence: seg.confidence,
            timestamp: seg.timestamp,
            duration: seg.duration
        ))
    }
    outSegments.pointee = UnsafeMutableRawPointer(buffer)
    outSegmentCount.pointee = count
    return SP_OK
}

@_cdecl("sp_transcription_segments_free")
public func sp_transcription_segments_free(_ array: UnsafeMutableRawPointer?, _ count: Int) {
    guard let array = array else { return }
    let typed = array.assumingMemoryBound(to: SPTranscriptionSegmentRaw.self)
    for i in 0..<count {
        if let text = typed.advanced(by: i).pointee.text {
            free(text)
        }
    }
    typed.deallocate()
}
