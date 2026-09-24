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

// MARK: - Authorization

/// Returns the current authorisation status:
///   0 = not determined, 1 = denied, 2 = restricted, 3 = authorized.
private func spAuthorizationCode(_ status: SFSpeechRecognizerAuthorizationStatus) -> Int32 {
    switch status {
    case .notDetermined: return 0
    case .denied: return 1
    case .restricted: return 2
    case .authorized: return 3
    @unknown default: return -1
    }
}

@_cdecl("sp_authorization_status")
public func sp_authorization_status() -> Int32 {
    spAuthorizationCode(SFSpeechRecognizer.authorizationStatus())
}

/// Synchronously requests authorisation (blocks until the user responds or
/// the system grants automatically) and writes the resulting status code to
/// `outStatus`. Returns SP_TIMED_OUT if there is no answer within 30s.
@_cdecl("sp_request_authorization")
public func sp_request_authorization(
    _ outStatus: UnsafeMutablePointer<Int32>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let semaphore = DispatchSemaphore(value: 0)
    let answer = SPXAsyncBridgeState<Int32>()
    SFSpeechRecognizer.requestAuthorization { status in
        answer.store(.success(spAuthorizationCode(status)))
        semaphore.signal()
    }
    guard semaphore.wait(timeout: .now() + .seconds(30)) == .success,
        case .success(let code)? = answer.load()
    else {
        outErrorMessage?.pointee = ffiString(
            "the authorization request got no answer within 30s and is still pending")
        return SP_TIMED_OUT
    }
    outStatus?.pointee = code
    return SP_OK
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

// MARK: - v0.2: Live audio-buffer streaming

/// Result handler for the live streaming API. Called with each partial
/// transcript as Apple emits it, plus a final call with isFinal=true.
///
/// All pointers are temporary — copy the text if you need to keep it.
public typealias SPStreamCallback = @convention(c) (
    UnsafeMutableRawPointer?,           // user_info
    UnsafePointer<CChar>?,              // transcript (NUL-terminated, transient)
    Bool                                 // is_final
) -> Void

private final class SPLiveResultRelay {
    private let callback: SPStreamCallback
    private let ctxRetain: SPContextRefCallback
    private let ctxRelease: SPContextRefCallback
    private let lock = NSLock()
    private var userInfo: UnsafeMutableRawPointer?

    init(
        callback: @escaping SPStreamCallback,
        userInfo: UnsafeMutableRawPointer?,
        ctxRetain: @escaping SPContextRefCallback,
        ctxRelease: @escaping SPContextRefCallback
    ) {
        self.callback = callback
        self.ctxRetain = ctxRetain
        self.ctxRelease = ctxRelease
        self.userInfo = userInfo
        ctxRetain(userInfo)
    }

    deinit {
        if let userInfo {
            ctxRelease(userInfo)
        }
    }

    func deliver(_ transcript: String, isFinal: Bool) {
        lock.lock()
        guard let context = userInfo else {
            lock.unlock()
            return
        }
        if isFinal {
            userInfo = nil
        } else {
            ctxRetain(context)
        }
        lock.unlock()
        transcript.withCString { ptr in
            callback(context, ptr, isFinal)
        }
        ctxRelease(context)
    }
}

private final class LiveSession {
    let recognizer: SFSpeechRecognizer
    let request: SFSpeechAudioBufferRecognitionRequest
    let audioEngine: AVAudioEngine
    private let lock = NSLock()
    private var task: SFSpeechRecognitionTask?

    init(recognizer: SFSpeechRecognizer) {
        self.recognizer = recognizer
        self.request = SFSpeechAudioBufferRecognitionRequest()
        self.request.shouldReportPartialResults = true
        self.request.requiresOnDeviceRecognition = true
        self.audioEngine = AVAudioEngine()
    }

    func start(relay: SPLiveResultRelay) throws {
        lock.lock()
        defer { lock.unlock() }

        // Install a tap on the input node to feed audio buffers into the request.
        let inputNode = audioEngine.inputNode
        let format = inputNode.outputFormat(forBus: 0)
        guard format.channelCount > 0, format.sampleRate > 0 else {
            throw SPXBridgeError.audioLoadFailed("no audio input device is available")
        }
        let request = self.request
        inputNode.installTap(onBus: 0, bufferSize: 1024, format: format) { buffer, _ in
            request.append(buffer)
        }

        audioEngine.prepare()
        do {
            try audioEngine.start()
        } catch {
            inputNode.removeTap(onBus: 0)
            throw SPXBridgeError.audioLoadFailed(
                "audio engine start failed: \(error.localizedDescription)")
        }

        task = recognizer.recognitionTask(with: request) { result, error in
            if let error {
                relay.deliver("error: \(error.localizedDescription)", isFinal: true)
                return
            }
            guard let result else { return }
            relay.deliver(result.bestTranscription.formattedString, isFinal: result.isFinal)
        }
    }

    private func stopAudioEngine() {
        audioEngine.stop()
        audioEngine.inputNode.removeTap(onBus: 0)
    }

    func endAudio() {
        lock.lock()
        defer { lock.unlock() }
        stopAudioEngine()
        request.endAudio()
    }

    func cancel() {
        lock.lock()
        defer { lock.unlock() }
        stopAudioEngine()
        task?.cancel()
    }

    func stop() {
        lock.lock()
        defer { lock.unlock() }
        stopAudioEngine()
        request.endAudio()
        task?.cancel()
    }
}

private let liveSessionsLock = NSLock()
private var liveSessions: [UnsafeMutableRawPointer: LiveSession] = [:]

private func spxLiveSession(_ token: UnsafeMutableRawPointer?) -> LiveSession? {
    guard let token else { return nil }
    liveSessionsLock.lock()
    defer { liveSessionsLock.unlock() }
    return liveSessions[token]
}

/// Start a live audio-buffer recognition session. Returns an opaque
/// token (never NULL on success) that you pass back to
/// `sp_live_recognition_stop`.
@_cdecl("sp_live_recognition_start")
public func sp_live_recognition_start(
    _ localeId: UnsafePointer<CChar>?,
    _ recognizerJson: UnsafePointer<CChar>?,
    _ callback: @escaping SPStreamCallback,
    _ userInfo: UnsafeMutableRawPointer?,
    _ ctxRetain: @escaping SPContextRefCallback,
    _ ctxRelease: @escaping SPContextRefCallback,
    _ outStatus: UnsafeMutablePointer<Int32>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    do {
        try spxEnsureAuthorized()
        let recognizerPayload = try spxDecodeJSONIfPresent(
            recognizerJson, as: SPXRecognizerPayload.self)
        let recognizer = try spxCreateRecognizer(
            localeId: localeId, recognizerPayload: recognizerPayload)
        guard recognizer.isAvailable else {
            throw SPXBridgeError.recognizerUnavailable("recognizer is unavailable for this locale")
        }
        let session = LiveSession(recognizer: recognizer)
        try session.start(
            relay: SPLiveResultRelay(
                callback: callback, userInfo: userInfo, ctxRetain: ctxRetain,
                ctxRelease: ctxRelease))
        let token = Unmanaged.passRetained(session).toOpaque()
        liveSessionsLock.lock()
        liveSessions[token] = session
        liveSessionsLock.unlock()
        return token
    } catch let error as SPXBridgeError {
        outStatus?.pointee = error.statusCode
        outErrorMessage?.pointee = ffiString(error.description)
        return nil
    } catch {
        outStatus?.pointee = SP_UNKNOWN
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

/// Stop a live recognition session started by `sp_live_recognition_start`.
/// Safe to call multiple times.
@_cdecl("sp_live_recognition_stop")
public func sp_live_recognition_stop(_ token: UnsafeMutableRawPointer?) {
    guard let token else { return }
    liveSessionsLock.lock()
    let session = liveSessions.removeValue(forKey: token)
    liveSessionsLock.unlock()
    guard let session else { return }
    session.stop()
    Unmanaged<LiveSession>.fromOpaque(token).release()
}

/// End the audio stream cleanly (`SFSpeechAudioBufferRecognitionRequest.endAudio()`)
/// without cancelling the underlying task — pending audio gets
/// finalised and the callback fires one last time with `is_final=true`.
/// The session token remains valid; you still need to call
/// `sp_live_recognition_stop` afterwards to release resources.
@_cdecl("sp_live_recognition_end_audio")
public func sp_live_recognition_end_audio(_ token: UnsafeMutableRawPointer?) {
    spxLiveSession(token)?.endAudio()
}

/// Cancel the recognition task immediately and discard any in-flight
/// audio. The session token remains valid; you still need to call
/// `sp_live_recognition_stop` afterwards to release resources.
@_cdecl("sp_live_recognition_cancel")
public func sp_live_recognition_cancel(_ token: UnsafeMutableRawPointer?) {
    spxLiveSession(token)?.cancel()
}

@_cdecl("sp_live_result_relay_exercise")
public func sp_live_result_relay_exercise(
    _ callback: @escaping SPStreamCallback,
    _ userInfo: UnsafeMutableRawPointer?,
    _ ctxRetain: @escaping SPContextRefCallback,
    _ ctxRelease: @escaping SPContextRefCallback,
    _ finalFlags: UnsafePointer<Bool>?,
    _ count: Int
) {
    let relay = SPLiveResultRelay(
        callback: callback, userInfo: userInfo, ctxRetain: ctxRetain, ctxRelease: ctxRelease)
    guard let finalFlags, count > 0 else { return }
    for index in 0..<count {
        relay.deliver("update \(index)", isFinal: finalFlags[index])
    }
}
