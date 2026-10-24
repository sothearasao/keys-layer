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
        // One automatic restart after privacy may have been granted while running.
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
        // Let the daemon log a session before we check Privacy.
        if target {
          try? await Task.sleep(nanoseconds: 1_800_000_000)
        }
        await MainActor.run {
          self.isOn = DaemonController.statusOn()
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

  /// Off → On in one admin prompt path (two privileged calls; user may get 2 dialogs
  /// unless cached). Prefer calling after Privacy is enabled.
  func applyPrivacyRestart() {
    guard !busy else { return }
    busy = true
    lastError = "Restarting remapping so Privacy applies…"
    Task.detached {
      do {
        try DaemonController.setEnabled(false)
        try DaemonController.setEnabled(true)
        try? await Task.sleep(nanoseconds: 1_500_000_000)
        await MainActor.run {
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
