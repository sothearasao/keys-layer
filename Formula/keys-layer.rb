# Homebrew formula for keys-layer (macOS only).
#
#   brew tap sothearasao/keys-layer https://github.com/sothearasao/keys-layer.git
#   brew trust sothearasao/keys-layer
#   brew install sothearasao/keys-layer/keys-layer
#   keys-layer-setup
#
# Latest from main:
#   brew install --HEAD sothearasao/keys-layer/keys-layer

class KeysLayer < Formula
  desc "Hold-to-layer keyboard remapper for macOS (Karabiner VirtualHID)"
  homepage "https://github.com/sothearasao/keys-layer"
  url "https://github.com/sothearasao/keys-layer/archive/refs/tags/v0.1.3.tar.gz"
  sha256 "7289ed4fd2125e7ee5efb996d518a84bb0559a5b4e925b821922fdd036a8ba1c"
  license "MIT"
  head "https://github.com/sothearasao/keys-layer.git", branch: "main"

  depends_on :macos
  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/keys-layer")

    (pkgshare/"examples").install "config.example.toml"
    pkgshare.install "packaging/local.keys-layer.plist.in"
    pkgshare.install "scripts/keys-layer-setup"

    # Wrapper so `keys-layer-setup` finds share files next to itself via env.
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
  end

  def caveats
    <<~EOS
      Permanent requirements (macOS):
        • Karabiner VirtualHIDDevice driver + daemon
        • Accessibility + Input Monitoring for:
            #{opt_bin}/keys-layer
        • Do not run Karabiner-Elements Core-Service alongside keys-layer

      Finish setup (config + LaunchDaemon):
        keys-layer-setup

      Then grant Accessibility + Input Monitoring to the binary above,
      and restart (TCC applies only to a new process):
        sudo launchctl kickstart -k system/local.keys-layer

      Or run in the foreground:
        sudo #{opt_bin}/keys-layer

      Docs: #{homepage}
    EOS
  end

  test do
    assert_path_exists bin/"keys-layer"
    assert_predicate bin/"keys-layer-setup", :executable?
  end
end
