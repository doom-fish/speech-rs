import AVFoundation
import CoreMedia
import Foundation
import Speech

private func spxDictationUnavailableMessage() -> String {
  "DictationTranscriber requires the macOS 26 SDK and macOS 26 runtime"
}

private struct SPXStoredError: Error, Sendable {
  let statusCode: Int32
  let message: String
}

private final class SPXAsyncState<T: Sendable>: @unchecked Sendable {
  private let lock = NSLock()
  private var result: Result<T, SPXStoredError>?

  func store(_ result: Result<T, SPXStoredError>) {
    lock.lock()
    self.result = result
    lock.unlock()
  }

  func load() -> Result<T, SPXStoredError>? {
    lock.lock()
    defer { lock.unlock() }
    return result
  }
}

private func spxBridgeError(from stored: SPXStoredError) -> SPXBridgeError {
  switch stored.statusCode {
  case SPX_INVALID_ARGUMENT:
    return .invalidArgument(stored.message)
  case SPX_NOT_AUTHORIZED:
    return .notAuthorized(stored.message)
  case SPX_RECOGNIZER_UNAVAILABLE:
    return .recognizerUnavailable(stored.message)
  case SPX_AUDIO_LOAD_FAILED:
    return .audioLoadFailed(stored.message)
  case SPX_RECOGNITION_FAILED:
    return .recognitionFailed(stored.message)
  case SPX_TIMED_OUT:
    return .timedOut(stored.message)
  default:
    return .unknown(stored.message)
  }
}

private func spxRunAsyncBlocking<T: Sendable>(
  timeoutSeconds: Int = 120,
  operation: @escaping @Sendable () async throws -> T
) throws -> T {
  let semaphore = DispatchSemaphore(value: 0)
  let state = SPXAsyncState<T>()
  let task = Task {
    do {
      state.store(.success(try await operation()))
    } catch let error as SPXBridgeError {
      state.store(.failure(SPXStoredError(statusCode: error.statusCode, message: error.description)))
    } catch {
      state.store(.failure(SPXStoredError(statusCode: SPX_UNKNOWN, message: error.localizedDescription)))
    }
    semaphore.signal()
  }

  if semaphore.wait(timeout: .now() + .seconds(timeoutSeconds)) == .timedOut {
    task.cancel()
    throw SPXBridgeError.timedOut("dictation operation timed out after \(timeoutSeconds)s")
  }

  guard let result = state.load() else {
    throw SPXBridgeError.unknown("dictation operation completed without a result")
  }

  switch result {
  case .success(let value):
    return value
  case .failure(let stored):
    throw spxBridgeError(from: stored)
  }
}

struct SPXDictationContentHintPayload: Codable, Sendable {
  var kind: String
  var languageModel: SPXLanguageModelConfigurationPayload?
}

struct SPXDictationTranscriberPayload: Codable, Sendable {
  var localeIdentifier: String
  var preset: String?
  var contentHints: [SPXDictationContentHintPayload]?
  var transcriptionOptions: [String]?
  var reportingOptions: [String]?
  var attributeOptions: [String]?
}

struct SPXDictationTimeRangePayload: Codable, Sendable {
  var startSeconds: Double
  var durationSeconds: Double
}

struct SPXDictationResultPayload: Codable, Sendable {
  var text: String
  var alternatives: [String]
  var range: SPXDictationTimeRangePayload
  var resultsFinalizationTimeSeconds: Double
  var isFinal: Bool
}

#if SPEECH_HAS_MACOS26_SDK
@available(macOS 26.0, *)
private func spxString(from attributedString: AttributedString) -> String {
  String(attributedString.characters)
}

@available(macOS 26.0, *)
private func spxSeconds(_ time: CMTime) -> Double {
  let seconds = CMTimeGetSeconds(time)
  return seconds.isFinite ? seconds : 0
}

@available(macOS 26.0, *)
private func spxMakeDictationPreset(from rawValue: String) throws -> DictationTranscriber.Preset {
  switch rawValue {
  case "phrase":
    return .phrase
  case "shortDictation":
    return .shortDictation
  case "progressiveShortDictation":
    return .progressiveShortDictation
  case "longDictation":
    return .longDictation
  case "progressiveLongDictation":
    return .progressiveLongDictation
  case "timeIndexedLongDictation":
    return .timeIndexedLongDictation
  default:
    throw SPXBridgeError.invalidArgument("unknown dictation preset: \(rawValue)")
  }
}

@available(macOS 26.0, *)
private func spxMakeDictationContentHints(from payloads: [SPXDictationContentHintPayload]?) throws
  -> Set<DictationTranscriber.ContentHint>
{
  guard let payloads else { return [] }
  var hints = Set<DictationTranscriber.ContentHint>()
  for payload in payloads {
    switch payload.kind {
    case "shortForm":
      hints.insert(.shortForm)
    case "farField":
      hints.insert(.farField)
    case "atypicalSpeech":
      hints.insert(.atypicalSpeech)
    case "customizedLanguage":
      guard let languageModel = payload.languageModel else {
        throw SPXBridgeError.invalidArgument(
          "customizedLanguage dictation content hint requires a language model configuration")
      }
      hints.insert(.customizedLanguage(modelConfiguration: try spxMakeLanguageModelConfiguration(from: languageModel)))
    default:
      throw SPXBridgeError.invalidArgument("unknown dictation content hint: \(payload.kind)")
    }
  }
  return hints
}

@available(macOS 26.0, *)
private func spxMakeDictationTranscriptionOptions(from rawValues: [String]?) throws
  -> Set<DictationTranscriber.TranscriptionOption>
{
  guard let rawValues else { return [] }
  return try Set(rawValues.map { rawValue in
    switch rawValue {
    case "punctuation":
      return .punctuation
    case "emoji":
      return .emoji
    case "etiquetteReplacements":
      return .etiquetteReplacements
    default:
      throw SPXBridgeError.invalidArgument("unknown dictation transcription option: \(rawValue)")
    }
  })
}

@available(macOS 26.0, *)
private func spxMakeDictationReportingOptions(from rawValues: [String]?) throws
  -> Set<DictationTranscriber.ReportingOption>
{
  guard let rawValues else { return [] }
  return try Set(rawValues.map { rawValue in
    switch rawValue {
    case "volatileResults":
      return .volatileResults
    case "alternativeTranscriptions":
      return .alternativeTranscriptions
    case "frequentFinalization":
      return .frequentFinalization
    default:
      throw SPXBridgeError.invalidArgument("unknown dictation reporting option: \(rawValue)")
    }
  })
}

@available(macOS 26.0, *)
private func spxMakeDictationAttributeOptions(from rawValues: [String]?) throws
  -> Set<DictationTranscriber.ResultAttributeOption>
{
  guard let rawValues else { return [] }
  return try Set(rawValues.map { rawValue in
    switch rawValue {
    case "audioTimeRange":
      return .audioTimeRange
    case "transcriptionConfidence":
      return .transcriptionConfidence
    default:
      throw SPXBridgeError.invalidArgument("unknown dictation result attribute option: \(rawValue)")
    }
  })
}

@available(macOS 26.0, *)
private func spxMakeDictationTranscriber(from payload: SPXDictationTranscriberPayload) throws
  -> DictationTranscriber
{
  let locale = Locale(identifier: payload.localeIdentifier)
  if let preset = payload.preset {
    return DictationTranscriber(locale: locale, preset: try spxMakeDictationPreset(from: preset))
  }
  return DictationTranscriber(
    locale: locale,
    contentHints: try spxMakeDictationContentHints(from: payload.contentHints),
    transcriptionOptions: try spxMakeDictationTranscriptionOptions(from: payload.transcriptionOptions),
    reportingOptions: try spxMakeDictationReportingOptions(from: payload.reportingOptions),
    attributeOptions: try spxMakeDictationAttributeOptions(from: payload.attributeOptions)
  )
}

@available(macOS 26.0, *)
private func spxMakeDictationAudioFormatPayload(_ format: AVAudioFormat) -> SPXAudioFormatPayload {
  SPXAudioFormatPayload(
    sampleRate: format.sampleRate,
    channelCount: Int(format.channelCount),
    isInterleaved: format.isInterleaved,
    commonFormat: Int(format.commonFormat.rawValue)
  )
}

@available(macOS 26.0, *)
private func spxEncodeDictationResult(_ result: DictationTranscriber.Result) -> SPXDictationResultPayload {
  SPXDictationResultPayload(
    text: spxString(from: result.text),
    alternatives: result.alternatives.map { spxString(from: $0) },
    range: SPXDictationTimeRangePayload(
      startSeconds: spxSeconds(result.range.start),
      durationSeconds: spxSeconds(result.range.duration)
    ),
    resultsFinalizationTimeSeconds: spxSeconds(result.resultsFinalizationTime),
    isFinal: result.isFinal
  )
}

@available(macOS 26.0, *)
private func spxRunDictationTranscription(
  audioPath: String,
  payload: SPXDictationTranscriberPayload
) async throws -> String {
  guard FileManager.default.fileExists(atPath: audioPath) else {
    throw SPXBridgeError.audioLoadFailed("audio file does not exist: \(audioPath)")
  }

  try spxEnsureAuthorized()

  let transcriber = try spxMakeDictationTranscriber(from: payload)
  let audioFile = try AVAudioFile(forReading: URL(fileURLWithPath: audioPath))
  let analyzer = SpeechAnalyzer(modules: [transcriber])
  let resultTask = Task { () throws -> [SPXDictationResultPayload] in
    var results: [SPXDictationResultPayload] = []
    for try await result in transcriber.results {
      results.append(spxEncodeDictationResult(result))
    }
    return results
  }

  do {
    try await analyzer.start(inputAudioFile: audioFile, finishAfterFile: true)
    return try spxEncodeJSON(try await resultTask.value)
  } catch {
    resultTask.cancel()
    await analyzer.cancelAndFinishNow()
    throw error
  }
}
#endif

@_cdecl("sp_dictation_supported_locales_json")
public func sp_dictation_supported_locales_json(
  _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let json = try spxRunAsyncBlocking { () async throws -> String in
        try spxEncodeJSON((await DictationTranscriber.supportedLocales).map(\.identifier).sorted())
      }
      outJson.pointee = spxCString(json)
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxDictationUnavailableMessage())
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_dictation_installed_locales_json")
public func sp_dictation_installed_locales_json(
  _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let json = try spxRunAsyncBlocking { () async throws -> String in
        try spxEncodeJSON((await DictationTranscriber.installedLocales).map(\.identifier).sorted())
      }
      outJson.pointee = spxCString(json)
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxDictationUnavailableMessage())
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_dictation_supported_locale_identifier")
public func sp_dictation_supported_locale_identifier(
  _ localeId: UnsafePointer<CChar>,
  _ outLocaleId: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    let localeIdentifier = String(cString: localeId)
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let resolvedLocale = try spxRunAsyncBlocking { () async throws -> String? in
        await DictationTranscriber.supportedLocale(equivalentTo: Locale(identifier: localeIdentifier))?.identifier
      }
      if let resolvedLocale {
        outLocaleId.pointee = spxCString(resolvedLocale)
      } else {
        outLocaleId.pointee = nil
      }
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxDictationUnavailableMessage())
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_dictation_selected_locales_json")
public func sp_dictation_selected_locales_json(
  _ configurationJson: UnsafePointer<CChar>,
  _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    let payload = try spxDecodeJSON(configurationJson, as: SPXDictationTranscriberPayload.self)
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let transcriber = try spxMakeDictationTranscriber(from: payload)
      outJson.pointee = spxCString(try spxEncodeJSON(transcriber.selectedLocales.map(\.identifier)))
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxDictationUnavailableMessage())
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_dictation_available_audio_formats_json")
public func sp_dictation_available_audio_formats_json(
  _ configurationJson: UnsafePointer<CChar>,
  _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    let payload = try spxDecodeJSON(configurationJson, as: SPXDictationTranscriberPayload.self)
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let json = try spxRunAsyncBlocking { () async throws -> String in
        let transcriber = try spxMakeDictationTranscriber(from: payload)
        let formats = await transcriber.availableCompatibleAudioFormats.map {
          spxMakeDictationAudioFormatPayload($0)
        }
        return try spxEncodeJSON(formats)
      }
      outJson.pointee = spxCString(json)
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxDictationUnavailableMessage())
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_dictation_transcribe_url_json")
public func sp_dictation_transcribe_url_json(
  _ audioPath: UnsafePointer<CChar>,
  _ configurationJson: UnsafePointer<CChar>,
  _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    let payload = try spxDecodeJSON(configurationJson, as: SPXDictationTranscriberPayload.self)
    let path = String(cString: audioPath)
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      outJson.pointee = spxCString(try spxRunAsyncBlocking {
        try await spxRunDictationTranscription(audioPath: path, payload: payload)
      })
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxDictationUnavailableMessage())
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}
