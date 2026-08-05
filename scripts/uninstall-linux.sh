#!/usr/bin/env bash
# Uninstall Linux keys-layer install (binary, systemd, udev). Keeps config by default.
#
#   ./scripts/uninstall-linux.sh
#   ./scripts/uninstall-linux.sh --purge-config

set -euo pipefail

PURGE_CONFIG=0
for arg in "$@"; do
  case "$arg" in
    --purge-config) PURGE_CONFIG=1 ;;
    -h|--help)
      sed -n '2,6p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *)
      echo "unknown option: $arg" >&2
      exit 1
      ;;
  esac
done

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "error: Linux-only" >&2
  exit 1
fi

REAL_USER="${SUDO_USER:-${USER}}"
REAL_HOME="$(getent passwd "$REAL_USER" | cut -d: -f6)"
BIN_PATH="${REAL_HOME}/.local/bin/keys-layer"
CONFIG_PATH="${REAL_HOME}/.config/keys-layer/config.toml"
USER_UNIT="${REAL_HOME}/.config/systemd/user/keys-layer.service"

echo "==> stopping services (if any)"
systemctl --user disable --now keys-layer 2>/dev/null || true
sudo systemctl disable --now keys-layer 2>/dev/null || true
sudo rm -f /etc/systemd/system/keys-layer.service
sudo systemctl daemon-reload 2>/dev/null || true
rm -f "${USER_UNIT}"
systemctl --user daemon-reload 2>/dev/null || true

echo "==> removing binary"
rm -f "${BIN_PATH}"

echo "==> removing udev rule"
sudo rm -f /etc/udev/rules.d/99-keys-layer-uinput.rules
sudo udevadm control --reload-rules 2>/dev/null || true

if [[ "${PURGE_CONFIG}" -eq 1 ]]; then
  echo "==> removing config"
  rm -f "${CONFIG_PATH}"
  rmdir "${REAL_HOME}/.config/keys-layer" 2>/dev/null || true
else
  echo "==> keeping config at ${CONFIG_PATH}"
fi

echo "==> done"
