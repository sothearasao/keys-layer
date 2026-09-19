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
  # install() copies scripts/keys-layer-emergency-stop.sh from this archive.
  # Keep url on a snapshot that actually contains that file (v0.1.3 / v0.1.4 do not).
  url "https://github.com/sothearasao/keys-layer/archive/6a2010640f5e6e5f4e28a7b2391563595ebd42aa.tar.gz"
  sha256 "06a2f183ba8895a91d336be08f611e0b36ff1604fe5b05fe7b2997f0cb1d3a52"
  version "0.1.5"
  license "MIT"
  head "https://github.com/sothearasao/keys-layer.git", branch: "main"

  depends_on :macos
  depends_on "rust" => :build

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

    (bin/"keys-layer-emergency-stop").write <<~SH
      #!/bin/bash
      set -euo pipefail
      exec "#{pkgshare}/keys-layer-emergency-stop.sh" "$@"
    SH
    chmod 0755, bin/"keys-layer-emergency-stop"
    chmod 0755, pkgshare/"keys-layer-emergency-stop.sh"
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

      If the keyboard ever dies (mouse still works):
        keys-layer-emergency-stop

      Or run in the foreground:
        sudo #{opt_bin}/keys-layer

      Docs: #{homepage}
    EOS
  end

  test do
    assert_path_exists bin/"keys-layer"
    assert_predicate bin/"keys-layer-setup", :executable?
    assert_predicate bin/"keys-layer-emergency-stop", :executable?
  end
end
