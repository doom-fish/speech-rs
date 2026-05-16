import AVFoundation
import CoreMedia
import Foundation
import Speech

public typealias SPTaskEventCallback =
  @convention(c) (UnsafeMutableRawPointer?, UnsafePointer<CChar>?) -> Void

private final class SPRustTaskDelegate: NSObject, SFSpeechRecognitionTaskDelegate {
  let callback: SPTaskEventCallback
  let userInfo: UnsafeMutableRawPointer?
  var isActive = true

  init(callback: @escaping SPTaskEventCallback, userInfo: UnsafeMutableRawPointer?) {
    self.callback = callback
    self.userInfo = userInfo
    super.init()
  }

  func deactivate() {
    isActive = false
  }

  private func send(_ payload: SPXTaskEventPayload) {
    guard isActive else { return }
    spxSendTaskEvent(callback: callback, userInfo: userInfo, payload: payload)
  }

  func speechRecognitionDidDetectSpeech(_ task: SFSpeechRecognitionTask) {
    send(
      SPXTaskEventPayload(
        event: "didDetectSpeech", transcription: nil, result: nil, duration: nil, successfully: nil)
    )
  }

  func speechRecognitionTask(
    _ task: SFSpeechRecognitionTask, didHypothesizeTranscription transcription: SFTranscription
  ) {
    send(
      SPXTaskEventPayload(
        event: "didHypothesizeTranscription",
        transcription: spxEncodeTranscription(transcription),
        result: nil,
        duration: nil,
        successfully: nil
      ))
  }

  func speechRecognitionTask(
    _ task: SFSpeechRecognitionTask,
    didFinishRecognition recognitionResult: SFSpeechRecognitionResult
  ) {
    send(
      SPXTaskEventPayload(
        event: "didFinishRecognition",
        transcription: nil,
        result: spxEncodeRecognitionResult(recognitionResult),
        duration: nil,
        successfully: nil
      ))
  }

  func speechRecognitionTaskFinishedReadingAudio(_ task: SFSpeechRecognitionTask) {
    send(
      SPXTaskEventPayload(
        event: "finishedReadingAudio", transcription: nil, result: nil, duration: nil,
        successfully: nil))
  }

  func speechRecognitionTaskWasCancelled(_ task: SFSpeechRecognitionTask) {
    send(
      SPXTaskEventPayload(
        event: "wasCancelled", transcription: nil, result: nil, duration: nil, successfully: nil))
  }

  func speechRecognitionTask(
    _ task: SFSpeechRecognitionTask, didFinishSuccessfully successfully: Bool
  ) {
    send(
      SPXTaskEventPayload(
        event: "didFinishSuccessfully", transcription: nil, result: nil, duration: nil,
        successfully: successfully))
  }

  func speechRecognitionTask(
    _ task: SFSpeechRecognitionTask, didProcessAudioDuration duration: TimeInterval
  ) {
    send(
      SPXTaskEventPayload(
        event: "didProcessAudioDuration", transcription: nil, result: nil, duration: duration,
        successfully: nil))
  }
}

private final class SPTaskBox: NSObject {
  let recognizer: SFSpeechRecognizer
  let request: SFSpeechRecognitionRequest
  let audioBufferRequest: SFSpeechAudioBufferRecognitionRequest?
  let delegate: SPRustTaskDelegate
  let task: SFSpeechRecognitionTask
  let audioEngine: AVAudioEngine?
  let hasMicrophoneTap: Bool
  private var cleanedUp = false

  init(
    recognizer: SFSpeechRecognizer,
    request: SFSpeechRecognitionRequest,
    audioBufferRequest: SFSpeechAudioBufferRecognitionRequest?,
    delegate: SPRustTaskDelegate,
    task: SFSpeechRecognitionTask,
    audioEngine: AVAudioEngine?,
    hasMicrophoneTap: Bool
  ) {
    self.recognizer = recognizer
    self.request = request
    self.audioBufferRequest = audioBufferRequest
    self.delegate = delegate
    self.task = task
    self.audioEngine = audioEngine
    self.hasMicrophoneTap = hasMicrophoneTap
  }

  private func stopAudioEngineIfNeeded() {
    guard let audioEngine else { return }
    audioEngine.stop()
    if hasMicrophoneTap {
      audioEngine.inputNode.removeTap(onBus: 0)
    }
  }

  func finish() {
    stopAudioEngineIfNeeded()
    task.finish()
  }

  func cancel() {
    stopAudioEngineIfNeeded()
    task.cancel()
  }

  func endAudio() {
    stopAudioEngineIfNeeded()
    audioBufferRequest?.endAudio()
  }

  func deactivate() {
    delegate.deactivate()
  }

  func cleanupAndRelease(cancelTask: Bool) {
    guard !cleanedUp else { return }
    cleanedUp = true
    delegate.deactivate()
    if cancelTask {
      cancel()
    } else {
      stopAudioEngineIfNeeded()
    }
  }
}

private func spxMakeAudioFormatPayload(_ format: AVAudioFormat) -> SPXAudioFormatPayload {
  SPXAudioFormatPayload(
    sampleRate: format.sampleRate,
    channelCount: Int(format.channelCount),
    isInterleaved: format.isInterleaved,
    commonFormat: Int(format.commonFormat.rawValue)
  )
}

private func spxTaskBox(_ token: UnsafeMutableRawPointer?) throws -> SPTaskBox {
  guard let token else {
    throw SPXBridgeError.invalidArgument("missing recognition task token")
  }
  return spxUnretained(token)
}

private func spxStartTask(
  requestFactory: () throws -> SFSpeechRecognitionRequest,
  audioBufferRequestFactory: () throws -> SFSpeechAudioBufferRecognitionRequest?,
  localeId: UnsafePointer<CChar>?,
  recognizerJson: UnsafePointer<CChar>?,
  requestJson: UnsafePointer<CChar>?,
  callback: @escaping SPTaskEventCallback,
  userInfo: UnsafeMutableRawPointer?
) throws -> UnsafeMutableRawPointer {
  try spxEnsureAuthorized()
  let recognizerPayload = try spxDecodeJSONIfPresent(recognizerJson, as: SPXRecognizerPayload.self)
  let requestPayload = try spxDecodeJSONIfPresent(requestJson, as: SPXRequestPayload.self)
  let recognizer = try spxCreateRecognizer(localeId: localeId, recognizerPayload: recognizerPayload)
  guard recognizer.isAvailable else {
    throw SPXBridgeError.recognizerUnavailable("recognizer is unavailable for this locale")
  }

  let request = try requestFactory()
  try spxApplyRequestPayload(requestPayload, recognizerPayload: recognizerPayload, to: request)
  let audioBufferRequest = try audioBufferRequestFactory()
  let delegate = SPRustTaskDelegate(callback: callback, userInfo: userInfo)
  let task = recognizer.recognitionTask(with: request, delegate: delegate)
  let taskBox = SPTaskBox(
    recognizer: recognizer,
    request: request,
    audioBufferRequest: audioBufferRequest,
    delegate: delegate,
    task: task,
    audioEngine: nil,
    hasMicrophoneTap: false
  )
  return spxRetain(taskBox)
}

@_cdecl("sp_start_url_task")
public func sp_start_url_task(
  _ audioPath: UnsafePointer<CChar>,
  _ localeId: UnsafePointer<CChar>?,
  _ recognizerJson: UnsafePointer<CChar>?,
  _ requestJson: UnsafePointer<CChar>?,
  _ callback: @escaping SPTaskEventCallback,
  _ userInfo: UnsafeMutableRawPointer?,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
  do {
    let path = String(cString: audioPath)
    guard FileManager.default.fileExists(atPath: path) else {
      throw SPXBridgeError.audioLoadFailed("audio file does not exist: \(path)")
    }
    return try spxStartTask(
      requestFactory: { SFSpeechURLRecognitionRequest(url: URL(fileURLWithPath: path)) },
      audioBufferRequestFactory: { nil },
      localeId: localeId,
      recognizerJson: recognizerJson,
      requestJson: requestJson,
      callback: callback,
      userInfo: userInfo
    )
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return nil
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return nil
  }
}

@_cdecl("sp_start_audio_buffer_task")
public func sp_start_audio_buffer_task(
  _ localeId: UnsafePointer<CChar>?,
  _ recognizerJson: UnsafePointer<CChar>?,
  _ requestJson: UnsafePointer<CChar>?,
  _ callback: @escaping SPTaskEventCallback,
  _ userInfo: UnsafeMutableRawPointer?,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
  do {
    let recognizerPayload = try spxDecodeJSONIfPresent(
      recognizerJson, as: SPXRecognizerPayload.self)
    let requestPayload = try spxDecodeJSONIfPresent(requestJson, as: SPXRequestPayload.self)
    let recognizer = try spxCreateRecognizer(
      localeId: localeId, recognizerPayload: recognizerPayload)
    guard recognizer.isAvailable else {
      throw SPXBridgeError.recognizerUnavailable("recognizer is unavailable for this locale")
    }
    let request = SFSpeechAudioBufferRecognitionRequest()
    try spxApplyRequestPayload(requestPayload, recognizerPayload: recognizerPayload, to: request)
    let delegate = SPRustTaskDelegate(callback: callback, userInfo: userInfo)
    let task = recognizer.recognitionTask(with: request, delegate: delegate)
    let taskBox = SPTaskBox(
      recognizer: recognizer,
      request: request,
      audioBufferRequest: request,
      delegate: delegate,
      task: task,
      audioEngine: nil,
      hasMicrophoneTap: false
    )
    return spxRetain(taskBox)
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return nil
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return nil
  }
}

@_cdecl("sp_start_microphone_task")
public func sp_start_microphone_task(
  _ localeId: UnsafePointer<CChar>?,
  _ recognizerJson: UnsafePointer<CChar>?,
  _ requestJson: UnsafePointer<CChar>?,
  _ callback: @escaping SPTaskEventCallback,
  _ userInfo: UnsafeMutableRawPointer?,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
  do {
    try spxEnsureAuthorized()
    let recognizerPayload = try spxDecodeJSONIfPresent(
      recognizerJson, as: SPXRecognizerPayload.self)
    let requestPayload = try spxDecodeJSONIfPresent(requestJson, as: SPXRequestPayload.self)
    let recognizer = try spxCreateRecognizer(
      localeId: localeId, recognizerPayload: recognizerPayload)
    guard recognizer.isAvailable else {
      throw SPXBridgeError.recognizerUnavailable("recognizer is unavailable for this locale")
    }

    let request = SFSpeechAudioBufferRecognitionRequest()
    try spxApplyRequestPayload(requestPayload, recognizerPayload: recognizerPayload, to: request)
    let delegate = SPRustTaskDelegate(callback: callback, userInfo: userInfo)
    let task = recognizer.recognitionTask(with: request, delegate: delegate)

    let audioEngine = AVAudioEngine()
    let inputNode = audioEngine.inputNode
    let nativeAudioFormat = request.nativeAudioFormat
    inputNode.installTap(onBus: 0, bufferSize: 1024, format: nativeAudioFormat) { buffer, _ in
      request.append(buffer)
    }
    audioEngine.prepare()
    try audioEngine.start()

    let taskBox = SPTaskBox(
      recognizer: recognizer,
      request: request,
      audioBufferRequest: request,
      delegate: delegate,
      task: task,
      audioEngine: audioEngine,
      hasMicrophoneTap: true
    )
    return spxRetain(taskBox)
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return nil
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return nil
  }
}

@_cdecl("sp_task_finish")
public func sp_task_finish(_ token: UnsafeMutableRawPointer?) {
  guard let taskBox = try? spxTaskBox(token) else { return }
  taskBox.finish()
}

@_cdecl("sp_task_cancel")
public func sp_task_cancel(_ token: UnsafeMutableRawPointer?) {
  guard let taskBox = try? spxTaskBox(token) else { return }
  taskBox.cancel()
}

@_cdecl("sp_task_state")
public func sp_task_state(_ token: UnsafeMutableRawPointer?) -> Int32 {
  guard let taskBox = try? spxTaskBox(token) else { return -1 }
  return Int32(taskBox.task.state.rawValue)
}

@_cdecl("sp_task_is_finishing")
public func sp_task_is_finishing(_ token: UnsafeMutableRawPointer?) -> Bool {
  guard let taskBox = try? spxTaskBox(token) else { return false }
  return taskBox.task.isFinishing
}

@_cdecl("sp_task_is_cancelled")
public func sp_task_is_cancelled(_ token: UnsafeMutableRawPointer?) -> Bool {
  guard let taskBox = try? spxTaskBox(token) else { return false }
  return taskBox.task.isCancelled
}

@_cdecl("sp_task_error_json")
public func sp_task_error_json(_ token: UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>? {
  guard let taskBox = try? spxTaskBox(token), let error = taskBox.task.error as NSError? else {
    return nil
  }
  do {
    return spxCString(try spxEncodeJSON(spxEncodeTaskError(error)))
  } catch {
    return nil
  }
}

@_cdecl("sp_task_release")
public func sp_task_release(_ token: UnsafeMutableRawPointer?) {
  guard let token, let taskBox = try? spxTaskBox(token) else { return }
  taskBox.cleanupAndRelease(cancelTask: true)
  spxRelease(token)
}

@_cdecl("sp_audio_buffer_request_native_format_json")
public func sp_audio_buffer_request_native_format_json() -> UnsafeMutablePointer<CChar>? {
  let request = SFSpeechAudioBufferRecognitionRequest()
  do {
    return spxCString(try spxEncodeJSON(spxMakeAudioFormatPayload(request.nativeAudioFormat)))
  } catch {
    return nil
  }
}

@_cdecl("sp_audio_buffer_task_end_audio")
public func sp_audio_buffer_task_end_audio(_ token: UnsafeMutableRawPointer?) {
  guard let taskBox = try? spxTaskBox(token) else { return }
  taskBox.endAudio()
}

@_cdecl("sp_audio_buffer_task_native_format_json")
public func sp_audio_buffer_task_native_format_json(_ token: UnsafeMutableRawPointer?)
  -> UnsafeMutablePointer<CChar>?
{
  guard let taskBox = try? spxTaskBox(token), let audioBufferRequest = taskBox.audioBufferRequest
  else {
    return nil
  }
  do {
    return spxCString(
      try spxEncodeJSON(spxMakeAudioFormatPayload(audioBufferRequest.nativeAudioFormat)))
  } catch {
    return nil
  }
}

private func spxMakeFloatBuffer(
  samples: UnsafePointer<Float>,
  sampleCount: Int,
  sampleRate: Double,
  channels: Int,
  interleaved: Bool
) throws -> AVAudioPCMBuffer {
  guard channels > 0, sampleCount >= 0, sampleCount % channels == 0 else {
    throw SPXBridgeError.invalidArgument(
      "sample count must be a non-negative multiple of channel count")
  }
  let frameCount = sampleCount / channels
  guard
    let format = AVAudioFormat(
      commonFormat: .pcmFormatFloat32,
      sampleRate: sampleRate,
      channels: AVAudioChannelCount(channels),
      interleaved: false
    ),
    let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: AVAudioFrameCount(frameCount)),
    let channelData = buffer.floatChannelData
  else {
    throw SPXBridgeError.invalidArgument("unable to allocate AVAudioPCMBuffer for Float32 samples")
  }

  buffer.frameLength = AVAudioFrameCount(frameCount)
  let input = UnsafeBufferPointer(start: samples, count: sampleCount)
  guard let baseAddress = input.baseAddress else { return buffer }

  for channel in 0..<channels {
    let destination = channelData[channel]
    if interleaved {
      for frame in 0..<frameCount {
        destination[frame] = baseAddress[(frame * channels) + channel]
      }
    } else {
      destination.update(from: baseAddress.advanced(by: channel * frameCount), count: frameCount)
    }
  }
  return buffer
}

private func spxMakeInt16Buffer(
  samples: UnsafePointer<Int16>,
  sampleCount: Int,
  sampleRate: Double,
  channels: Int,
  interleaved: Bool
) throws -> AVAudioPCMBuffer {
  guard channels > 0, sampleCount >= 0, sampleCount % channels == 0 else {
    throw SPXBridgeError.invalidArgument(
      "sample count must be a non-negative multiple of channel count")
  }
  let frameCount = sampleCount / channels
  guard
    let format = AVAudioFormat(
      commonFormat: .pcmFormatInt16,
      sampleRate: sampleRate,
      channels: AVAudioChannelCount(channels),
      interleaved: false
    ),
    let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: AVAudioFrameCount(frameCount)),
    let channelData = buffer.int16ChannelData
  else {
    throw SPXBridgeError.invalidArgument("unable to allocate AVAudioPCMBuffer for Int16 samples")
  }

  buffer.frameLength = AVAudioFrameCount(frameCount)
  let input = UnsafeBufferPointer(start: samples, count: sampleCount)
  guard let baseAddress = input.baseAddress else { return buffer }

  for channel in 0..<channels {
    let destination = channelData[channel]
    if interleaved {
      for frame in 0..<frameCount {
        destination[frame] = baseAddress[(frame * channels) + channel]
      }
    } else {
      destination.update(from: baseAddress.advanced(by: channel * frameCount), count: frameCount)
    }
  }
  return buffer
}

@_cdecl("sp_audio_buffer_task_append_f32")
public func sp_audio_buffer_task_append_f32(
  _ token: UnsafeMutableRawPointer?,
  _ samples: UnsafePointer<Float>?,
  _ sampleCount: Int,
  _ sampleRate: Double,
  _ channels: Int32,
  _ interleaved: Bool,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    guard let taskBox = try? spxTaskBox(token), let audioBufferRequest = taskBox.audioBufferRequest
    else {
      throw SPXBridgeError.invalidArgument(
        "task token does not reference an audio-buffer recognition request")
    }
    guard let samples else {
      throw SPXBridgeError.invalidArgument("missing Float32 samples pointer")
    }
    let buffer = try spxMakeFloatBuffer(
      samples: samples,
      sampleCount: sampleCount,
      sampleRate: sampleRate,
      channels: Int(channels),
      interleaved: interleaved
    )
    audioBufferRequest.append(buffer)
    return SPX_OK
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_audio_buffer_task_append_i16")
public func sp_audio_buffer_task_append_i16(
  _ token: UnsafeMutableRawPointer?,
  _ samples: UnsafePointer<Int16>?,
  _ sampleCount: Int,
  _ sampleRate: Double,
  _ channels: Int32,
  _ interleaved: Bool,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    guard let taskBox = try? spxTaskBox(token), let audioBufferRequest = taskBox.audioBufferRequest
    else {
      throw SPXBridgeError.invalidArgument(
        "task token does not reference an audio-buffer recognition request")
    }
    guard let samples else {
      throw SPXBridgeError.invalidArgument("missing Int16 samples pointer")
    }
    let buffer = try spxMakeInt16Buffer(
      samples: samples,
      sampleCount: sampleCount,
      sampleRate: sampleRate,
      channels: Int(channels),
      interleaved: interleaved
    )
    audioBufferRequest.append(buffer)
    return SPX_OK
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_audio_buffer_task_append_pcm_buffer_raw")
public func sp_audio_buffer_task_append_pcm_buffer_raw(
  _ token: UnsafeMutableRawPointer?,
  _ bufferPointer: UnsafeMutableRawPointer?,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    guard let taskBox = try? spxTaskBox(token), let audioBufferRequest = taskBox.audioBufferRequest
    else {
      throw SPXBridgeError.invalidArgument(
        "task token does not reference an audio-buffer recognition request")
    }
    guard let bufferPointer else {
      throw SPXBridgeError.invalidArgument("missing AVAudioPCMBuffer pointer")
    }
    let buffer = Unmanaged<AVAudioPCMBuffer>.fromOpaque(bufferPointer).takeUnretainedValue()
    audioBufferRequest.append(buffer)
    return SPX_OK
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_audio_buffer_task_append_sample_buffer_raw")
public func sp_audio_buffer_task_append_sample_buffer_raw(
  _ token: UnsafeMutableRawPointer?,
  _ sampleBufferPointer: UnsafeMutableRawPointer?,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    guard let taskBox = try? spxTaskBox(token), let audioBufferRequest = taskBox.audioBufferRequest
    else {
      throw SPXBridgeError.invalidArgument(
        "task token does not reference an audio-buffer recognition request")
    }
    guard let sampleBufferPointer else {
      throw SPXBridgeError.invalidArgument("missing CMSampleBuffer pointer")
    }
    let sampleBuffer = unsafeBitCast(sampleBufferPointer, to: CMSampleBuffer.self)
    audioBufferRequest.appendAudioSampleBuffer(sampleBuffer)
    return SPX_OK
  } catch let error as SPXBridgeError {
    outErrorMessage?.pointee = spxCString(error.description)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}
