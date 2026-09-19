#!/usr/bin/env bash
# Install/activate Karabiner VirtualHIDDevice for keys-layer (macOS).
#
# Usage:
#   ./scripts/setup-virtualhid.sh           # pkg if needed, forceActivate, start daemon
#   ./scripts/setup-virtualhid.sh --no-pkg  # only activate + start (already installed)
#
# Pins VirtualHIDDevice 6.x to match karabiner-driverkit 0.3.x (Karabiner-Elements ~6.x).
# Do not install standalone v8.0.0 unless you also bump the crate to 0.4.x.
#
# Still manual (Apple): System Settings → Driver Extensions → enable the pqrs dext.

set -euo pipefail

NO_PKG=0
for arg in "$@"; do
  case "$arg" in
    --no-pkg) NO_PKG=1 ;;
    -h|--help)
      sed -n '2,14p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *)
      echo "unknown option: $arg (try --help)" >&2
      exit 1
      ;;
  esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "error: macOS only" >&2
  exit 1
fi

REAL_USER="${SUDO_USER:-${USER}}"
if [[ "$REAL_USER" == "root" ]]; then
  echo "error: run as your login user (sudo is prompted when needed):" >&2
  echo "  ./scripts/setup-virtualhid.sh" >&2
  exit 1
fi

# 6.x matches crate 0.3.x. Override only if you know you need another 6.x pkg.
PKG_VERSION="${KEYS_LAYER_VHID_PKG_VERSION:-6.10.0}"
PKG_URL="https://github.com/pqrs-org/Karabiner-DriverKit-VirtualHIDDevice/releases/download/v${PKG_VERSION}/Karabiner-DriverKit-VirtualHIDDevice-${PKG_VERSION}.pkg"

MANAGER="/Applications/.Karabiner-VirtualHIDDevice-Manager.app/Contents/MacOS/Karabiner-VirtualHIDDevice-Manager"
DAEMON="/Library/Application Support/org.pqrs/Karabiner-DriverKit-VirtualHIDDevice/Applications/Karabiner-VirtualHIDDevice-Daemon.app/Contents/MacOS/Karabiner-VirtualHIDDevice-Daemon"
DAEMON_LABEL="org.pqrs.Karabiner-VirtualHIDDevice-Daemon"

echo "==> keys-layer VirtualHID setup"
echo "    user: ${REAL_USER}"
echo

install_pkg() {
  local tmp
  tmp="$(mktemp -t keys-layer-vhid.XXXXXX).pkg"
  echo "==> downloading VirtualHIDDevice ${PKG_VERSION}"
  echo "    ${PKG_URL}"
  if ! curl -fL --retry 3 --retry-delay 1 -o "${tmp}" "${PKG_URL}"; then
    rm -f "${tmp}"
    echo "error: download failed. Get a 6.x pkg from:" >&2
    echo "  https://github.com/pqrs-org/Karabiner-DriverKit-VirtualHIDDevice/releases" >&2
    exit 1
  fi
  echo "==> installing pkg (password prompt)"
  sudo installer -pkg "${tmp}" -target /
  rm -f "${tmp}"
}

if [[ ! -x "${MANAGER}" ]]; then
  if [[ "${NO_PKG}" -eq 1 ]]; then
    echo "error: manager not found:" >&2
    echo "  ${MANAGER}" >&2
    echo "Install the 6.x pkg, or re-run without --no-pkg." >&2
    exit 1
  fi
  install_pkg
  if [[ ! -x "${MANAGER}" ]]; then
    echo "error: pkg installed but manager still missing:" >&2
    echo "  ${MANAGER}" >&2
    exit 1
  fi
else
  echo "==> manager already installed"
fi

echo "==> forceActivate"
if sudo "${MANAGER}" forceActivate; then
  echo "    ok"
else
  echo "    forceActivate failed — enable the Driver Extension, then re-run this script" >&2
fi

start_daemon() {
  local plist
  for plist in \
    "/Library/LaunchDaemons/${DAEMON_LABEL}.plist" \
    "/Library/Application Support/org.pqrs/Karabiner-DriverKit-VirtualHIDDevice/Library/LaunchDaemons/${DAEMON_LABEL}.plist"
  do
    if [[ -f "${plist}" ]]; then
      echo "    using ${plist}"
      sudo launchctl bootout "system/${DAEMON_LABEL}" 2>/dev/null || true
      sudo launchctl bootstrap system "${plist}" 2>/dev/null || true
      sudo launchctl kickstart -k "system/${DAEMON_LABEL}" 2>/dev/null || true
      return 0
    fi
  done

  if [[ ! -x "${DAEMON}" ]]; then
    echo "    warning: daemon binary not found:" >&2
    echo "      ${DAEMON}" >&2
    return 1
  fi

  echo "    starting daemon in background"
  sudo pkill -x Karabiner-VirtualHIDDevice-Daemon 2>/dev/null || true
  sudo "${DAEMON}" >/tmp/karabiner-virtualhid-daemon.log 2>&1 &
  disown || true
}

if pgrep -x Karabiner-VirtualHIDDevice-Daemon >/dev/null 2>&1; then
  echo "==> daemon already running"
else
  echo "==> starting VirtualHID daemon"
  start_daemon || true
  sleep 0.4
fi

echo "==> disabling Karabiner-Core-Service if present (conflicts with keys-layer)"
osascript -e 'quit app "Karabiner-Elements"' 2>/dev/null || true
sudo launchctl bootout system/org.pqrs.service.daemon.Karabiner-Core-Service 2>/dev/null || true
sudo launchctl disable system/org.pqrs.service.daemon.Karabiner-Core-Service 2>/dev/null || true
sudo killall Karabiner-Core-Service 2>/dev/null || true

echo "==> opening Driver Extensions (enable org.pqrs.Karabiner-DriverKit-VirtualHIDDevice)"
open "x-apple.systempreferences:com.apple.LoginItems-Settings.extension" 2>/dev/null || true

echo
echo "==> status"
if [[ -x "${MANAGER}" ]]; then
  echo "    manager: present"
else
  echo "    manager: MISSING"
fi
if pgrep -x Karabiner-VirtualHIDDevice-Daemon >/dev/null 2>&1; then
  echo "    daemon:  running"
else
  echo "    daemon:  NOT running"
fi
if command -v systemextensionsctl >/dev/null 2>&1; then
  systemextensionsctl list 2>/dev/null | grep -i pqrs || echo "    dext:    not listed yet (enable it in System Settings)"
fi

echo
echo "==> still required (macOS will not automate this):"
echo "    System Settings → General → Login Items & Extensions → Driver Extensions"
echo "    → enable org.pqrs.Karabiner-DriverKit-VirtualHIDDevice"
echo
echo "Then install keys-layer (if you have not):"
echo "    ./scripts/install.sh"
echo "    # or: brew install --HEAD sothearasao/keys-layer/keys-layer && keys-layer-setup"
echo
echo "Re-run this script after enabling the Driver Extension if the daemon is down."
