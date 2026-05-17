// AsyncThunks.swift — Non-blocking @_cdecl thunks for `async` feature gate.
//
// Each thunk takes a C callback + opaque ctx pointer and launches a Swift
// Task (or bridges a completion-handler API) so the caller is never blocked.
// The callback signature is one of:
//   - SPXAuthAsyncCb  : (Int32, ctx)               — authorization status
//   - SPXStringAsyncCb: (result_cstr?, error_cstr?, ctx) — JSON result or error
//   - SPXVoidAsyncCb  : (error_cstr?, ctx)          — success/failure only

import AVFoundation
import Foundation
import Speech

// MARK: — Callback type aliases (must match Rust FFI declarations)

/// Authorization callback: delivers the raw SFSpeechRecognizerAuthorizationStatus code.
public typealias SPXAuthAsyncCb = @convention(c) (Int32, UnsafeMutableRawPointer) -> Void

/// String-result callback: delivers a JSON result C-string on success, or an
/// error C-string on failure.  Both pointers are valid only for the duration
/// of the callback.
public typealias SPXStringAsyncCb = @convention(c) (
    UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafeMutableRawPointer
) -> Void

/// Void callback: delivers a non-null error C-string on failure, nil on success.
public typealias SPXVoidAsyncCb = @convention(c) (
    UnsafePointer<CChar>?, UnsafeMutableRawPointer
) -> Void

// MARK: — Fire-once helper (prevents continuation double-resume on multi-fire delegates)

private final class SPXFireOnce: @unchecked Sendable {
    private let lock = NSLock()
    private var _fired = false

    /// Returns `true` the first time; `false` on every subsequent call.
    func tryFire() -> Bool {
        lock.lock()
        defer { lock.unlock() }
        guard !_fired else { return false }
        _fired = true
        return true
    }
}

// MARK: — 1. SFSpeechRecognizer.requestAuthorization (completion handler)

/// Async bridge for `SFSpeechRecognizer.requestAuthorization`.
///
/// Calls `cb(statusCode, ctx)` once the system has determined authorization.
/// Status codes mirror `AuthorizationStatus` in Rust:
///   0 = notDetermined, 1 = denied, 2 = restricted, 3 = authorized, -1 = unknown.
@_cdecl("sp_request_authorization_async")
public func sp_request_authorization_async(
    _ cb: @escaping SPXAuthAsyncCb,
    _ ctx: UnsafeMutableRawPointer
) {
    SFSpeechRecognizer.requestAuthorization { status in
        let code: Int32
        switch status {
        case .notDetermined: code = 0
        case .denied:        code = 1
        case .restricted:    code = 2
        case .authorized:    code = 3
        @unknown default:    code = -1
        }
        cb(code, ctx)
    }
}

// MARK: — 2. One-shot URL recognition (SFSpeechRecognitionTask, fires once with final result)

/// Async bridge for `SFSpeechRecognizer.recognitionTask(with:resultHandler:)`.
///
/// Launches recognition in a detached Swift Task; calls `cb` exactly once:
/// - `cb(json_cstr, nil, ctx)` on success (JSON matches `DetailedRecognitionResult`)
/// - `cb(nil, error_cstr, ctx)` on failure
@_cdecl("sp_recognize_url_async")
public func sp_recognize_url_async(
    _ audioPath: UnsafePointer<CChar>,
    _ localeId: UnsafePointer<CChar>?,
    _ recognizerJson: UnsafePointer<CChar>?,
    _ requestJson: UnsafePointer<CChar>?,
    _ cb: @escaping SPXStringAsyncCb,
    _ ctx: UnsafeMutableRawPointer
) {
    Task.detached {
        do {
            let path = String(cString: audioPath)
            let recognizerPayload = try spxDecodeJSONIfPresent(
                recognizerJson, as: SPXRecognizerPayload.self)
            let requestPayload = try spxDecodeJSONIfPresent(
                requestJson, as: SPXRequestPayload.self)

            let recognizer = try spxCreateRecognizer(
                localeId: localeId, recognizerPayload: recognizerPayload)
            guard recognizer.isAvailable else {
                throw SPXBridgeError.recognizerUnavailable(
                    "recognizer is unavailable for this locale")
            }

            let request = SFSpeechURLRecognitionRequest(url: URL(fileURLWithPath: path))
            try spxApplyRequestPayload(
                requestPayload, recognizerPayload: recognizerPayload, to: request)

            let once = SPXFireOnce()
            let finalResult: SFSpeechRecognitionResult =
                try await withCheckedThrowingContinuation { cont in
                    let task = recognizer.recognitionTask(with: request) { result, error in
                        guard once.tryFire() else { return }
                        if let error {
                            cont.resume(throwing: SPXBridgeError.framework(error as NSError))
                            return
                        }
                        if let result, result.isFinal {
                            cont.resume(returning: result)
                        } else if result == nil {
                            cont.resume(
                                throwing: SPXBridgeError.recognitionFailed(
                                    "recognition produced no final result"))
                        }
                        // partial results: ignore (once guards against double-resume)
                    }
                    _ = task  // retain reference
                }

            let json = try spxEncodeJSON(spxEncodeRecognitionResult(finalResult))
            json.withCString { ptr in cb(ptr, nil, ctx) }
        } catch let err as SPXBridgeError {
            err.description.withCString { ptr in cb(nil, ptr, ctx) }
        } catch {
            error.localizedDescription.withCString { ptr in cb(nil, ptr, ctx) }
        }
    }
}

// MARK: — 3. SpeechAnalyzer.analyze (macOS 26.0+, natively async throws)

/// Async bridge for `SpeechAnalyzer` / `SpeechTranscriber` (macOS 26.0+).
///
/// On older OS versions the error callback is fired immediately with a
/// "requires macOS 26" message.
///
/// On macOS 26+: calls `cb(json_cstr, nil, ctx)` on success (JSON matches
/// `SpeechAnalyzerOutput`), or `cb(nil, error_cstr, ctx)` on failure.
@_cdecl("sp_speech_analyzer_analyze_url_async")
public func sp_speech_analyzer_analyze_url_async(
    _ audioPath: UnsafePointer<CChar>,
    _ analyzerJson: UnsafePointer<CChar>,
    _ cb: @escaping SPXStringAsyncCb,
    _ ctx: UnsafeMutableRawPointer
) {
    Task.detached {
        do {
            #if SPEECH_HAS_MACOS26_SDK
            if #available(macOS 26.0, *) {
                let path = String(cString: audioPath)
                let payload = try spxDecodeJSON(analyzerJson, as: SPXSpeechAnalyzerPayload.self)
                let json = try await spxRunSpeechAnalyzer(audioPath: path, payload: payload)
                json.withCString { ptr in cb(ptr, nil, ctx) }
                return
            }
            #endif
            throw SPXBridgeError.recognizerUnavailable(spxSpeechAnalyzerUnavailableMessage())
        } catch let err as SPXBridgeError {
            err.description.withCString { ptr in cb(nil, ptr, ctx) }
        } catch {
            error.localizedDescription.withCString { ptr in cb(nil, ptr, ctx) }
        }
    }
}

// MARK: — 4. SFSpeechLanguageModel.prepareCustomLanguageModel (completion handler)

/// Async bridge for `SFSpeechLanguageModel.prepareCustomLanguageModel` (macOS 14.0+).
///
/// Calls `cb(nil, ctx)` on success, or `cb(error_cstr, ctx)` on failure.
@_cdecl("sp_prepare_custom_language_model_async")
public func sp_prepare_custom_language_model_async(
    _ assetPath: UnsafePointer<CChar>,
    _ configurationJson: UnsafePointer<CChar>,
    _ ignoresCache: Bool,
    _ cb: @escaping SPXVoidAsyncCb,
    _ ctx: UnsafeMutableRawPointer
) {
    Task.detached {
        do {
            if #available(macOS 14.0, *) {
                let path = String(cString: assetPath)
                guard FileManager.default.fileExists(atPath: path) else {
                    throw SPXBridgeError.audioLoadFailed(
                        "custom language model asset does not exist: \(path)")
                }

                let configPayload = try spxDecodeJSON(
                    configurationJson, as: SPXLanguageModelConfigurationPayload.self)
                let configuration = try spxMakeLanguageModelConfiguration(from: configPayload)
                let assetURL = URL(fileURLWithPath: path)

                try await withCheckedThrowingContinuation {
                    (cont: CheckedContinuation<Void, Error>) in
                    let handler: (Error?) -> Void = { error in
                        if let error {
                            cont.resume(throwing: SPXBridgeError.framework(error as NSError))
                        } else {
                            cont.resume()
                        }
                    }
                    // Apple split the API on macOS 26: the older form requires
                    // `clientIdentifier:`, the newer system-managed form drops
                    // it. We use the older form unconditionally — it stays
                    // available on every supported macOS, it's just marked
                    // deprecated on 26+ (the system-managed variant doesn't
                    // exist on macOS 14/15 runners).
                    let bundleIdentifier =
                        Bundle.main.bundleIdentifier ?? "doom-fish.speech-rs.bridge"
                    if ignoresCache {
                        SFSpeechLanguageModel.prepareCustomLanguageModel(
                            for: assetURL, clientIdentifier: bundleIdentifier,
                            configuration: configuration, ignoresCache: true,
                            completion: handler)
                    } else {
                        SFSpeechLanguageModel.prepareCustomLanguageModel(
                            for: assetURL, clientIdentifier: bundleIdentifier,
                            configuration: configuration,
                            completion: handler)
                    }
                }
                cb(nil, ctx)
            } else {
                throw SPXBridgeError.unavailableOnThisMacOS(
                    "custom language models require macOS 14+")
            }
        } catch let err as SPXBridgeError {
            err.description.withCString { ptr in cb(ptr, ctx) }
        } catch {
            error.localizedDescription.withCString { ptr in cb(ptr, ctx) }
        }
    }
}
