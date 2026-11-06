import AppKit
import ServiceManagement
import SwiftUI

@main
struct KeysLayerMenuApp: App {
  @StateObject private var model = MenuModel()

  var body: some Scene {
    MenuBarExtra {
      ContentView(model: model)
    } label: {
      Label(
        model.isOn ? "Keys Layer On" : "Keys Layer Off",
        systemImage: model.isOn ? "keyboard.fill" : "keyboard"
      )
    }
    .menuBarExtraStyle(.menu)
  }
}

@MainActor
final class MenuModel: ObservableObject {
  @Published var isOn = false
  @Published var busy = false
  @Published var launchAtLogin = false
  @Published var needsPrivacyFix = false
  @Published var keyboards: [KeyboardItem] = []
  @Published var lastError: String?

  private var timer: Timer?
  private var didAutoRestartForPrivacy = false

  var remapperPath: String { DaemonController.remapperBinaryPath() }

  init() {
    refresh()
    refreshLaunchAtLogin()
    timer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { [weak self] _ in
      Task { @MainActor in
        self?.refresh()
      }
    }
  }

  func refresh() {
    guard !busy else { return }
    isOn = DaemonController.statusOn()
    if isOn {
      let denied = DaemonController.currentSessionDeniedPrivacy()
      needsPrivacyFix = denied
      if denied {
        lastError =
          "Input Monitoring not active. Enable it for:\n\(remapperPath)\n(not Keys Layer.app), then remapping will restart automatically — or click Fix below."
        if !didAutoRestartForPrivacy {
          didAutoRestartForPrivacy = true
          applyPrivacyRestart()
        }
      } else {
        if needsPrivacyFix { needsPrivacyFix = false }
        if lastError?.contains("Input Monitoring") == true {
          lastError = nil
        }
      }
    } else {
      needsPrivacyFix = false
      didAutoRestartForPrivacy = false
    }
  }

  func refreshLaunchAtLogin() {
    if #available(macOS 13.0, *) {
      launchAtLogin = SMAppService.mainApp.status == .enabled
    }
  }

  func refreshKeyboards() {
    guard !busy else { return }
    busy = true
    lastError = nil
    Task.detached {
      do {
        let items = try DaemonController.listKeyboards()
        await MainActor.run {
          self.keyboards = items
          self.busy = false
          if items.isEmpty {
            self.lastError = "No keyboards reported. Is VirtualHID running?"
          }
        }
      } catch {
        await MainActor.run {
          self.lastError = error.localizedDescription
          self.busy = false
        }
      }
    }
  }

  func setKeyboardEnabled(_ item: KeyboardItem, enabled: Bool) {
    guard !busy else { return }
    busy = true
    lastError = nil
    let name = item.name
    Task.detached {
      do {
        try DaemonController.setKeyboardEnabled(name: name, enabled: enabled)
        let items = try DaemonController.listKeyboards()
        await MainActor.run {
          self.keyboards = items
          self.isOn = DaemonController.statusOn()
          self.busy = false
        }
      } catch {
        await MainActor.run {
          self.lastError = error.localizedDescription
          self.busy = false
        }
      }
    }
  }

  func setEnabled(_ enabled: Bool) {
    guard !busy else { return }
    busy = true
    lastError = nil
    needsPrivacyFix = false
    if enabled {
      didAutoRestartForPrivacy = false
    }
    let target = enabled
    Task.detached {
      do {
        try DaemonController.setEnabled(target)
        if target {
          try? await Task.sleep(nanoseconds: 1_800_000_000)
        }
        let listed: [KeyboardItem]
        if target {
          listed = (try? DaemonController.listKeyboards()) ?? []
        } else {
          listed = []
        }
        await MainActor.run {
          self.isOn = DaemonController.statusOn()
          if target { self.keyboards = listed }
          self.busy = false
          self.refresh()
        }
      } catch {
        await MainActor.run {
          self.lastError = error.localizedDescription
          self.isOn = DaemonController.statusOn()
          self.busy = false
        }
      }
    }
  }

  func applyPrivacyRestart() {
    guard !busy else { return }
    busy = true
    lastError = "Restarting remapping so Privacy applies…"
    Task.detached {
      do {
        try DaemonController.setEnabled(false)
        try DaemonController.setEnabled(true)
        try? await Task.sleep(nanoseconds: 1_500_000_000)
        let keyboards = (try? DaemonController.listKeyboards()) ?? []
        await MainActor.run {
          self.keyboards = keyboards
          self.isOn = DaemonController.statusOn()
          self.busy = false
          let denied = DaemonController.currentSessionDeniedPrivacy()
          self.needsPrivacyFix = denied
          if denied {
            self.lastError =
              "Still blocked. In Privacy, enable Accessibility + Input Monitoring for:\n\(DaemonController.remapperBinaryPath())"
            DaemonController.openPrivacyHelp()
          } else {
            self.lastError = nil
          }
        }
      } catch {
        await MainActor.run {
          self.lastError = error.localizedDescription
          self.isOn = DaemonController.statusOn()
          self.busy = false
        }
      }
    }
  }

  func openPrivacySettings() {
    DaemonController.openPrivacyHelp()
    lastError =
      "Binary path copied. Enable Accessibility + Input Monitoring for that path, then click Fix Privacy / Restart."
  }

  func setLaunchAtLogin(_ enabled: Bool) {
    guard #available(macOS 13.0, *) else { return }
    do {
      if enabled {
        try SMAppService.mainApp.register()
      } else {
        try SMAppService.mainApp.unregister()
      }
      refreshLaunchAtLogin()
    } catch {
      lastError = error.localizedDescription
      refreshLaunchAtLogin()
    }
  }

  func openLog() {
    NSWorkspace.shared.open(URL(fileURLWithPath: "/var/log/keys-layer.log"))
  }

  func openConfig() {
    let home = FileManager.default.homeDirectoryForCurrentUser
    let cfg = home.appendingPathComponent(".config/keys-layer/config.toml")
    if FileManager.default.fileExists(atPath: cfg.path) {
      NSWorkspace.shared.open(cfg)
    } else {
      lastError = "No config yet — Turn Remapping On once to create it."
    }
  }

  func quit() {
    NSApp.terminate(nil)
  }

  /// Short label for the menu (full name stays the id for ctl).
  static func shortLabel(for name: String) -> String {
    if name.localizedCaseInsensitiveContains("Apple Internal") {
      return "MacBook Keyboard"
    }
    if name.count > 36 {
      return String(name.prefix(33)) + "…"
    }
    return name
  }
}

struct ContentView: View {
  @ObservedObject var model: MenuModel

  var body: some View {
    Button {
      model.setEnabled(true)
    } label: {
      HStack {
        Text("Turn Remapping On")
        if model.isOn && !model.needsPrivacyFix { Text("✓") }
      }
    }
    .disabled(model.busy || (model.isOn && !model.needsPrivacyFix))

    Button {
      model.setEnabled(false)
    } label: {
      HStack {
        Text("Turn Remapping Off")
        if !model.isOn { Text("✓") }
      }
    }
    .disabled(model.busy || !model.isOn)

    if model.needsPrivacyFix || model.isOn == false {
      Divider()
      Button("Open Privacy Settings…") { model.openPrivacySettings() }
      if model.needsPrivacyFix {
        Button("Fix Privacy / Restart Remapping") { model.applyPrivacyRestart() }
          .disabled(model.busy)
      }
    }

    Divider()

    Text("Keyboards")
      .font(.caption)
      .foregroundStyle(.secondary)

    if model.keyboards.isEmpty {
      Button(model.busy ? "Loading keyboards…" : "Refresh Keyboards…") {
        model.refreshKeyboards()
      }
      .disabled(model.busy)
    } else {
      ForEach(model.keyboards) { kb in
        Toggle(isOn: Binding(
          get: { kb.enabled },
          set: { model.setKeyboardEnabled(kb, enabled: $0) }
        )) {
          Text(MenuModel.shortLabel(for: kb.name))
        }
        .disabled(model.busy)
      }
      Button("Refresh Keyboards…") { model.refreshKeyboards() }
        .disabled(model.busy)
    }

    Divider()

    Toggle(
      "Launch Menu at Login",
      isOn: Binding(
        get: { model.launchAtLogin },
        set: { model.setLaunchAtLogin($0) }
      )
    )

    Divider()

    Button("Open Config") { model.openConfig() }
    Button("Open Log") { model.openLog() }

    if let err = model.lastError, !err.isEmpty {
      Divider()
      Text(err)
        .foregroundStyle(.secondary)
        .font(.caption)
        .lineLimit(6)
    }

    Divider()

    Button("Quit Keys Layer Menu") { model.quit() }
  }
}
