# Homebrew formula for keys-layer (macOS only).
#
#   brew tap sothearasao/keys-layer https://github.com/sothearasao/keys-layer.git
#   brew trust sothearasao/keys-layer
#   brew install --HEAD sothearasao/keys-layer/keys-layer
#   keys-layer-setup
#
# After you publish a GitHub release tag, add stable `url` + `sha256` and users
# can `brew install sothearasao/keys-layer/keys-layer` without --HEAD.

class KeysLayer < Formula
  desc "Hold-to-layer keyboard remapper for macOS (Karabiner VirtualHID)"
  homepage "https://github.com/sothearasao/keys-layer"
  license "MIT"
  head "https://github.com/sothearasao/keys-layer.git", branch: "main"

  # Stable (uncomment after tagging v0.1.0 and filling sha256):
  # url "https://github.com/sothearasao/keys-layer/archive/refs/tags/v0.1.0.tar.gz"
  # sha256 "REPLACE_ME"
  # version "0.1.0"

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
