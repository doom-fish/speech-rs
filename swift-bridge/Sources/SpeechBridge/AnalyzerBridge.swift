import AVFoundation
import CoreMedia
import Foundation
import Speech

private func spxSpeechAnalyzerUnavailableMessage() -> String {
  "SpeechAnalyzer, SpeechTranscriber, and SpeechDetector require the macOS 26 SDK and macOS 26 runtime"
}

struct SPXSpeechTranscriberPayload: Codable, Sendable {
  var localeIdentifier: String
  var preset: String?
  var transcriptionOptions: [String]?
  var reportingOptions: [String]?
  var attributeOptions: [String]?
}

struct SPXSpeechDetectorPayload: Codable, Sendable {
  var sensitivityLevel: String
  var reportResults: Bool
}

struct SPXSpeechAnalyzerOptionsPayload: Codable, Sendable {
  var priority: String
  var modelRetention: String
}

struct SPXAnalysisContextPayload: Codable, Sendable {
  var contextualStrings: [String: [String]]?
  var userData: [String: String]?
}

struct SPXSpeechModulePayload: Codable, Sendable {
  var kind: String
  var transcriber: SPXSpeechTranscriberPayload?
  var detector: SPXSpeechDetectorPayload?
}

struct SPXSpeechAnalyzerPayload: Codable, Sendable {
  var modules: [SPXSpeechModulePayload]
  var options: SPXSpeechAnalyzerOptionsPayload?
  var context: SPXAnalysisContextPayload
}

struct SPXAnalyzerTimeRangePayload: Codable, Sendable {
  var startSeconds: Double
  var durationSeconds: Double
}

struct SPXSpeechAttributeSpanPayload: Codable, Sendable {
  var start: Int
  var end: Int
  var transcriptionConfidence: Double?
  var audioTimeRange: SPXAnalyzerTimeRangePayload?
}

struct SPXSpeechAttributedTextPayload: Codable, Sendable {
  var text: String
  var spans: [SPXSpeechAttributeSpanPayload]
}

struct SPXSpeechTranscriberResultPayload: Codable, Sendable {
  var range: SPXAnalyzerTimeRangePayload
  var resultsFinalizationTimeSeconds: Double
  var text: SPXSpeechAttributedTextPayload
  var alternatives: [SPXSpeechAttributedTextPayload]
  var isFinal: Bool
}

struct SPXSpeechDetectorResultPayload: Codable, Sendable {
  var range: SPXAnalyzerTimeRangePayload
  var resultsFinalizationTimeSeconds: Double
  var speechDetected: Bool
  var isFinal: Bool
}

struct SPXSpeechAnalyzerModuleOutputPayload: Codable, Sendable {
  var moduleIndex: Int
  var kind: String
  var transcriberResults: [SPXSpeechTranscriberResultPayload]?
  var detectorResults: [SPXSpeechDetectorResultPayload]?
}

struct SPXSpeechAnalyzerOutputPayload: Codable, Sendable {
  var modules: [SPXSpeechAnalyzerModuleOutputPayload]
  var volatileRange: SPXAnalyzerTimeRangePayload?
}

#if SPEECH_HAS_MACOS26_SDK
@available(macOS 26.0, *)
private func spxAnalyzerAudioFormatPayload(_ format: AVAudioFormat) -> SPXAudioFormatPayload {
  SPXAudioFormatPayload(
    sampleRate: format.sampleRate,
    channelCount: Int(format.channelCount),
    isInterleaved: format.isInterleaved,
    commonFormat: Int(format.commonFormat.rawValue)
  )
}

@available(macOS 26.0, *)
private func spxAnalyzerSeconds(_ time: CMTime) -> Double {
  let seconds = CMTimeGetSeconds(time)
  return seconds.isFinite ? seconds : 0
}

@available(macOS 26.0, *)
private func spxAnalyzerTimeRange(_ range: CMTimeRange) -> SPXAnalyzerTimeRangePayload {
  SPXAnalyzerTimeRangePayload(
    startSeconds: spxAnalyzerSeconds(range.start),
    durationSeconds: spxAnalyzerSeconds(range.duration)
  )
}

@available(macOS 26.0, *)
private func spxMakeSpeechTranscriberPreset(from rawValue: String) throws -> SpeechTranscriber.Preset {
  switch rawValue {
  case "transcription":
    return .transcription
  case "transcriptionWithAlternatives":
    return .transcriptionWithAlternatives
  case "timeIndexedTranscriptionWithAlternatives":
    return .timeIndexedTranscriptionWithAlternatives
  case "progressiveTranscription":
    return .progressiveTranscription
  case "timeIndexedProgressiveTranscription":
    return .timeIndexedProgressiveTranscription
  default:
    throw SPXBridgeError.invalidArgument("unknown SpeechTranscriber preset: \(rawValue)")
  }
}

@available(macOS 26.0, *)
private func spxMakeSpeechTranscriptionOptions(from rawValues: [String]?) throws
  -> Set<SpeechTranscriber.TranscriptionOption>
{
  guard let rawValues else { return [] }
  return try Set(rawValues.map { rawValue in
    switch rawValue {
    case "etiquetteReplacements":
      return .etiquetteReplacements
    default:
      throw SPXBridgeError.invalidArgument(
        "unknown SpeechTranscriber transcription option: \(rawValue)")
    }
  })
}

@available(macOS 26.0, *)
private func spxMakeSpeechReportingOptions(from rawValues: [String]?) throws
  -> Set<SpeechTranscriber.ReportingOption>
{
  guard let rawValues else { return [] }
  return try Set(rawValues.map { rawValue in
    switch rawValue {
    case "volatileResults":
      return .volatileResults
    case "alternativeTranscriptions":
      return .alternativeTranscriptions
    case "fastResults":
      return .fastResults
    default:
      throw SPXBridgeError.invalidArgument(
        "unknown SpeechTranscriber reporting option: \(rawValue)")
    }
  })
}

@available(macOS 26.0, *)
private func spxMakeSpeechAttributeOptions(from rawValues: [String]?) throws
  -> Set<SpeechTranscriber.ResultAttributeOption>
{
  guard let rawValues else { return [] }
  return try Set(rawValues.map { rawValue in
    switch rawValue {
    case "audioTimeRange":
      return .audioTimeRange
    case "transcriptionConfidence":
      return .transcriptionConfidence
    default:
      throw SPXBridgeError.invalidArgument(
        "unknown SpeechTranscriber result attribute option: \(rawValue)")
    }
  })
}

@available(macOS 26.0, *)
private func spxMakeSpeechTranscriber(from payload: SPXSpeechTranscriberPayload) throws
  -> SpeechTranscriber
{
  let locale = Locale(identifier: payload.localeIdentifier)
  if let preset = payload.preset {
    return SpeechTranscriber(locale: locale, preset: try spxMakeSpeechTranscriberPreset(from: preset))
  }
  return SpeechTranscriber(
    locale: locale,
    transcriptionOptions: try spxMakeSpeechTranscriptionOptions(from: payload.transcriptionOptions),
    reportingOptions: try spxMakeSpeechReportingOptions(from: payload.reportingOptions),
    attributeOptions: try spxMakeSpeechAttributeOptions(from: payload.attributeOptions)
  )
}

@available(macOS 26.0, *)
private func spxMakeSpeechDetectorSensitivity(from rawValue: String)
  throws -> SpeechDetector.SensitivityLevel
{
  switch rawValue {
  case "low":
    return .low
  case "medium":
    return .medium
  case "high":
    return .high
  default:
    throw SPXBridgeError.invalidArgument("unknown SpeechDetector sensitivity level: \(rawValue)")
  }
}

@available(macOS 26.0, *)
private func spxMakeSpeechDetector(from payload: SPXSpeechDetectorPayload) throws -> SpeechDetector {
  SpeechDetector(
    detectionOptions: .init(sensitivityLevel: try spxMakeSpeechDetectorSensitivity(from: payload.sensitivityLevel)),
    reportResults: payload.reportResults
  )
}

@available(macOS 26.0, *)
private func spxTaskPriority(from rawValue: String) throws -> TaskPriority {
  switch rawValue {
  case "background":
    return .background
  case "utility":
    return .utility
  case "low":
    return .low
  case "medium":
    return .medium
  case "high":
    return .high
  case "userInitiated":
    return .userInitiated
  default:
    throw SPXBridgeError.invalidArgument("unknown SpeechAnalyzer priority: \(rawValue)")
  }
}

@available(macOS 26.0, *)
private func spxModelRetention(from rawValue: String) throws -> SpeechAnalyzer.Options.ModelRetention {
  switch rawValue {
  case "whileInUse":
    return .whileInUse
  case "lingering":
    return .lingering
  case "processLifetime":
    return .processLifetime
  default:
    throw SPXBridgeError.invalidArgument("unknown SpeechAnalyzer model retention: \(rawValue)")
  }
}

@available(macOS 26.0, *)
private func spxMakeAnalyzerOptions(from payload: SPXSpeechAnalyzerOptionsPayload?) throws
  -> SpeechAnalyzer.Options?
{
  guard let payload else { return nil }
  return .init(
    priority: try spxTaskPriority(from: payload.priority),
    modelRetention: try spxModelRetention(from: payload.modelRetention)
  )
}

@available(macOS 26.0, *)
private func spxMakeAnalysisContext(from payload: SPXAnalysisContextPayload) -> AnalysisContext {
  let context = AnalysisContext()
  if let contextualStrings = payload.contextualStrings {
    for (tag, values) in contextualStrings {
      context.contextualStrings[.init(tag)] = values
    }
  }
  if let userData = payload.userData {
    for (tag, value) in userData {
      context.userData[.init(tag)] = value
    }
  }
  return context
}

@available(macOS 26.0, *)
func spxMakeSpeechModule(from payload: SPXSpeechModulePayload) throws -> any SpeechModule {
  switch payload.kind {
  case "speechTranscriber":
    guard let transcriber = payload.transcriber else {
      throw SPXBridgeError.invalidArgument("missing SpeechTranscriber payload")
    }
    return try spxMakeSpeechTranscriber(from: transcriber)
  case "speechDetector":
    guard let detector = payload.detector else {
      throw SPXBridgeError.invalidArgument("missing SpeechDetector payload")
    }
    return try spxMakeSpeechDetector(from: detector)
  default:
    throw SPXBridgeError.invalidArgument("unsupported SpeechModule kind: \(payload.kind)")
  }
}

@available(macOS 26.0, *)
private func spxEncodeSpeechAttributedText(_ value: AttributedString) -> SPXSpeechAttributedTextPayload {
  let plainText = String(value.characters)
  var spans: [SPXSpeechAttributeSpanPayload] = []

  for run in value.runs {
    guard let stringRange = Range(run.range, in: plainText) else { continue }
    let confidence = run[Foundation.AttributeScopes.SpeechAttributes.ConfidenceAttribute.self]
    let audioTimeRange = run[Foundation.AttributeScopes.SpeechAttributes.TimeRangeAttribute.self]
    guard confidence != nil || audioTimeRange != nil else { continue }

    spans.append(
      SPXSpeechAttributeSpanPayload(
        start: plainText.utf8.distance(from: plainText.startIndex, to: stringRange.lowerBound),
        end: plainText.utf8.distance(from: plainText.startIndex, to: stringRange.upperBound),
        transcriptionConfidence: confidence,
        audioTimeRange: audioTimeRange.map(spxAnalyzerTimeRange)
      ))
  }

  return SPXSpeechAttributedTextPayload(text: plainText, spans: spans)
}

@available(macOS 26.0, *)
private func spxEncodeSpeechTranscriberResult(_ result: SpeechTranscriber.Result)
  -> SPXSpeechTranscriberResultPayload
{
  SPXSpeechTranscriberResultPayload(
    range: spxAnalyzerTimeRange(result.range),
    resultsFinalizationTimeSeconds: spxAnalyzerSeconds(result.resultsFinalizationTime),
    text: spxEncodeSpeechAttributedText(result.text),
    alternatives: result.alternatives.map(spxEncodeSpeechAttributedText),
    isFinal: result.isFinal
  )
}

@available(macOS 26.0, *)
private func spxEncodeSpeechDetectorResult(_ result: SpeechDetector.Result)
  -> SPXSpeechDetectorResultPayload
{
  SPXSpeechDetectorResultPayload(
    range: spxAnalyzerTimeRange(result.range),
    resultsFinalizationTimeSeconds: spxAnalyzerSeconds(result.resultsFinalizationTime),
    speechDetected: result.speechDetected,
    isFinal: result.isFinal
  )
}

@available(macOS 26.0, *)
private func spxCollectAnalyzerOutputs(from modules: [any SpeechModule]) async throws
  -> [SPXSpeechAnalyzerModuleOutputPayload]
{
  try await withThrowingTaskGroup(of: SPXSpeechAnalyzerModuleOutputPayload.self) { group in
    for (index, module) in modules.enumerated() {
      if let transcriber = module as? SpeechTranscriber {
        group.addTask {
          var results: [SPXSpeechTranscriberResultPayload] = []
          for try await result in transcriber.results {
            results.append(spxEncodeSpeechTranscriberResult(result))
          }
          return SPXSpeechAnalyzerModuleOutputPayload(
            moduleIndex: index,
            kind: "speechTranscriber",
            transcriberResults: results,
            detectorResults: nil
          )
        }
      } else if let detector = module as? SpeechDetector {
        group.addTask {
          var results: [SPXSpeechDetectorResultPayload] = []
          for try await result in detector.results {
            results.append(spxEncodeSpeechDetectorResult(result))
          }
          return SPXSpeechAnalyzerModuleOutputPayload(
            moduleIndex: index,
            kind: "speechDetector",
            transcriberResults: nil,
            detectorResults: results
          )
        }
      }
    }

    var outputs: [SPXSpeechAnalyzerModuleOutputPayload] = []
    for try await output in group {
      outputs.append(output)
    }
    return outputs.sorted { $0.moduleIndex < $1.moduleIndex }
  }
}

@available(macOS 26.0, *)
private func spxRunSpeechAnalyzer(
  audioPath: String,
  payload: SPXSpeechAnalyzerPayload
) async throws -> String {
  guard FileManager.default.fileExists(atPath: audioPath) else {
    throw SPXBridgeError.audioLoadFailed("audio file does not exist: \(audioPath)")
  }

  try spxEnsureAuthorized()

  let modules = try payload.modules.map(spxMakeSpeechModule)
  let analyzer = SpeechAnalyzer(modules: modules, options: try spxMakeAnalyzerOptions(from: payload.options))
  try await analyzer.setContext(spxMakeAnalysisContext(from: payload.context))

  let resultTask = Task { () throws -> [SPXSpeechAnalyzerModuleOutputPayload] in
    try await spxCollectAnalyzerOutputs(from: modules)
  }

  do {
    let audioFile = try AVAudioFile(forReading: URL(fileURLWithPath: audioPath))
    try await analyzer.start(inputAudioFile: audioFile, finishAfterFile: true)
    let output = SPXSpeechAnalyzerOutputPayload(
      modules: try await resultTask.value,
      volatileRange: await analyzer.volatileRange.map(spxAnalyzerTimeRange)
    )
    return try spxEncodeJSON(output)
  } catch {
    resultTask.cancel()
    await analyzer.cancelAndFinishNow()
    throw error
  }
}
#endif

@_cdecl("sp_speech_transcriber_is_available")
public func sp_speech_transcriber_is_available() -> Bool {
  #if SPEECH_HAS_MACOS26_SDK
  if #available(macOS 26.0, *) {
    return SpeechTranscriber.isAvailable
  }
  #endif
  return false
}

@_cdecl("sp_speech_transcriber_supported_locales_json")
public func sp_speech_transcriber_supported_locales_json(
  _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let json = try spxRunAsyncBridgeBlocking { () async throws -> String in
        try spxEncodeJSON((await SpeechTranscriber.supportedLocales).map(\.identifier).sorted())
      }
      outJson.pointee = spxCString(json)
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxSpeechAnalyzerUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_speech_transcriber_installed_locales_json")
public func sp_speech_transcriber_installed_locales_json(
  _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let json = try spxRunAsyncBridgeBlocking { () async throws -> String in
        try spxEncodeJSON((await SpeechTranscriber.installedLocales).map(\.identifier).sorted())
      }
      outJson.pointee = spxCString(json)
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxSpeechAnalyzerUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_speech_transcriber_supported_locale_identifier")
public func sp_speech_transcriber_supported_locale_identifier(
  _ localeId: UnsafePointer<CChar>,
  _ outLocaleId: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let localeIdentifier = String(cString: localeId)
      let locale = Locale(identifier: localeIdentifier)
      let resolved = try spxRunAsyncBridgeBlocking { () async throws -> String? in
        await SpeechTranscriber.supportedLocale(equivalentTo: locale)?.identifier
      }
      if let resolved {
        outLocaleId.pointee = spxCString(resolved)
      } else {
        outLocaleId.pointee = nil
      }
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxSpeechAnalyzerUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_speech_transcriber_selected_locales_json")
public func sp_speech_transcriber_selected_locales_json(
  _ configurationJson: UnsafePointer<CChar>,
  _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let payload = try spxDecodeJSON(configurationJson, as: SPXSpeechTranscriberPayload.self)
      let json = try spxRunAsyncBridgeBlocking { () async throws -> String in
        let transcriber = try spxMakeSpeechTranscriber(from: payload)
        return try spxEncodeJSON(transcriber.selectedLocales.map(\.identifier))
      }
      outJson.pointee = spxCString(json)
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxSpeechAnalyzerUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_speech_transcriber_available_audio_formats_json")
public func sp_speech_transcriber_available_audio_formats_json(
  _ configurationJson: UnsafePointer<CChar>,
  _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let payload = try spxDecodeJSON(configurationJson, as: SPXSpeechTranscriberPayload.self)
      let json = try spxRunAsyncBridgeBlocking { () async throws -> String in
        let transcriber = try spxMakeSpeechTranscriber(from: payload)
        return try spxEncodeJSON(await transcriber.availableCompatibleAudioFormats.map(spxAnalyzerAudioFormatPayload))
      }
      outJson.pointee = spxCString(json)
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxSpeechAnalyzerUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_speech_detector_available_audio_formats_json")
public func sp_speech_detector_available_audio_formats_json(
  _ configurationJson: UnsafePointer<CChar>,
  _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let payload = try spxDecodeJSON(configurationJson, as: SPXSpeechDetectorPayload.self)
      let json = try spxRunAsyncBridgeBlocking { () async throws -> String in
        let detector = try spxMakeSpeechDetector(from: payload)
        return try spxEncodeJSON(detector.availableCompatibleAudioFormats.map(spxAnalyzerAudioFormatPayload))
      }
      outJson.pointee = spxCString(json)
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxSpeechAnalyzerUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_speech_analyzer_best_audio_format_json")
public func sp_speech_analyzer_best_audio_format_json(
  _ modulesJson: UnsafePointer<CChar>,
  _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let payload = try spxDecodeJSON(modulesJson, as: [SPXSpeechModulePayload].self)
      let json = try spxRunAsyncBridgeBlocking { () async throws -> String in
        let modules = try payload.map(spxMakeSpeechModule)
        let format = await SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith: modules)
        return try spxEncodeJSON(format.map(spxAnalyzerAudioFormatPayload))
      }
      outJson.pointee = spxCString(json)
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxSpeechAnalyzerUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_speech_analyzer_analyze_url_json")
public func sp_speech_analyzer_analyze_url_json(
  _ audioPath: UnsafePointer<CChar>,
  _ analyzerJson: UnsafePointer<CChar>,
  _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let payload = try spxDecodeJSON(analyzerJson, as: SPXSpeechAnalyzerPayload.self)
      let audioPathString = String(cString: audioPath)
      let json = try spxRunAsyncBridgeBlocking { () async throws -> String in
        try await spxRunSpeechAnalyzer(audioPath: audioPathString, payload: payload)
      }
      outJson.pointee = spxCString(json)
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxSpeechAnalyzerUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_speech_models_end_retention")
public func sp_speech_models_end_retention(
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      _ = try spxRunAsyncBridgeBlocking { () async throws -> Bool in
        await SpeechModels.endRetention()
        return true
      }
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxSpeechAnalyzerUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}
