import AppKit
import Foundation

/// Talks to `keys-layer-ctl` (status is unprivileged; on/off need admin).
enum DaemonController {
  enum Action: String {
    case status, on, off
  }

  struct Result {
    let exitCode: Int32
    let output: String
  }

  /// Remapper binary Privacy must be granted for (not this menu app).
  static func remapperBinaryPath() -> String {
    if let env = ProcessInfo.processInfo.environment["KEYS_LAYER_BIN"], !env.isEmpty {
      return env
    }
    for path in [
      "/opt/homebrew/opt/keys-layer/bin/keys-layer",
      "/usr/local/opt/keys-layer/bin/keys-layer",
      "/usr/local/bin/keys-layer",
    ] where FileManager.default.isExecutableFile(atPath: path) {
      return path
    }
    return "/opt/homebrew/opt/keys-layer/bin/keys-layer"
  }

  static func ctlPath() -> String {
    if let env = ProcessInfo.processInfo.environment["KEYS_LAYER_CTL"], !env.isEmpty {
      return env
    }
    let macOS = Bundle.main.bundleURL
      .appendingPathComponent("Contents/MacOS", isDirectory: true)
    let candidates = [
      macOS.appendingPathComponent("keys-layer-ctl").path,
      "/opt/homebrew/bin/keys-layer-ctl",
      "/usr/local/bin/keys-layer-ctl",
    ]
    for path in candidates where FileManager.default.isExecutableFile(atPath: path) {
      return path
    }
    return "keys-layer-ctl"
  }

  static func statusOn() -> Bool {
    runUnprivileged(.status).exitCode == 0
  }

  /// True if the latest remapper session logged Input Monitoring denial.
  static func currentSessionDeniedPrivacy() -> Bool {
    let log = "/var/log/keys-layer.log"
    guard let data = try? Data(contentsOf: URL(fileURLWithPath: log)),
          let text = String(data: data, encoding: .utf8) ?? String(data: data, encoding: .isoLatin1)
    else { return false }

    // Only inspect lines after the last successful start banner.
    let marker = "keys-layer (DriverKit) running"
    let session: String
    if let range = text.range(of: marker, options: .backwards) {
      session = String(text[range.lowerBound...])
    } else {
      // Fall back to last 8 KiB
      session = String(text.suffix(8192))
    }
    return session.contains("not permitted")
  }

  static func setEnabled(_ enabled: Bool) throws {
    let action: Action = enabled ? .on : .off
    let r = try runPrivileged(action)
    if r.exitCode != 0 {
      throw ControllerError.commandFailed(action.rawValue, r.output)
    }
  }

  /// Open Privacy panes and put the remapper path on the clipboard.
  static func openPrivacyHelp() {
    let path = remapperBinaryPath()
    NSPasteboard.general.clearContents()
    NSPasteboard.general.setString(path, forType: .string)

    let urls = [
      "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent",
      "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
      "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension",
    ]
    for s in urls {
      if let u = URL(string: s) {
        NSWorkspace.shared.open(u)
        break
      }
    }
  }

  private static func runUnprivileged(_ action: Action) -> Result {
    let proc = Process()
    proc.executableURL = URL(fileURLWithPath: "/bin/bash")
    proc.arguments = ["-lc", shellEscape(ctlPath()) + " " + action.rawValue]
    let pipe = Pipe()
    proc.standardOutput = pipe
    proc.standardError = pipe
    do {
      try proc.run()
      proc.waitUntilExit()
    } catch {
      return Result(exitCode: 127, output: error.localizedDescription)
    }
    let data = pipe.fileHandleForReading.readDataToEndOfFile()
    let text = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    return Result(exitCode: proc.terminationStatus, output: text)
  }

  private static func runPrivileged(_ action: Action) throws -> Result {
    let ctl = ctlPath()
    let shell = "\(shellEscape(ctl)) \(action.rawValue)"
    let apple = """
    do shell script \(appleString(shell)) with administrator privileges
    """
    var error: NSDictionary?
    guard let script = NSAppleScript(source: apple) else {
      throw ControllerError.appleScriptUnavailable
    }
    let descriptor = script.executeAndReturnError(&error)
    if let error {
      let msg = error[NSAppleScript.errorMessage] as? String ?? "authorization failed"
      let code = (error[NSAppleScript.errorNumber] as? Int) ?? 1
      if code == -128 {
        throw ControllerError.cancelled
      }
      throw ControllerError.commandFailed(action.rawValue, msg)
    }
    let text = descriptor.stringValue?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    return Result(exitCode: 0, output: text)
  }

  private static func shellEscape(_ s: String) -> String {
    "'" + s.replacingOccurrences(of: "'", with: "'\\''") + "'"
  }

  private static func appleString(_ s: String) -> String {
    "\"" + s
      .replacingOccurrences(of: "\\", with: "\\\\")
      .replacingOccurrences(of: "\"", with: "\\\"")
      + "\""
  }

  enum ControllerError: LocalizedError {
    case appleScriptUnavailable
    case cancelled
    case commandFailed(String, String)

    var errorDescription: String? {
      switch self {
      case .appleScriptUnavailable:
        return "Could not run admin AppleScript."
      case .cancelled:
        return "Cancelled."
      case .commandFailed(let action, let detail):
        return "keys-layer-ctl \(action) failed: \(detail)"
      }
    }
  }
}
