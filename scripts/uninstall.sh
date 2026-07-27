#!/usr/bin/env bash
# Remove keys-layer binary, LaunchDaemon, and optionally config.
#
# Usage:
#   ./scripts/uninstall.sh           # daemon + binary; keep config
#   ./scripts/uninstall.sh --purge   # also delete ~/.config/keys-layer
#
# Does NOT remove Karabiner VirtualHID (shared with KE / kanata).

set -euo pipefail

PURGE=0
for arg in "$@"; do
  case "$arg" in
    --purge) PURGE=1 ;;
    -h|--help)
      sed -n '2,10p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *)
      echo "unknown option: $arg (try --help)" >&2
      exit 1
      ;;
  esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "error: macOS-only" >&2
  exit 1
fi

REAL_USER="${SUDO_USER:-${USER}}"
if [[ "$REAL_USER" == "root" ]]; then
  echo "error: run as your login user (sudo will be used when needed):" >&2
  echo "  ./scripts/uninstall.sh" >&2
  exit 1
fi
REAL_HOME="$(dscl . -read "/Users/${REAL_USER}" NFSHomeDirectory 2>/dev/null | awk '{print $2}')"
if [[ -z "${REAL_HOME}" || ! -d "${REAL_HOME}" ]]; then
  REAL_HOME="$(eval echo "~${REAL_USER}")"
fi

BIN_PATH="/usr/local/bin/keys-layer"
CARGO_BIN="${REAL_HOME}/.cargo/bin/keys-layer"
CONFIG_DIR="${REAL_HOME}/.config/keys-layer"
PLIST_LABEL="local.keys-layer"
PLIST_DEST="/Library/LaunchDaemons/${PLIST_LABEL}.plist"

echo "==> keys-layer uninstall"
echo

echo "==> stopping LaunchDaemon"
sudo launchctl bootout "system/${PLIST_LABEL}" 2>/dev/null || true
if [[ -f "${PLIST_DEST}" ]]; then
  sudo rm -f "${PLIST_DEST}"
  echo "    removed ${PLIST_DEST}"
else
  echo "    no plist at ${PLIST_DEST}"
fi

echo "==> removing binary"
for path in "${BIN_PATH}" "${CARGO_BIN}"; do
  if [[ -e "${path}" ]]; then
    sudo rm -f "${path}" 2>/dev/null || rm -f "${path}"
    echo "    removed ${path}"
  fi
done

if [[ "${PURGE}" -eq 1 ]]; then
  if [[ -d "${CONFIG_DIR}" ]]; then
    rm -rf "${CONFIG_DIR}"
    echo "==> removed config ${CONFIG_DIR}"
  else
    echo "==> no config dir to purge"
  fi
else
  echo "==> keeping config ${CONFIG_DIR} (use --purge to delete)"
fi

echo
echo "==> done"
echo
echo "Optional cleanup:"
echo "  • System Settings → Privacy & Security → Accessibility / Input Monitoring"
echo "    → remove keys-layer entries"
echo "  • Karabiner VirtualHID left installed (shared). To remove it, see uninstall.md"
echo
echo "Reinstall:  ./scripts/install.sh"
