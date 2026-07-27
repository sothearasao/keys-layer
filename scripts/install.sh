#!/usr/bin/env bash
# Install keys-layer: build binary, config, optional LaunchDaemon.
#
# Usage (from repo root or any cwd):
#   ./scripts/install.sh              # binary + config + autostart
#   ./scripts/install.sh --no-daemon  # binary + config only
#
# Prerequisites (still manual — Apple requires them):
#   - Karabiner VirtualHIDDevice driver + daemon
#   - Accessibility + Input Monitoring for the installed binary
#   - Karabiner-Core-Service disabled while using keys-layer
#
# See prerequisite.md

set -euo pipefail

NO_DAEMON=0
for arg in "$@"; do
  case "$arg" in
    --no-daemon) NO_DAEMON=1 ;;
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
  echo "error: keys-layer install is macOS-only" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Real user when invoked via sudo (don't install into /var/root).
REAL_USER="${SUDO_USER:-${USER}}"
if [[ "$REAL_USER" == "root" ]]; then
  echo "error: run as your login user (use sudo only when the script asks), e.g.:" >&2
  echo "  ./scripts/install.sh" >&2
  exit 1
fi
REAL_HOME="$(dscl . -read "/Users/${REAL_USER}" NFSHomeDirectory 2>/dev/null | awk '{print $2}')"
if [[ -z "${REAL_HOME}" || ! -d "${REAL_HOME}" ]]; then
  REAL_HOME="$(eval echo "~${REAL_USER}")"
fi

BIN_DIR="/usr/local/bin"
BIN_PATH="${BIN_DIR}/keys-layer"
CONFIG_DIR="${REAL_HOME}/.config/keys-layer"
CONFIG_PATH="${CONFIG_DIR}/config.toml"
PLIST_LABEL="local.keys-layer"
PLIST_DEST="/Library/LaunchDaemons/${PLIST_LABEL}.plist"
PLIST_TEMPLATE="${ROOT}/packaging/local.keys-layer.plist.in"
EXAMPLE_CONFIG="${ROOT}/config.example.toml"

echo "==> keys-layer install"
echo "    user:   ${REAL_USER}"
echo "    home:   ${REAL_HOME}"
echo "    binary: ${BIN_PATH}"
echo "    config: ${CONFIG_PATH}"
echo

# --- build ---
if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found. Install Rust from https://rustup.rs" >&2
  exit 1
fi

echo "==> building release binary"
# Build as the login user so cargo cache stays in their home.
if [[ "$(id -un)" == "root" ]]; then
  sudo -u "${REAL_USER}" -H bash -lc "cd $(printf %q "$ROOT") && cargo build --release -p keys-layer"
else
  cargo build --release -p keys-layer
fi

BUILT="${ROOT}/target/release/keys-layer"
if [[ ! -x "${BUILT}" ]]; then
  echo "error: build succeeded but ${BUILT} is missing" >&2
  exit 1
fi

echo "==> installing binary to ${BIN_PATH}"
sudo mkdir -p "${BIN_DIR}"
sudo cp "${BUILT}" "${BIN_PATH}"
sudo chown root:wheel "${BIN_PATH}"
sudo chmod 755 "${BIN_PATH}"

# --- config ---
echo "==> config"
mkdir -p "${CONFIG_DIR}"
if [[ -f "${CONFIG_PATH}" ]]; then
  echo "    keeping existing ${CONFIG_PATH}"
else
  if [[ ! -f "${EXAMPLE_CONFIG}" ]]; then
    echo "error: missing ${EXAMPLE_CONFIG}" >&2
    exit 1
  fi
  cp "${EXAMPLE_CONFIG}" "${CONFIG_PATH}"
  chown "${REAL_USER}" "${CONFIG_PATH}" 2>/dev/null || true
  echo "    created ${CONFIG_PATH} (from config.example.toml)"
fi

# --- LaunchDaemon ---
if [[ "${NO_DAEMON}" -eq 0 ]]; then
  if [[ ! -f "${PLIST_TEMPLATE}" ]]; then
    echo "error: missing ${PLIST_TEMPLATE}" >&2
    exit 1
  fi

  echo "==> installing LaunchDaemon (${PLIST_LABEL})"
  TMP_PLIST="$(mktemp)"
  sed \
    -e "s|__KEYS_LAYER_BIN__|${BIN_PATH}|g" \
    -e "s|__KEYS_LAYER_CONFIG__|${CONFIG_PATH}|g" \
    "${PLIST_TEMPLATE}" > "${TMP_PLIST}"

  if ! plutil -lint "${TMP_PLIST}" >/dev/null; then
    echo "error: generated plist failed plutil -lint" >&2
    cat "${TMP_PLIST}" >&2
    rm -f "${TMP_PLIST}"
    exit 1
  fi

  sudo cp "${TMP_PLIST}" "${PLIST_DEST}"
  rm -f "${TMP_PLIST}"
  sudo chown root:wheel "${PLIST_DEST}"
  sudo chmod 644 "${PLIST_DEST}"

  sudo launchctl bootout "system/${PLIST_LABEL}" 2>/dev/null || true
  sudo launchctl bootstrap system "${PLIST_DEST}"
  sudo launchctl kickstart -k "system/${PLIST_LABEL}"
  echo "    daemon started (log: /var/log/keys-layer.log)"
else
  echo "==> skipping LaunchDaemon (--no-daemon)"
fi

# --- optional: disable KE Core-Service (keep VirtualHID daemon) ---
echo "==> disabling Karabiner-Core-Service if present (conflicts with keys-layer)"
sudo launchctl bootout system/org.pqrs.service.daemon.Karabiner-Core-Service 2>/dev/null || true
sudo launchctl disable system/org.pqrs.service.daemon.Karabiner-Core-Service 2>/dev/null || true
sudo killall Karabiner-Core-Service 2>/dev/null || true
if pgrep -x Karabiner-Core-Service >/dev/null 2>&1; then
  echo "    warning: Karabiner-Core-Service still running — quit Karabiner-Elements"
else
  echo "    Core-Service not running (good)"
fi

echo
echo "==> done"
echo
echo "Still required (macOS will not automate these):"
echo "  1. VirtualHID driver enabled:"
echo "       System Settings → General → Login Items & Extensions → Driver Extensions"
echo "  2. Privacy for THIS binary:"
echo "       ${BIN_PATH}"
echo "       → Accessibility  AND  Input Monitoring"
echo "       (remove old ~/.cargo/bin/keys-layer entries if present)"
echo "  3. Keep Karabiner-Elements remapping quit; VirtualHID daemon must stay running."
echo
if [[ "${NO_DAEMON}" -eq 0 ]]; then
  echo "Verify:"
  echo "  tail -20 /var/log/keys-layer.log"
  echo "  sudo launchctl print system/${PLIST_LABEL} | head -20"
  echo
  echo "Config edits hot-reload automatically (see /var/log/keys-layer.log)."
  echo "After rebuilding the binary:"
  echo "  sudo launchctl kickstart -k system/${PLIST_LABEL}"
else
  echo "Run in foreground:"
  echo "  sudo ${BIN_PATH} ${CONFIG_PATH}"
fi
echo
echo "Uninstall:  ./scripts/uninstall.sh"
echo "Docs:       prerequisite.md · configuration.md"
