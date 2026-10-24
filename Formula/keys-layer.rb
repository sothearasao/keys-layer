# Homebrew formula for keys-layer (macOS only).
#
#   brew tap sothearasao/keys-layer https://github.com/sothearasao/keys-layer.git
#   brew trust sothearasao/keys-layer
#   brew install --HEAD sothearasao/keys-layer/keys-layer
#
# After install: grant Privacy, open Keys Layer from /Applications, Turn On.

class KeysLayer < Formula
  desc "Hold-to-layer keyboard remapper for macOS (Karabiner VirtualHID)"
  homepage "https://github.com/sothearasao/keys-layer"
  # install() copies scripts/keys-layer-emergency-stop.sh from this archive.
  # Keep url on a snapshot that actually contains that file (v0.1.3 / v0.1.4 do not).
  url "https://github.com/sothearasao/keys-layer/archive/499a4a599e16fd6675f40c56b132ad48c570ed58.tar.gz"
  sha256 "3b66adc95693a262bbbcc7258fdb930df84bf70d5e8090e4d0818ea5f16d12dc"
  version "0.1.6"
  license "MIT"
  head "https://github.com/sothearasao/keys-layer.git", branch: "main"

  depends_on :macos
  depends_on "rust" => :build
  # Menu bar builds with Swift from Xcode *or* Command Line Tools — do not
  # depends_on xcode (that forces full Xcode.app and breaks CLT-only Macs).

  def install
    # Homebrew's rust bottle ships `rust-objcopy` as a symlink to `llvm-objcopy`.
    # On current llvm bottles that tool is often missing / the symlink breaks after
    # relocation, and `cargo install --release` then fails with:
    #   unable to run `rust-objcopy`: No such file or directory
    # Disable release stripping so the build does not need rust-objcopy.
    ENV["CARGO_PROFILE_RELEASE_STRIP"] = "none"

    system "cargo", "install", *std_cargo_args(path: "crates/keys-layer")

    (pkgshare/"examples").install "config.example.toml"
    pkgshare.install "packaging/local.keys-layer.plist.in"
    pkgshare.install "scripts/keys-layer-setup"
    pkgshare.install "scripts/keys-layer-emergency-stop.sh"
    pkgshare.install "scripts/keys-layer-ctl"

    # Wrappers so helpers find share files / binary via env (stable across upgrades).
    (bin/"keys-layer-setup").write <<~SH
      #!/bin/bash
      set -euo pipefail
      export KEYS_LAYER_BIN="#{opt_bin}/keys-layer"
      export KEYS_LAYER_EXAMPLE="#{pkgshare}/examples/config.example.toml"
      export KEYS_LAYER_PLIST_IN="#{pkgshare}/local.keys-layer.plist.in"
      exec "#{pkgshare}/keys-layer-setup" "$@"
    SH
    chmod 0755, bin/"keys-layer-setup"
    chmod 0755, pkgshare/"keys-layer-setup"

    (bin/"keys-layer-ctl").write <<~SH
      #!/bin/bash
      set -euo pipefail
      export KEYS_LAYER_BIN="#{opt_bin}/keys-layer"
      export KEYS_LAYER_EXAMPLE="#{pkgshare}/examples/config.example.toml"
      export KEYS_LAYER_PLIST_IN="#{pkgshare}/local.keys-layer.plist.in"
      exec "#{pkgshare}/keys-layer-ctl" "$@"
    SH
    chmod 0755, bin/"keys-layer-ctl"
    chmod 0755, pkgshare/"keys-layer-ctl"

    (bin/"keys-layer-emergency-stop").write <<~SH
      #!/bin/bash
      set -euo pipefail
      exec "#{pkgshare}/keys-layer-emergency-stop.sh" "$@"
    SH
    chmod 0755, bin/"keys-layer-emergency-stop"
    chmod 0755, pkgshare/"keys-layer-emergency-stop.sh"

    # Menu bar toggle — Swift from CLT is enough; skip if unavailable.
    if which("swift")
      system "./scripts/build-menubar-app.sh"
      prefix.install "dist/Keys Layer.app"
      (bin/"keys-layer-menu").write <<~SH
        #!/bin/bash
        set -euo pipefail
        export KEYS_LAYER_CTL="#{opt_bin}/keys-layer-ctl"
        open "/Applications/Keys Layer.app" 2>/dev/null || open "#{opt_prefix}/Keys Layer.app"
      SH
      chmod 0755, bin/"keys-layer-menu"
    else
      opoo "swift not found; skipping Keys Layer.app (install Xcode CLT to build it)"
    end
  end

  def post_install
    app_src = prefix/"Keys Layer.app"
    return unless app_src.directory?

    apps = Pathname.new("/Applications")
    dest = apps/"Keys Layer.app"
    # Install like a normal Mac app for Spotlight / Launchpad.
    system "rm", "-rf", dest.to_s if dest.exist?
    system "cp", "-R", app_src.to_s, dest.to_s
    ohai "Installed Keys Layer.app → #{dest}"
  end

  def caveats
    <<~EOS
      Permanent requirements (macOS):
        • Karabiner VirtualHIDDevice driver + daemon
          (./scripts/setup-virtualhid.sh or Karabiner-Elements)
        • Accessibility + Input Monitoring for:
            #{opt_bin}/keys-layer
        • Do not run Karabiner-Elements Core-Service alongside keys-layer

      Simple use:
        1. Grant Accessibility + Input Monitoring to the binary above
        2. Open "Keys Layer" from /Applications (or: keys-layer-menu)
        3. Menu bar → Turn Remapping On  (admin password once;
           creates config + LaunchDaemon on first On)
        Remapping does NOT start at login — only when you turn it on.

      Emergency stop: keys-layer-emergency-stop
      Uninstall app copy: rm -rf "/Applications/Keys Layer.app"
      Docs: #{homepage}
    EOS
  end

  test do
    assert_path_exists bin/"keys-layer"
    assert_predicate bin/"keys-layer-setup", :executable?
    assert_predicate bin/"keys-layer-emergency-stop", :executable?
    assert_predicate bin/"keys-layer-ctl", :executable?
    assert_path_exists prefix/"Keys Layer.app" if (prefix/"Keys Layer.app").exist?
  end
end
