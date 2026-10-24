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
  @Published var lastError: String?

  private var timer: Timer?

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
    let target = enabled
    Task.detached {
      do {
        try DaemonController.setEnabled(target)
        await MainActor.run {
          self.isOn = DaemonController.statusOn()
          self.busy = false
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
      lastError = "No config at \(cfg.path) — run keys-layer-setup first."
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
        if model.isOn { Text("✓") }
      }
    }
    .disabled(model.busy || model.isOn)

    Button {
      model.setEnabled(false)
    } label: {
      HStack {
        Text("Turn Remapping Off")
        if !model.isOn { Text("✓") }
      }
    }
    .disabled(model.busy || !model.isOn)

    Divider()

    Toggle("Launch Menu at Login", isOn: Binding(
      get: { model.launchAtLogin },
      set: { model.setLaunchAtLogin($0) }
    ))

    Divider()

    Button("Open Config") { model.openConfig() }
    Button("Open Log") { model.openLog() }

    if let err = model.lastError, !err.isEmpty {
      Divider()
      Text(err)
        .foregroundStyle(.secondary)
        .font(.caption)
        .lineLimit(3)
    }

    Divider()

    Button("Quit Keys Layer Menu") { model.quit() }
  }
}
