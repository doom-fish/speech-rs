import AVFoundation
import CoreMedia
import Foundation
import Speech

let SPX_OK: Int32 = 0
let SPX_INVALID_ARGUMENT: Int32 = -1
let SPX_NOT_AUTHORIZED: Int32 = -2
let SPX_RECOGNIZER_UNAVAILABLE: Int32 = -3
let SPX_AUDIO_LOAD_FAILED: Int32 = -4
let SPX_RECOGNITION_FAILED: Int32 = -5
let SPX_TIMED_OUT: Int32 = -6
let SPX_UNKNOWN: Int32 = -99

func spxCString(_ string: String) -> UnsafeMutablePointer<CChar>? {
  string.withCString { strdup($0) }
}

func spxRetain(_ object: some AnyObject) -> UnsafeMutableRawPointer {
  Unmanaged.passRetained(object).toOpaque()
}

func spxUnretained<T: AnyObject>(_ ptr: UnsafeMutableRawPointer, as type: T.Type = T.self) -> T {
  Unmanaged<T>.fromOpaque(ptr).takeUnretainedValue()
}

func spxRelease(_ ptr: UnsafeMutableRawPointer) {
  Unmanaged<AnyObject>.fromOpaque(ptr).release()
}

enum SPXBridgeError: Error, CustomStringConvertible {
  case invalidArgument(String)
  case notAuthorized(String)
  case recognizerUnavailable(String)
  case audioLoadFailed(String)
  case recognitionFailed(String)
  case timedOut(String)
  case framework(Error)
  case unavailableOnThisMacOS(String)
  case unknown(String)

  var description: String {
    switch self {
    case .invalidArgument(let message): return message
    case .notAuthorized(let message): return message
    case .recognizerUnavailable(let message): return message
    case .audioLoadFailed(let message): return message
    case .recognitionFailed(let message): return message
    case .timedOut(let message): return message
    case .framework(let error): return error.localizedDescription
    case .unavailableOnThisMacOS(let message): return message
    case .unknown(let message): return message
    }
  }

  var statusCode: Int32 {
    switch self {
    case .invalidArgument: return SPX_INVALID_ARGUMENT
    case .notAuthorized: return SPX_NOT_AUTHORIZED
    case .recognizerUnavailable, .unavailableOnThisMacOS: return SPX_RECOGNIZER_UNAVAILABLE
    case .audioLoadFailed: return SPX_AUDIO_LOAD_FAILED
    case .recognitionFailed, .framework: return SPX_RECOGNITION_FAILED
    case .timedOut: return SPX_TIMED_OUT
    case .unknown: return SPX_UNKNOWN
    }
  }
}

func spxDecodeJSON<T: Decodable>(_ cString: UnsafePointer<CChar>?, as type: T.Type) throws -> T {
  guard let cString else {
    throw SPXBridgeError.invalidArgument("missing JSON payload")
  }
  let data = Data(String(cString: cString).utf8)
  do {
    return try JSONDecoder().decode(T.self, from: data)
  } catch {
    throw SPXBridgeError.invalidArgument("invalid JSON payload: \(error.localizedDescription)")
  }
}

func spxDecodeJSONIfPresent<T: Decodable>(_ cString: UnsafePointer<CChar>?, as type: T.Type) throws
  -> T?
{
  guard cString != nil else { return nil }
  return try spxDecodeJSON(cString, as: type)
}

func spxEncodeJSON<T: Encodable>(_ value: T) throws -> String {
  let data = try JSONEncoder().encode(value)
  guard let string = String(data: data, encoding: .utf8) else {
    throw SPXBridgeError.unknown("failed to encode JSON as UTF-8")
  }
  return string
}

struct SPXQueuePayload: Codable {
  var kind: String
  var name: String?
  var maxConcurrentOperationCount: Int?
}

struct SPXRecognizerPayload: Codable {
  var defaultTaskHint: Int?
  var queue: SPXQueuePayload?
}

struct SPXLanguageModelConfigurationPayload: Codable, Sendable {
  var languageModelPath: String
  var vocabularyPath: String?
  var weight: Double?
}

struct SPXRequestPayload: Codable {
  var taskHint: Int?
  var shouldReportPartialResults: Bool?
  var contextualStrings: [String]?
  var interactionIdentifier: String?
  var requiresOnDeviceRecognition: Bool?
  var addsPunctuation: Bool?
  var customizedLanguageModel: SPXLanguageModelConfigurationPayload?
}

struct SPXRangePayload: Codable {
  var location: Int
  var length: Int
}

struct SPXAcousticFeaturePayload: Codable {
  var acousticFeatureValuePerFrame: [Double]
  var frameDuration: Double
}

struct SPXVoiceAnalyticsPayload: Codable {
  var jitter: SPXAcousticFeaturePayload
  var shimmer: SPXAcousticFeaturePayload
  var pitch: SPXAcousticFeaturePayload
  var voicing: SPXAcousticFeaturePayload
}

struct SPXTranscriptionSegmentPayload: Codable {
  var substring: String
  var substringRange: SPXRangePayload
  var timestamp: Double
  var duration: Double
  var confidence: Float
  var alternativeSubstrings: [String]
  var voiceAnalytics: SPXVoiceAnalyticsPayload?
}

struct SPXTranscriptionPayload: Codable {
  var formattedString: String
  var segments: [SPXTranscriptionSegmentPayload]
  var speakingRate: Double?
  var averagePauseDuration: Double?
}

struct SPXRecognitionMetadataPayload: Codable {
  var speakingRate: Double
  var averagePauseDuration: Double
  var speechStartTimestamp: Double
  var speechDuration: Double
  var voiceAnalytics: SPXVoiceAnalyticsPayload?
}

struct SPXRecognitionResultPayload: Codable {
  var bestTranscription: SPXTranscriptionPayload
  var transcriptions: [SPXTranscriptionPayload]
  var isFinal: Bool
  var speechRecognitionMetadata: SPXRecognitionMetadataPayload?
}

struct SPXTaskErrorPayload: Codable {
  var domain: String
  var code: Int
  var localizedDescription: String
}

struct SPXTaskEventPayload: Codable {
  var event: String
  var transcription: SPXTranscriptionPayload?
  var result: SPXRecognitionResultPayload?
  var duration: Double?
  var successfully: Bool?
}

struct SPXAudioFormatPayload: Codable {
  var sampleRate: Double
  var channelCount: Int
  var isInterleaved: Bool
  var commonFormat: Int
}

func spxTaskHint(from rawValue: Int?) -> SFSpeechRecognitionTaskHint {
  guard let rawValue, let hint = SFSpeechRecognitionTaskHint(rawValue: rawValue) else {
    return .unspecified
  }
  return hint
}

func spxMakeOperationQueue(from payload: SPXQueuePayload?) -> OperationQueue {
  guard let payload else {
    return OperationQueue.main
  }

  switch payload.kind {
  case "main":
    return OperationQueue.main
  case "background":
    let queue = OperationQueue()
    queue.name = payload.name
    if let maxConcurrentOperationCount = payload.maxConcurrentOperationCount {
      queue.maxConcurrentOperationCount = maxConcurrentOperationCount
    }
    return queue
  default:
    return OperationQueue.main
  }
}

@available(macOS 14.0, *)
func spxMakeLanguageModelConfiguration(from payload: SPXLanguageModelConfigurationPayload) throws
  -> SFSpeechLanguageModel.Configuration
{
  let languageModelURL = URL(fileURLWithPath: payload.languageModelPath)
  let vocabularyURL = payload.vocabularyPath.map(URL.init(fileURLWithPath:))

  if let weight = payload.weight {
    if #available(macOS 26.0, *) {
      return SFSpeechLanguageModel.Configuration(
        languageModel: languageModelURL,
        vocabulary: vocabularyURL,
        weight: NSNumber(value: weight)
      )
    }
    throw SPXBridgeError.unavailableOnThisMacOS("custom language model weight requires macOS 26+")
  }

  if let vocabularyURL {
    return SFSpeechLanguageModel.Configuration(
      languageModel: languageModelURL, vocabulary: vocabularyURL)
  }
  return SFSpeechLanguageModel.Configuration(languageModel: languageModelURL)
}

func spxApplyRecognizerPayload(_ payload: SPXRecognizerPayload?, to recognizer: SFSpeechRecognizer)
{
  guard let payload else { return }
  if let defaultTaskHint = payload.defaultTaskHint {
    recognizer.defaultTaskHint = spxTaskHint(from: defaultTaskHint)
  }
  recognizer.queue = spxMakeOperationQueue(from: payload.queue)
}

func spxApplyRequestPayload(
  _ payload: SPXRequestPayload?,
  recognizerPayload: SPXRecognizerPayload?,
  to request: SFSpeechRecognitionRequest
) throws {
  request.taskHint = spxTaskHint(from: payload?.taskHint ?? recognizerPayload?.defaultTaskHint)

  if let shouldReportPartialResults = payload?.shouldReportPartialResults {
    request.shouldReportPartialResults = shouldReportPartialResults
  }
  if let contextualStrings = payload?.contextualStrings {
    request.contextualStrings = contextualStrings
  }
  if let interactionIdentifier = payload?.interactionIdentifier {
    request.interactionIdentifier = interactionIdentifier
  }
  if let requiresOnDeviceRecognition = payload?.requiresOnDeviceRecognition {
    request.requiresOnDeviceRecognition = requiresOnDeviceRecognition
  }
  if let addsPunctuation = payload?.addsPunctuation {
    if #available(macOS 13.0, *) {
      request.addsPunctuation = addsPunctuation
    } else if addsPunctuation {
      throw SPXBridgeError.unavailableOnThisMacOS("automatic punctuation requires macOS 13+")
    }
  }
  if let customizedLanguageModel = payload?.customizedLanguageModel {
    if #available(macOS 14.0, *) {
      request.customizedLanguageModel = try spxMakeLanguageModelConfiguration(
        from: customizedLanguageModel)
    } else {
      throw SPXBridgeError.unavailableOnThisMacOS("custom language models require macOS 14+")
    }
  }
}

func spxCreateRecognizer(
  localeId: UnsafePointer<CChar>?,
  recognizerPayload: SPXRecognizerPayload?
) throws -> SFSpeechRecognizer {
  let recognizer: SFSpeechRecognizer?
  if let localeId {
    recognizer = SFSpeechRecognizer(locale: Locale(identifier: String(cString: localeId)))
  } else {
    recognizer = SFSpeechRecognizer()
  }
  guard let recognizer else {
    throw SPXBridgeError.recognizerUnavailable("unable to create speech recognizer")
  }
  spxApplyRecognizerPayload(recognizerPayload, to: recognizer)
  return recognizer
}

func spxEnsureAuthorized() throws {
  let status = SFSpeechRecognizer.authorizationStatus()
  guard status == .authorized else {
    throw SPXBridgeError.notAuthorized(
      "Speech recognition not authorized (status=\(status.rawValue)). Add NSSpeechRecognitionUsageDescription to your Info.plist and call requestAuthorization()."
    )
  }
}

@available(macOS 14.0, *)
func spxEncodeAcousticFeature(_ feature: SFAcousticFeature) -> SPXAcousticFeaturePayload {
  SPXAcousticFeaturePayload(
    acousticFeatureValuePerFrame: feature.acousticFeatureValuePerFrame.map { Double($0) },
    frameDuration: feature.frameDuration
  )
}

func spxEncodeVoiceAnalytics(_ analytics: SFVoiceAnalytics?) -> SPXVoiceAnalyticsPayload? {
  guard let analytics else { return nil }
  if #available(macOS 14.0, *) {
    return SPXVoiceAnalyticsPayload(
      jitter: spxEncodeAcousticFeature(analytics.jitter),
      shimmer: spxEncodeAcousticFeature(analytics.shimmer),
      pitch: spxEncodeAcousticFeature(analytics.pitch),
      voicing: spxEncodeAcousticFeature(analytics.voicing)
    )
  }
  return nil
}

func spxEncodeTranscriptionSegment(_ segment: SFTranscriptionSegment)
  -> SPXTranscriptionSegmentPayload
{
  SPXTranscriptionSegmentPayload(
    substring: segment.substring,
    substringRange: SPXRangePayload(
      location: segment.substringRange.location, length: segment.substringRange.length),
    timestamp: segment.timestamp,
    duration: segment.duration,
    confidence: segment.confidence,
    alternativeSubstrings: segment.alternativeSubstrings,
    voiceAnalytics: spxEncodeVoiceAnalytics(segment.voiceAnalytics)
  )
}

func spxEncodeTranscription(_ transcription: SFTranscription) -> SPXTranscriptionPayload {
  SPXTranscriptionPayload(
    formattedString: transcription.formattedString,
    segments: transcription.segments.map(spxEncodeTranscriptionSegment),
    speakingRate: transcription.speakingRate,
    averagePauseDuration: transcription.averagePauseDuration
  )
}

func spxEncodeRecognitionMetadata(_ metadata: SFSpeechRecognitionMetadata?)
  -> SPXRecognitionMetadataPayload?
{
  guard let metadata else { return nil }
  return SPXRecognitionMetadataPayload(
    speakingRate: metadata.speakingRate,
    averagePauseDuration: metadata.averagePauseDuration,
    speechStartTimestamp: metadata.speechStartTimestamp,
    speechDuration: metadata.speechDuration,
    voiceAnalytics: spxEncodeVoiceAnalytics(metadata.voiceAnalytics)
  )
}

func spxEncodeRecognitionResult(_ result: SFSpeechRecognitionResult) -> SPXRecognitionResultPayload
{
  SPXRecognitionResultPayload(
    bestTranscription: spxEncodeTranscription(result.bestTranscription),
    transcriptions: result.transcriptions.map(spxEncodeTranscription),
    isFinal: result.isFinal,
    speechRecognitionMetadata: spxEncodeRecognitionMetadata(result.speechRecognitionMetadata)
  )
}

func spxEncodeTaskError(_ error: NSError?) -> SPXTaskErrorPayload? {
  guard let error else { return nil }
  return SPXTaskErrorPayload(
    domain: error.domain, code: error.code, localizedDescription: error.localizedDescription)
}

func spxSendTaskEvent(
  callback: @escaping @convention(c) (UnsafeMutableRawPointer?, UnsafePointer<CChar>?) -> Void,
  userInfo: UnsafeMutableRawPointer?,
  payload: SPXTaskEventPayload
) {
  guard let json = try? spxEncodeJSON(payload) else {
    callback(userInfo, nil)
    return
  }
  json.withCString { callback(userInfo, $0) }
}
