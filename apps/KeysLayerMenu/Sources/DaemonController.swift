import AppKit
import Foundation

struct KeyboardItem: Identifiable, Equatable {
  var id: String { name }
  let name: String
  /// `true` = keyboard produces input (remapped). `false` = muted completely.
  var enabled: Bool
}

/// Talks to `keys-layer-ctl`.
/// Turn On/Off need admin once. Mute/unmute writes user config (no password).
enum DaemonController {
  enum Action: String {
    case status, on, off, restart, devices
  }

  struct Result {
    let exitCode: Int32
    let output: String
  }

  private static let cachedNamesKey = "keysLayer.cachedKeyboardNames"

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
    runCtl("status").exitCode == 0
  }

  static func currentSessionDeniedPrivacy() -> Bool {
    let log = "/var/log/keys-layer.log"
    guard let data = try? Data(contentsOf: URL(fileURLWithPath: log)),
          let text = String(data: data, encoding: .utf8) ?? String(data: data, encoding: .isoLatin1)
    else { return false }

    let marker = "keys-layer (DriverKit) running"
    let session: String
    if let range = text.range(of: marker, options: .backwards) {
      session = String(text[range.lowerBound...])
    } else {
      session = String(text.suffix(8192))
    }
    return session.contains("not permitted")
  }

  /// LaunchDaemon start/stop — password once (macOS caches briefly afterward).
  static func setEnabled(_ enabled: Bool) throws {
    let action = enabled ? "on" : "off"
    let r = try runPrivileged(action)
    if r.exitCode != 0 {
      throw ControllerError.commandFailed(action, r.output)
    }
  }

  /// Prefer cached/config list (no password). Hardware scan still needs admin.
  static func listKeyboards(preferHardware: Bool = false) throws -> [KeyboardItem] {
    if preferHardware {
      let r = try runPrivileged("devices")
      if r.exitCode != 0 {
        throw ControllerError.commandFailed("devices", r.output)
      }
      var items: [KeyboardItem] = []
      for line in r.output.split(whereSeparator: \.isNewline) {
        let parts = line.split(separator: "\t", maxSplits: 1).map(String.init)
        guard parts.count == 2 else { continue }
        let enabled = parts[1].trimmingCharacters(in: .whitespaces) == "on"
        items.append(KeyboardItem(name: parts[0], enabled: enabled))
      }
      rememberNames(items.map(\.name))
      if !items.isEmpty { return items }
    }
    return keyboardsFromConfigAndCache()
  }

  /// Mute/unmute via user config — no admin prompt. Daemon hot-reloads.
  static func setKeyboardEnabled(name: String, enabled: Bool) throws {
    let action = enabled ? "device-enable" : "device-disable"
    let r = runCtl(action, extraArg: name)
    if r.exitCode != 0 {
      throw ControllerError.commandFailed(action, r.output)
    }
  }

  static func keyboardsFromConfigAndCache() -> [KeyboardItem] {
    let remap = readStringArray(key: "devices")
    let muted = readStringArray(key: "disabled_devices")
    var names = rememberedNames()
    for p in remap where !names.contains(where: { $0.caseInsensitiveCompare(p) == .orderedSame }) {
      names.append(displayName(forPattern: p))
    }
    for p in muted where !names.contains(where: {
      $0.localizedCaseInsensitiveContains(p) || p.localizedCaseInsensitiveContains($0)
    }) {
      names.append(displayName(forPattern: p))
    }
    if names.isEmpty {
      names = ["Apple Internal Keyboard / Trackpad"]
    }

    return names.map { name in
      let isMuted = muted.contains { pattern in
        name.range(of: pattern, options: [.caseInsensitive, .diacriticInsensitive]) != nil
          || pattern.range(of: name, options: [.caseInsensitive, .diacriticInsensitive]) != nil
      }
      return KeyboardItem(name: name, enabled: !isMuted)
    }
    .sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
  }

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

  private static func configURL() -> URL {
    FileManager.default.homeDirectoryForCurrentUser
      .appendingPathComponent(".config/keys-layer/config.toml")
  }

  private static func readStringArray(key: String) -> [String] {
    guard let text = try? String(contentsOf: configURL(), encoding: .utf8) else { return [] }
    let pattern = #"\#(NSRegularExpression.escapedPattern(for: key))\s*=\s*\[(.*?)\]"#
    guard let regex = try? NSRegularExpression(
      pattern: pattern,
      options: [.dotMatchesLineSeparators]
    ) else { return [] }
    let range = NSRange(text.startIndex..<text.endIndex, in: text)
    guard let match = regex.firstMatch(in: text, options: [], range: range),
          let bodyRange = Range(match.range(at: 1), in: text)
    else { return [] }
    let body = String(text[bodyRange])
    let partRegex = try? NSRegularExpression(pattern: #""([^"]*)"|'([^']*)'"#)
    var out: [String] = []
    let bodyRangeNS = NSRange(body.startIndex..<body.endIndex, in: body)
    partRegex?.enumerateMatches(in: body, options: [], range: bodyRangeNS) { m, _, _ in
      guard let m else { return }
      for i in 1...2 {
        if let r = Range(m.range(at: i), in: body) {
          let s = String(body[r])
          if !s.isEmpty { out.append(s) }
          break
        }
      }
    }
    return out
  }

  private static func displayName(forPattern pattern: String) -> String {
    if pattern.localizedCaseInsensitiveContains("Apple Internal") {
      return "Apple Internal Keyboard / Trackpad"
    }
    return pattern
  }

  private static func rememberedNames() -> [String] {
    UserDefaults.standard.stringArray(forKey: cachedNamesKey) ?? []
  }

  private static func rememberNames(_ names: [String]) {
    guard !names.isEmpty else { return }
    UserDefaults.standard.set(names, forKey: cachedNamesKey)
  }

  private static func runCtl(_ action: String, extraArg: String? = nil) -> Result {
    let proc = Process()
    proc.executableURL = URL(fileURLWithPath: "/bin/bash")
    var cmd = shellEscape(ctlPath()) + " " + shellEscape(action)
    if let extraArg {
      cmd += " " + shellEscape(extraArg)
    }
    proc.arguments = ["-lc", cmd]
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

  private static func runPrivileged(_ action: String, extraArg: String? = nil) throws -> Result {
    let ctl = ctlPath()
    var inner = "\(shellEscape(ctl)) \(action)"
    if let extraArg {
      inner += " \(shellEscape(extraArg))"
    }
    // bash -lc keeps quoting reliable under AppleScript admin.
    let shell = "/bin/bash -lc \(shellEscape(inner))"
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
      throw ControllerError.commandFailed(action, msg)
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
