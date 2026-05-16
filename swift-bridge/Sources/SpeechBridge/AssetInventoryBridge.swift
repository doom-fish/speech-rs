import Foundation
import Speech

private func spxAssetInventoryUnavailableMessage() -> String {
  "AssetInventory requires the macOS 26 SDK and macOS 26 runtime"
}

struct SPXAssetInstallationProgressPayload: Codable {
  var fractionCompleted: Double
  var completedUnitCount: Int64
  var totalUnitCount: Int64
  var isFinished: Bool
  var localizedDescription: String?
  var localizedAdditionalDescription: String?
}

#if SPEECH_HAS_MACOS26_SDK
@available(macOS 26.0, *)
private final class SPXAssetInstallationRequestBox: NSObject {
  let request: AssetInstallationRequest

  init(request: AssetInstallationRequest) {
    self.request = request
  }
}

@available(macOS 26.0, *)
private func spxAssetInstallationRequestBox(_ token: UnsafeMutableRawPointer?) throws
  -> SPXAssetInstallationRequestBox
{
  guard let token else {
    throw SPXBridgeError.invalidArgument("missing asset installation request token")
  }
  return spxUnretained(token)
}

@available(macOS 26.0, *)
private func spxAssetInventoryStatusRaw(_ status: AssetInventory.Status) -> Int32 {
  switch status {
  case .unsupported:
    return 0
  case .supported:
    return 1
  case .downloading:
    return 2
  case .installed:
    return 3
  @unknown default:
    return 0
  }
}

@available(macOS 26.0, *)
private func spxMakeAssetInstallationProgressPayload(_ progress: Progress)
  -> SPXAssetInstallationProgressPayload
{
  SPXAssetInstallationProgressPayload(
    fractionCompleted: progress.fractionCompleted,
    completedUnitCount: progress.completedUnitCount,
    totalUnitCount: progress.totalUnitCount,
    isFinished: progress.isFinished,
    localizedDescription: progress.localizedDescription,
    localizedAdditionalDescription: progress.localizedAdditionalDescription
  )
}
#endif

@_cdecl("sp_asset_inventory_maximum_reserved_locales")
public func sp_asset_inventory_maximum_reserved_locales(
  _ outValue: UnsafeMutablePointer<Int>?,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      outValue?.pointee = AssetInventory.maximumReservedLocales
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxAssetInventoryUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_asset_inventory_reserved_locales_json")
public func sp_asset_inventory_reserved_locales_json(
  _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let json = try spxRunAsyncBridgeBlocking { () async throws -> String in
        try spxEncodeJSON((await AssetInventory.reservedLocales).map(\.identifier).sorted())
      }
      outJson.pointee = spxCString(json)
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxAssetInventoryUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_asset_inventory_reserve_locale")
public func sp_asset_inventory_reserve_locale(
  _ localeId: UnsafePointer<CChar>,
  _ outReserved: UnsafeMutablePointer<Bool>?,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let locale = Locale(identifier: String(cString: localeId))
      let reserved = try spxRunAsyncBridgeBlocking { () async throws -> Bool in
        try await AssetInventory.reserve(locale: locale)
      }
      outReserved?.pointee = reserved
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxAssetInventoryUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_asset_inventory_release_locale")
public func sp_asset_inventory_release_locale(
  _ localeId: UnsafePointer<CChar>,
  _ outReleased: UnsafeMutablePointer<Bool>?,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let locale = Locale(identifier: String(cString: localeId))
      let released = try spxRunAsyncBridgeBlocking { () async throws -> Bool in
        await AssetInventory.release(reservedLocale: locale)
      }
      outReleased?.pointee = released
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxAssetInventoryUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_asset_inventory_status_for_modules")
public func sp_asset_inventory_status_for_modules(
  _ modulesJson: UnsafePointer<CChar>,
  _ outStatus: UnsafeMutablePointer<Int32>?,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let payload = try spxDecodeJSON(modulesJson, as: [SPXSpeechModulePayload].self)
      let status = try spxRunAsyncBridgeBlocking { () async throws -> Int32 in
        let modules = try payload.map { try spxMakeSpeechModule(from: $0) }
        return spxAssetInventoryStatusRaw(await AssetInventory.status(forModules: modules))
      }
      outStatus?.pointee = status
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxAssetInventoryUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_asset_inventory_installation_request_for_modules")
public func sp_asset_inventory_installation_request_for_modules(
  _ modulesJson: UnsafePointer<CChar>,
  _ outHasRequest: UnsafeMutablePointer<Bool>?,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let payload = try spxDecodeJSON(modulesJson, as: [SPXSpeechModulePayload].self)
      let request = try spxRunAsyncBridgeBlocking { () async throws -> AssetInstallationRequest? in
        let modules = try payload.map { try spxMakeSpeechModule(from: $0) }
        return try await AssetInventory.assetInstallationRequest(supporting: modules)
      }
      outHasRequest?.pointee = request != nil
      return request.map { spxRetain(SPXAssetInstallationRequestBox(request: $0)) }
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxAssetInventoryUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    outHasRequest?.pointee = false
    return nil
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    outHasRequest?.pointee = false
    return nil
  }
}

@_cdecl("sp_asset_installation_request_progress_json")
public func sp_asset_installation_request_progress_json(
  _ token: UnsafeMutableRawPointer?
) -> UnsafeMutablePointer<CChar>? {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let requestBox = try spxAssetInstallationRequestBox(token)
      return spxCString(try spxEncodeJSON(spxMakeAssetInstallationProgressPayload(requestBox.request.progress)))
    }
    #endif
    return nil
  } catch {
    return nil
  }
}

@_cdecl("sp_asset_installation_request_download_and_install")
public func sp_asset_installation_request_download_and_install(
  _ token: UnsafeMutableRawPointer?,
  _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
  do {
    #if SPEECH_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
      let requestBox = try spxAssetInstallationRequestBox(token)
      _ = try spxRunAsyncBridgeBlocking { () async throws -> Bool in
        try await requestBox.request.downloadAndInstall()
        return true
      }
      return SPX_OK
    }
    #endif
    throw SPXBridgeError.recognizerUnavailable(spxAssetInventoryUnavailableMessage())
  } catch let error as SPXBridgeError {
    spxWriteError(error, to: outErrorMessage)
    return error.statusCode
  } catch {
    outErrorMessage?.pointee = spxCString(error.localizedDescription)
    return SPX_UNKNOWN
  }
}

@_cdecl("sp_asset_installation_request_release")
public func sp_asset_installation_request_release(_ token: UnsafeMutableRawPointer?) {
  guard let token else { return }
  spxRelease(token)
}
