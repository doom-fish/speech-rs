import Foundation
import Speech

public typealias SPAvailabilityCallback = @convention(c) (UnsafeMutableRawPointer?, Bool) -> Void

private final class SPAvailabilityObserver: NSObject, SFSpeechRecognizerDelegate {
  let recognizer: SFSpeechRecognizer
  let callback: SPAvailabilityCallback
  let userInfo: UnsafeMutableRawPointer?
  let ctxRelease: SPContextRefCallback
  // Guarded by `activeLock` so a callback delivered by the recognizer cannot
  // race the `deactivate()` performed during teardown.
  private let activeLock = NSLock()
  private var active = true

  init(
    recognizer: SFSpeechRecognizer, callback: @escaping SPAvailabilityCallback,
    userInfo: UnsafeMutableRawPointer?,
    ctxRetain: SPContextRefCallback,
    ctxRelease: @escaping SPContextRefCallback
  ) {
    self.recognizer = recognizer
    self.callback = callback
    self.userInfo = userInfo
    self.ctxRelease = ctxRelease
    super.init()
    // Take a +1 on the Rust callback context for the lifetime of this object so
    // an in-flight availability callback can never observe a freed context.
    ctxRetain(userInfo)
    recognizer.delegate = self
  }

  deinit {
    ctxRelease(userInfo)
  }

  private var isActive: Bool {
    activeLock.lock()
    defer { activeLock.unlock() }
    return active
  }

  func speechRecognizer(
    _ speechRecognizer: SFSpeechRecognizer, availabilityDidChange available: Bool
  ) {
    guard isActive else { return }
    callback(userInfo, available)
  }

  func stop() {
    activeLock.lock()
    active = false
    activeLock.unlock()
    recognizer.delegate = nil
  }
}

@_cdecl("sp_supported_locales_json")
public func sp_supported_locales_json() -> UnsafeMutablePointer<CChar>? {
  let locales = SFSpeechRecognizer.supportedLocales()
    .map(\.identifier)
    .sorted()
  do {
    return spxCString(try spxEncodeJSON(locales))
  } catch {
    return nil
  }
}

@_cdecl("sp_recognizer_locale_identifier")
public func sp_recognizer_locale_identifier(
  _ localeId: UnsafePointer<CChar>?,
  _ recognizerJson: UnsafePointer<CChar>?,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
  do {
    let recognizerPayload = try spxDecodeJSONIfPresent(
      recognizerJson, as: SPXRecognizerPayload.self)
    let recognizer = try spxCreateRecognizer(
      localeId: localeId, recognizerPayload: recognizerPayload)
    return spxCString(recognizer.locale.identifier)
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return nil
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return nil
  }
}

@_cdecl("sp_recognizer_supports_on_device_recognition")
public func sp_recognizer_supports_on_device_recognition(
  _ localeId: UnsafePointer<CChar>?,
  _ recognizerJson: UnsafePointer<CChar>?
) -> Bool {
  let recognizerPayload = try? spxDecodeJSONIfPresent(recognizerJson, as: SPXRecognizerPayload.self)
  let recognizer = try? spxCreateRecognizer(
    localeId: localeId, recognizerPayload: recognizerPayload ?? nil)
  return recognizer?.supportsOnDeviceRecognition ?? false
}

@_cdecl("sp_recognizer_observe_availability")
public func sp_recognizer_observe_availability(
  _ localeId: UnsafePointer<CChar>?,
  _ recognizerJson: UnsafePointer<CChar>?,
  _ callback: @escaping SPAvailabilityCallback,
  _ userInfo: UnsafeMutableRawPointer?,
  _ ctxRetain: SPContextRefCallback,
  _ ctxRelease: @escaping SPContextRefCallback,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
  do {
    let recognizerPayload = try spxDecodeJSONIfPresent(
      recognizerJson, as: SPXRecognizerPayload.self)
    let recognizer = try spxCreateRecognizer(
      localeId: localeId, recognizerPayload: recognizerPayload)
    let observer = SPAvailabilityObserver(
      recognizer: recognizer, callback: callback, userInfo: userInfo,
      ctxRetain: ctxRetain, ctxRelease: ctxRelease)
    return spxRetain(observer)
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return nil
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return nil
  }
}

@_cdecl("sp_recognizer_availability_observer_stop")
public func sp_recognizer_availability_observer_stop(_ token: UnsafeMutableRawPointer?) {
  guard let token else { return }
  let observer: SPAvailabilityObserver = spxUnretained(token)
  observer.stop()
  spxRelease(token)
}

@_cdecl("sp_recognize_url_detailed_json")
public func sp_recognize_url_detailed_json(
  _ audioPath: UnsafePointer<CChar>,
  _ localeId: UnsafePointer<CChar>?,
  _ recognizerJson: UnsafePointer<CChar>?,
  _ requestJson: UnsafePointer<CChar>?,
  _ outResultJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    try spxEnsureAuthorized()
    let path = String(cString: audioPath)
    guard FileManager.default.fileExists(atPath: path) else {
      throw SPXBridgeError.audioLoadFailed("audio file does not exist: \(path)")
    }

    let recognizerPayload = try spxDecodeJSONIfPresent(
      recognizerJson, as: SPXRecognizerPayload.self)
    let requestPayload = try spxDecodeJSONIfPresent(requestJson, as: SPXRequestPayload.self)
    let recognizer = try spxCreateRecognizer(
      localeId: localeId, recognizerPayload: recognizerPayload)
    guard recognizer.isAvailable else {
      throw SPXBridgeError.recognizerUnavailable("recognizer is unavailable for this locale")
    }

    let request = SFSpeechURLRecognitionRequest(url: URL(fileURLWithPath: path))
    try spxApplyRequestPayload(requestPayload, recognizerPayload: recognizerPayload, to: request)

    let semaphore = DispatchSemaphore(value: 0)
    let gate = SPXFinalResultGate()
    var finalResult: SFSpeechRecognitionResult?
    var finalError: Error?

    let task = recognizer.recognitionTask(with: request) { result, error in
      guard
        gate.admits(
          hasResult: result != nil, isFinal: result?.isFinal ?? false, hasError: error != nil)
      else { return }
      finalResult = result
      finalError = error
      semaphore.signal()
    }

    let waited = semaphore.wait(timeout: .now() + .seconds(120))
    task.cancel()
    if waited == .timedOut {
      throw SPXBridgeError.timedOut("recognition timed out after 120s")
    }
    if let finalError {
      throw SPXBridgeError.framework(finalError)
    }
    guard let finalResult else {
      throw SPXBridgeError.recognitionFailed("recognition produced no final result")
    }

    outResultJson.pointee = spxCString(try spxEncodeJSON(spxEncodeRecognitionResult(finalResult)))
    return SPX_OK
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}
