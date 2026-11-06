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
    if keyboards.isEmpty {
      keyboards = DaemonController.keyboardsFromConfigAndCache()
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
        // Give the remapper a moment to hot-reload disabled_devices.
        try? await Task.sleep(nanoseconds: 700_000_000)
        let items = (try? DaemonController.listKeyboards())
          ?? DaemonController.keyboardsFromConfigAndCache()
        await MainActor.run {
          self.keyboards = items
          self.didLiveScan = true
          self.isOn = DaemonController.statusOn()
          self.busy = false
          self.lastError = enabled
            ? nil
            : "\(MenuModel.shortLabel(for: name)) muted — no keys from that board."
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

  static func shortLabel(for name: String) -> String {
    if name.localizedCaseInsensitiveContains("Apple Internal") {
      return "MacBook Keyboard"
    }
    if name.count > 26 {
      return String(name.prefix(23)) + "…"
    }
    return name
  }
}

struct ContentView: View {
  @ObservedObject var model: MenuModel

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      HStack(spacing: 8) {
        Button("Turn On") { model.setEnabled(true) }
          .disabled(model.busy || (model.isOn && !model.needsPrivacyFix))
        Button("Turn Off") { model.setEnabled(false) }
          .disabled(model.busy || !model.isOn)
        Spacer(minLength: 8)
        Text(model.isOn ? "On" : "Off")
          .font(.caption.weight(.semibold))
          .foregroundStyle(model.isOn ? Color.green : Color.secondary)
      }

      if model.needsPrivacyFix || !model.isOn {
        HStack(spacing: 8) {
          Button("Privacy…") { model.openPrivacySettings() }
          if model.needsPrivacyFix {
            Button("Fix Privacy") { model.applyPrivacyRestart() }
              .disabled(model.busy)
          }
        }
      }

      Divider()

      HStack(alignment: .firstTextBaseline) {
        VStack(alignment: .leading, spacing: 2) {
          Text("Keyboards")
            .font(.headline)
          Text("Off = muted completely")
            .font(.caption2)
            .foregroundStyle(.secondary)
        }
        Spacer(minLength: 8)
        Button(model.busy ? "…" : "Scan") {
          model.refreshKeyboards()
        }
        .disabled(model.busy)
        .controlSize(.small)
      }

      if model.keyboards.isEmpty {
        Text("No keyboards yet — click Scan.")
          .font(.caption)
          .foregroundStyle(.secondary)
          .frame(maxWidth: .infinity, alignment: .leading)
      } else {
        VStack(spacing: 0) {
          ForEach(Array(model.keyboards.enumerated()), id: \.element.id) { index, kb in
            HStack(alignment: .center, spacing: 12) {
              Text(MenuModel.shortLabel(for: kb.name))
                .lineLimit(1)
                .truncationMode(.tail)
                .frame(maxWidth: .infinity, alignment: .leading)
              Toggle("", isOn: Binding(
                get: { kb.enabled },
                set: { model.setKeyboardEnabled(kb, enabled: $0) }
              ))
              .labelsHidden()
              .toggleStyle(.switch)
              .controlSize(.small)
              .disabled(model.busy || !model.isOn)
              .frame(width: 40, alignment: .trailing)
            }
            .padding(.vertical, 6)
            if index < model.keyboards.count - 1 {
              Divider()
            }
          }
        }
        .padding(.horizontal, 4)
      }

      if !model.isOn {
        Text("Turn remapping On to mute/unmute boards.")
          .font(.caption2)
          .foregroundStyle(.secondary)
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
          .lineLimit(4)
          .fixedSize(horizontal: false, vertical: true)
      }
    }
    .padding(14)
    .frame(width: 320)
    .onAppear {
      if model.keyboards.isEmpty {
        model.keyboards = DaemonController.keyboardsFromConfigAndCache()
      }
    }
  }
}
