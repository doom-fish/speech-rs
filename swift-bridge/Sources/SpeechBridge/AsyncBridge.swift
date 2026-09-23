import Foundation

final class SPXAsyncBridgeState<T: Sendable>: @unchecked Sendable {
  private let lock = NSLock()
  private var result: Result<T, SPXBridgeError>?

  func store(_ result: Result<T, SPXBridgeError>) {
    lock.lock()
    self.result = result
    lock.unlock()
  }

  func load() -> Result<T, SPXBridgeError>? {
    lock.lock()
    defer { lock.unlock() }
    return result
  }
}

func spxErrorMessage(_ error: SPXBridgeError) -> String {
  if case let .framework(wrapped) = error,
    let json = try? spxEncodeJSON(spxEncodeTaskError(wrapped as NSError))
  {
    return json
  }
  return error.description
}

func spxWriteError(
  _ error: SPXBridgeError,
  to outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) {
  outErrorMessage?.pointee = spxCString(spxErrorMessage(error))
}

func spxRunAsyncBridgeBlocking<T: Sendable>(
  timeoutSeconds: Double? = 120,
  operation: @escaping @Sendable () async throws -> T
) throws -> T {
  let semaphore = DispatchSemaphore(value: 0)
  let state = SPXAsyncBridgeState<T>()
  let task = Task {
    do {
      state.store(.success(try await operation()))
    } catch let error as SPXBridgeError {
      state.store(.failure(error))
    } catch {
      if let wrapped = error as NSError? {
        state.store(.failure(.framework(wrapped)))
      } else {
        state.store(.failure(.unknown(error.localizedDescription)))
      }
    }
    semaphore.signal()
  }

  let deadline: DispatchTime
  if let timeoutSeconds, timeoutSeconds.isFinite, timeoutSeconds < 1_000_000_000 {
    deadline = .now() + max(timeoutSeconds, 0)
  } else {
    deadline = .distantFuture
  }
  if semaphore.wait(timeout: deadline) == .timedOut {
    task.cancel()
    throw SPXBridgeError.timedOut(
      "operation timed out after \(String(format: "%g", timeoutSeconds ?? 0))s")
  }

  guard let result = state.load() else {
    throw SPXBridgeError.unknown("operation completed without a result")
  }

  switch result {
  case .success(let value):
    return value
  case .failure(let error):
    throw error
  }
}
