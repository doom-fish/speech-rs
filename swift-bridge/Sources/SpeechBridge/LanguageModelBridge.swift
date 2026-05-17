import Foundation
import Speech

@available(macOS 14.0, *)
private func spxPrepareCustomLanguageModel(
  assetPath: String,
  configurationPayload: SPXLanguageModelConfigurationPayload,
  ignoresCache: Bool,
  clientIdentifier: String?
) throws {
  guard FileManager.default.fileExists(atPath: assetPath) else {
    throw SPXBridgeError.audioLoadFailed("custom language model asset does not exist: \(assetPath)")
  }

  let assetURL = URL(fileURLWithPath: assetPath)
  let configuration = try spxMakeLanguageModelConfiguration(from: configurationPayload)
  let semaphore = DispatchSemaphore(value: 0)
  var finalError: Error?

  if let clientIdentifier {
    if ignoresCache {
      SFSpeechLanguageModel.prepareCustomLanguageModel(
        for: assetURL,
        clientIdentifier: clientIdentifier,
        configuration: configuration,
        ignoresCache: true
      ) { error in
        finalError = error
        semaphore.signal()
      }
    } else {
      SFSpeechLanguageModel.prepareCustomLanguageModel(
        for: assetURL,
        clientIdentifier: clientIdentifier,
        configuration: configuration
      ) { error in
        finalError = error
        semaphore.signal()
      }
    }
  } else if ignoresCache {
    // The clientIdentifier-free overload only exists on macOS 26+.
    // Older runner SDKs require the clientIdentifier form; use a
    // bundle-derived fallback identifier when the caller didn't pass one.
    let fallbackIdentifier =
      Bundle.main.bundleIdentifier ?? "doom-fish.speech-rs.bridge"
    SFSpeechLanguageModel.prepareCustomLanguageModel(
      for: assetURL,
      clientIdentifier: fallbackIdentifier,
      configuration: configuration,
      ignoresCache: true
    ) { error in
      finalError = error
      semaphore.signal()
    }
  } else {
    let fallbackIdentifier =
      Bundle.main.bundleIdentifier ?? "doom-fish.speech-rs.bridge"
    SFSpeechLanguageModel.prepareCustomLanguageModel(
      for: assetURL,
      clientIdentifier: fallbackIdentifier,
      configuration: configuration
    ) { error in
      finalError = error
      semaphore.signal()
    }
  }

  if semaphore.wait(timeout: .now() + .seconds(120)) == .timedOut {
    throw SPXBridgeError.timedOut("custom language model preparation timed out after 120s")
  }
  if let finalError {
    throw SPXBridgeError.framework(finalError)
  }
}

private func spxRunLanguageModelPreparation(
  assetPathPointer: UnsafePointer<CChar>,
  configurationJson: UnsafePointer<CChar>,
  ignoresCache: Bool,
  clientIdentifierPointer: UnsafePointer<CChar>?,
  outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    if #unavailable(macOS 14.0) {
      throw SPXBridgeError.unavailableOnThisMacOS("custom language models require macOS 14+")
    }
    let assetPath = String(cString: assetPathPointer)
    let configurationPayload = try spxDecodeJSON(
      configurationJson, as: SPXLanguageModelConfigurationPayload.self)
    let clientIdentifier = clientIdentifierPointer.map { String(cString: $0) }
    if #available(macOS 14.0, *) {
      try spxPrepareCustomLanguageModel(
        assetPath: assetPath,
        configurationPayload: configurationPayload,
        ignoresCache: ignoresCache,
        clientIdentifier: clientIdentifier
      )
    }
    return SPX_OK
  } catch let error as SPXBridgeError {
    switch error {
    case let .framework(wrapped):
      if let json = try? spxEncodeJSON(spxEncodeTaskError(wrapped as NSError)) {
        outErrorMessage?.pointee = spxCString(json)
      } else {
        outErrorMessage?.pointee = spxCString(error.description)
      }
    default:
      outErrorMessage?.pointee = spxCString(error.description)
    }
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_prepare_custom_language_model")
public func sp_prepare_custom_language_model(
  _ assetPath: UnsafePointer<CChar>,
  _ configurationJson: UnsafePointer<CChar>,
  _ ignoresCache: Bool,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  spxRunLanguageModelPreparation(
    assetPathPointer: assetPath,
    configurationJson: configurationJson,
    ignoresCache: ignoresCache,
    clientIdentifierPointer: nil,
    outErrorMessage: outErrorMessage
  )
}

@_cdecl("sp_prepare_custom_language_model_with_client_identifier")
public func sp_prepare_custom_language_model_with_client_identifier(
  _ assetPath: UnsafePointer<CChar>,
  _ clientIdentifier: UnsafePointer<CChar>,
  _ configurationJson: UnsafePointer<CChar>,
  _ ignoresCache: Bool,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  spxRunLanguageModelPreparation(
    assetPathPointer: assetPath,
    configurationJson: configurationJson,
    ignoresCache: ignoresCache,
    clientIdentifierPointer: clientIdentifier,
    outErrorMessage: outErrorMessage
  )
}
