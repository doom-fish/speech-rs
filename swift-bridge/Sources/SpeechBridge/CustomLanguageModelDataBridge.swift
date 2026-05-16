import Foundation
import Speech

private func spxCustomLanguageModelDataUnavailableMessage() -> String {
  "SFCustomLanguageModelData requires macOS 14 or newer"
}

struct SPXPhraseCountPayload: Codable, Sendable {
  var phrase: String
  var count: Int
}

struct SPXTemplateInsertableItemPayload: Codable, Sendable {
  var kind: String
  var body: String?
  var count: Int?
  var components: [SPXTemplateInsertableItemPayload]?
}

struct SPXDataInsertableItemPayload: Codable, Sendable {
  var kind: String
  var phrase: String?
  var count: Int?
  var grapheme: String?
  var phonemes: [String]?
  var values: [SPXPhraseCountPayload]?
  var templates: [SPXTemplateInsertableItemPayload]?
  var classes: [String: [String]]?
}

struct SPXCustomLanguageModelDataPayload: Codable, Sendable {
  var localeIdentifier: String
  var identifier: String
  var version: String
  var items: [SPXDataInsertableItemPayload]
}

@available(macOS 14.0, *)
private func spxMakePhraseCount(_ payload: SPXPhraseCountPayload) -> SFCustomLanguageModelData.PhraseCount {
  .init(phrase: payload.phrase, count: payload.count)
}

@available(macOS 14.0, *)
private func spxMakeCustomPronunciation(
  grapheme: String,
  phonemes: [String]
) -> SFCustomLanguageModelData.CustomPronunciation {
  .init(grapheme: grapheme, phonemes: phonemes)
}

@available(macOS 14.0, *)
private func spxMakeTemplateInsertableItem(_ payload: SPXTemplateInsertableItemPayload)
  throws -> any TemplateInsertable
{
  switch payload.kind {
  case "template":
    guard let body = payload.body, let count = payload.count else {
      throw SPXBridgeError.invalidArgument("template payload requires body and count")
    }
    return SFCustomLanguageModelData.TemplatePhraseCountGenerator.Template(body, count: count)
  case "compoundTemplate":
    let components = try (payload.components ?? []).map(spxMakeTemplateInsertableItem)
    return SFCustomLanguageModelData.CompoundTemplate(components)
  default:
    throw SPXBridgeError.invalidArgument(
      "unsupported template insertable kind: \(payload.kind)")
  }
}

@available(macOS 14.0, *)
private func spxMakeTemplateInsertable(from payloads: [SPXTemplateInsertableItemPayload]) throws
  -> any TemplateInsertable
{
  if payloads.count == 1, let first = payloads.first {
    return try spxMakeTemplateInsertableItem(first)
  }
  return SFCustomLanguageModelData.CompoundTemplate(try payloads.map(spxMakeTemplateInsertableItem))
}

@available(macOS 14.0, *)
private func spxInsertDataItem(
  _ payload: SPXDataInsertableItemPayload,
  into data: SFCustomLanguageModelData
) throws {
  switch payload.kind {
  case "phraseCount":
    guard let phrase = payload.phrase, let count = payload.count else {
      throw SPXBridgeError.invalidArgument("phrase count payload requires phrase and count")
    }
    data.insert(phraseCount: .init(phrase: phrase, count: count))
  case "customPronunciation":
    guard let grapheme = payload.grapheme, let phonemes = payload.phonemes else {
      throw SPXBridgeError.invalidArgument(
        "custom pronunciation payload requires grapheme and phonemes")
    }
    data.insert(term: spxMakeCustomPronunciation(grapheme: grapheme, phonemes: phonemes))
  case "phraseCountGenerator":
    for value in payload.values ?? [] {
      data.insert(phraseCount: spxMakePhraseCount(value))
    }
  case "templatePhraseCountGenerator":
    let generator = SFCustomLanguageModelData.TemplatePhraseCountGenerator()
    for (className, values) in payload.classes ?? [:] {
      generator.define(className: className, values: values)
    }
    for template in payload.templates ?? [] {
      let insertable = try spxMakeTemplateInsertableItem(template)
      insertable.insert(generator: generator)
    }
    data.insert(phraseCountGenerator: generator)
  case "phraseCountsFromTemplates":
    let insertable = try spxMakeTemplateInsertable(from: payload.templates ?? [])
    let phraseCounts = SFCustomLanguageModelData.PhraseCountsFromTemplates(
      classes: payload.classes ?? [:]
    ) {
      insertable
    }
    phraseCounts.insert(data: data)
  default:
    throw SPXBridgeError.invalidArgument("unsupported data insertable kind: \(payload.kind)")
  }
}

@available(macOS 14.0, *)
private func spxMakeCustomLanguageModelData(from payload: SPXCustomLanguageModelDataPayload)
  throws -> SFCustomLanguageModelData
{
  let data = SFCustomLanguageModelData(
    locale: Locale(identifier: payload.localeIdentifier),
    identifier: payload.identifier,
    version: payload.version
  )
  for item in payload.items {
    try spxInsertDataItem(item, into: data)
  }
  return data
}

@_cdecl("sp_custom_language_model_supported_phonemes_json")
public func sp_custom_language_model_supported_phonemes_json(
  _ localeId: UnsafePointer<CChar>,
  _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    if #available(macOS 14.0, *) {
      let locale = Locale(identifier: String(cString: localeId))
      outJson.pointee = spxCString(try spxEncodeJSON(SFCustomLanguageModelData.supportedPhonemes(locale: locale)))
      return SPX_OK
    }
    throw SPXBridgeError.recognizerUnavailable(spxCustomLanguageModelDataUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_custom_language_model_export")
public func sp_custom_language_model_export(
  _ dataJson: UnsafePointer<CChar>,
  _ outputPath: UnsafePointer<CChar>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    if #available(macOS 14.0, *) {
      let payload = try spxDecodeJSON(dataJson, as: SPXCustomLanguageModelDataPayload.self)
      let destination = URL(fileURLWithPath: String(cString: outputPath))
      _ = try spxRunAsyncBridgeBlocking { () async throws -> Bool in
        try await spxMakeCustomLanguageModelData(from: payload).export(to: destination)
        return true
      }
      return SPX_OK
    }
    throw SPXBridgeError.recognizerUnavailable(spxCustomLanguageModelDataUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}
