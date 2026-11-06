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
    // Window style so keyboard toggles are always visible (plain .menu hid them).
    .menuBarExtraStyle(.window)
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
  private var didLiveScan = false

  var remapperPath: String { DaemonController.remapperBinaryPath() }

  init() {
    keyboards = DaemonController.keyboardsFromConfigAndCache()
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
    if !didLiveScan {
      // Keep toggles visible from config while idle.
      let seeded = DaemonController.keyboardsFromConfigAndCache()
      if keyboards.isEmpty {
        keyboards = seeded
      }
    }
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
          self.keyboards = items.isEmpty
            ? DaemonController.keyboardsFromConfigAndCache()
            : items
          self.didLiveScan = !items.isEmpty
          self.busy = false
          if items.isEmpty {
            self.lastError = "No keyboards reported. Is VirtualHID running?"
          }
        }
      } catch {
        await MainActor.run {
          self.keyboards = DaemonController.keyboardsFromConfigAndCache()
          self.lastError = error.localizedDescription
          self.busy = false
        }
      }
    }
  }

  func setKeyboardEnabled(_ item: KeyboardItem, enabled: Bool) {
    guard !busy else { return }
    // Optimistic UI so the toggle feels instant.
    if let idx = keyboards.firstIndex(where: { $0.id == item.id }) {
      var copy = keyboards
      copy[idx].enabled = enabled
      keyboards = copy
    }
    busy = true
    lastError = nil
    let name = item.name
    Task.detached {
      do {
        try DaemonController.setKeyboardEnabled(name: name, enabled: enabled)
        let items = (try? DaemonController.listKeyboards())
          ?? DaemonController.keyboardsFromConfigAndCache()
        await MainActor.run {
          self.keyboards = items
          self.didLiveScan = true
          self.isOn = DaemonController.statusOn()
          self.busy = false
        }
      } catch {
        await MainActor.run {
          self.keyboards = DaemonController.keyboardsFromConfigAndCache()
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
          listed = (try? DaemonController.listKeyboards())
            ?? DaemonController.keyboardsFromConfigAndCache()
        } else {
          listed = DaemonController.keyboardsFromConfigAndCache()
        }
        await MainActor.run {
          self.isOn = DaemonController.statusOn()
          self.keyboards = listed
          self.didLiveScan = target && !listed.isEmpty
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
        let keyboards = (try? DaemonController.listKeyboards())
          ?? DaemonController.keyboardsFromConfigAndCache()
        await MainActor.run {
          self.keyboards = keyboards
          self.didLiveScan = true
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
    if name.count > 28 {
      return String(name.prefix(25)) + "…"
    }
    return name
  }
}

struct ContentView: View {
  @ObservedObject var model: MenuModel

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      HStack(spacing: 8) {
        Button("Turn On") { model.setEnabled(true) }
          .disabled(model.busy || (model.isOn && !model.needsPrivacyFix))
        Button("Turn Off") { model.setEnabled(false) }
          .disabled(model.busy || !model.isOn)
        Spacer()
        Text(model.isOn ? "On" : "Off")
          .foregroundStyle(model.isOn ? .green : .secondary)
          .font(.caption.weight(.semibold))
      }

      if model.needsPrivacyFix || !model.isOn {
        HStack(spacing: 8) {
          Button("Privacy Settings…") { model.openPrivacySettings() }
          if model.needsPrivacyFix {
            Button("Fix Privacy") { model.applyPrivacyRestart() }
              .disabled(model.busy)
          }
        }
      }

      Divider()

      HStack {
        Text("Keyboards")
          .font(.headline)
        Spacer()
        Button(model.busy ? "Scanning…" : "Scan") {
          model.refreshKeyboards()
        }
        .disabled(model.busy)
      }

      if model.keyboards.isEmpty {
        Text("No keyboards yet — click Scan (password once).")
          .font(.caption)
          .foregroundStyle(.secondary)
      } else {
        VStack(alignment: .leading, spacing: 4) {
          ForEach(model.keyboards) { kb in
            Toggle(isOn: Binding(
              get: { kb.enabled },
              set: { model.setKeyboardEnabled(kb, enabled: $0) }
            )) {
              Text(MenuModel.shortLabel(for: kb.name))
            }
            .toggleStyle(.switch)
            .disabled(model.busy)
          }
        }
      }

      Divider()

      Toggle(
        "Launch Menu at Login",
        isOn: Binding(
          get: { model.launchAtLogin },
          set: { model.setLaunchAtLogin($0) }
        )
      )

      HStack {
        Button("Config") { model.openConfig() }
        Button("Log") { model.openLog() }
        Spacer()
        Button("Quit") { model.quit() }
      }

      if let err = model.lastError, !err.isEmpty {
        Text(err)
          .foregroundStyle(.secondary)
          .font(.caption)
          .lineLimit(5)
          .fixedSize(horizontal: false, vertical: true)
      }
    }
    .padding(12)
    .frame(width: 300)
    .onAppear {
      if model.keyboards.isEmpty {
        model.keyboards = DaemonController.keyboardsFromConfigAndCache()
      }
    }
  }
}
