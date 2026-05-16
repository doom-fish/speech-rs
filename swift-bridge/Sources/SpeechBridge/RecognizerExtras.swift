import Foundation
import Speech

public typealias SPAvailabilityCallback = @convention(c) (UnsafeMutableRawPointer?, Bool) -> Void

private final class SPAvailabilityObserver: NSObject, SFSpeechRecognizerDelegate {
  let recognizer: SFSpeechRecognizer
  let callback: SPAvailabilityCallback
  let userInfo: UnsafeMutableRawPointer?

  init(
    recognizer: SFSpeechRecognizer, callback: @escaping SPAvailabilityCallback,
    userInfo: UnsafeMutableRawPointer?
  ) {
    self.recognizer = recognizer
    self.callback = callback
    self.userInfo = userInfo
    super.init()
    recognizer.delegate = self
  }

  func speechRecognizer(
    _ speechRecognizer: SFSpeechRecognizer, availabilityDidChange available: Bool
  ) {
    callback(userInfo, available)
  }

  func stop() {
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
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
  do {
    let recognizerPayload = try spxDecodeJSONIfPresent(
      recognizerJson, as: SPXRecognizerPayload.self)
    let recognizer = try spxCreateRecognizer(
      localeId: localeId, recognizerPayload: recognizerPayload)
    let observer = SPAvailabilityObserver(
      recognizer: recognizer, callback: callback, userInfo: userInfo)
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
    var finalResult: SFSpeechRecognitionResult?
    var finalError: Error?

    let task = recognizer.recognitionTask(with: request) { result, error in
      if let error {
        finalError = error
        semaphore.signal()
        return
      }
      if let result, result.isFinal {
        finalResult = result
        semaphore.signal()
      }
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
