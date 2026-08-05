#!/usr/bin/env bash
# Install keys-layer on Linux: build, config, udev, optional systemd unit.
#
# Usage (from repo root):
#   ./scripts/install-linux.sh                 # binary + config + udev
#   ./scripts/install-linux.sh --user-systemd  # also enable systemd --user unit
#   ./scripts/install-linux.sh --system-systemd  # system unit (needs input group)
#   ./scripts/install-linux.sh --no-udev
#
# See linux.md

set -euo pipefail

USER_SYSTEMD=0
SYSTEM_SYSTEMD=0
NO_UDEV=0
for arg in "$@"; do
  case "$arg" in
    --user-systemd) USER_SYSTEMD=1 ;;
    --system-systemd) SYSTEM_SYSTEMD=1 ;;
    --no-udev) NO_UDEV=1 ;;
    -h|--help)
      sed -n '2,12p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *)
      echo "unknown option: $arg (try --help)" >&2
      exit 1
      ;;
  esac
done

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "error: this installer is Linux-only (use ./scripts/install.sh on macOS)" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

REAL_USER="${SUDO_USER:-${USER}}"
if [[ "$REAL_USER" == "root" ]]; then
  echo "error: run as your login user (sudo is prompted when needed)" >&2
  exit 1
fi
REAL_HOME="$(getent passwd "$REAL_USER" | cut -d: -f6)"
if [[ -z "$REAL_HOME" || ! -d "$REAL_HOME" ]]; then
  REAL_HOME="$(eval echo "~${REAL_USER}")"
fi

BIN_DIR="${REAL_HOME}/.local/bin"
BIN_PATH="${BIN_DIR}/keys-layer"
CONFIG_DIR="${REAL_HOME}/.config/keys-layer"
CONFIG_PATH="${CONFIG_DIR}/config.toml"
EXAMPLE_CONFIG="${ROOT}/config.example.toml"
UDEV_SRC="${ROOT}/packaging/99-keys-layer-uinput.rules"
UDEV_DEST="/etc/udev/rules.d/99-keys-layer-uinput.rules"
USER_UNIT_TEMPLATE="${ROOT}/packaging/keys-layer.user.service.in"
SYSTEM_UNIT_TEMPLATE="${ROOT}/packaging/keys-layer.system.service.in"

echo "==> keys-layer Linux install"
echo "    user:   ${REAL_USER}"
echo "    home:   ${REAL_HOME}"
echo "    binary: ${BIN_PATH}"
echo "    config: ${CONFIG_PATH}"
echo

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found. Install Rust from https://rustup.rs" >&2
  exit 1
fi

echo "==> building release binary"
if [[ "$(id -un)" == "root" ]]; then
  sudo -u "${REAL_USER}" -H bash -lc "cd $(printf %q "$ROOT") && cargo build --release -p keys-layer"
else
  cargo build --release -p keys-layer
fi

mkdir -p "${BIN_DIR}"
install -m 0755 "${ROOT}/target/release/keys-layer" "${BIN_PATH}"
echo "==> installed ${BIN_PATH}"

mkdir -p "${CONFIG_DIR}"
if [[ -f "${CONFIG_PATH}" ]]; then
  echo "==> keeping existing config"
else
  cp "${EXAMPLE_CONFIG}" "${CONFIG_PATH}"
  chown "${REAL_USER}:" "${CONFIG_PATH}" 2>/dev/null || true
  echo "==> created ${CONFIG_PATH}"
fi

if [[ "${NO_UDEV}" -eq 0 ]]; then
  echo "==> installing udev rule (uinput → group input)"
  sudo install -m 0644 "${UDEV_SRC}" "${UDEV_DEST}"
  if ! getent group input >/dev/null; then
    echo "warning: group 'input' does not exist; create it or edit ${UDEV_DEST}" >&2
  else
    if ! id -nG "${REAL_USER}" | tr ' ' '\n' | grep -qx input; then
      echo "==> adding ${REAL_USER} to group input (re-login required)"
      sudo usermod -aG input "${REAL_USER}"
    fi
  fi
  sudo modprobe uinput 2>/dev/null || true
  sudo udevadm control --reload-rules
  sudo udevadm trigger
fi

if [[ "${USER_SYSTEMD}" -eq 1 ]]; then
  UNIT_DIR="${REAL_HOME}/.config/systemd/user"
  UNIT_PATH="${UNIT_DIR}/keys-layer.service"
  mkdir -p "${UNIT_DIR}"
  sed \
    -e "s|__KEYS_LAYER_BIN__|${BIN_PATH}|g" \
    -e "s|__KEYS_LAYER_CONFIG__|${CONFIG_PATH}|g" \
    "${USER_UNIT_TEMPLATE}" > "${UNIT_PATH}"
  chown "${REAL_USER}:" "${UNIT_PATH}" 2>/dev/null || true
  echo "==> installed ${UNIT_PATH}"
  if [[ "$(id -un)" == "root" ]]; then
    sudo -u "${REAL_USER}" -H bash -lc "systemctl --user daemon-reload && systemctl --user enable --now keys-layer"
  else
    systemctl --user daemon-reload
    systemctl --user enable --now keys-layer
  fi
  echo "==> systemd --user keys-layer enabled"
fi

if [[ "${SYSTEM_SYSTEMD}" -eq 1 ]]; then
  TMP="$(mktemp)"
  sed \
    -e "s|__KEYS_LAYER_BIN__|${BIN_PATH}|g" \
    -e "s|__KEYS_LAYER_CONFIG__|${CONFIG_PATH}|g" \
    -e "s|__KEYS_LAYER_USER__|${REAL_USER}|g" \
    "${SYSTEM_UNIT_TEMPLATE}" > "${TMP}"
  sudo install -m 0644 "${TMP}" /etc/systemd/system/keys-layer.service
  rm -f "${TMP}"
  sudo systemctl daemon-reload
  sudo systemctl enable --now keys-layer
  echo "==> systemd system keys-layer enabled"
fi

echo
echo "==> done"
echo "If you were added to group 'input', log out and back in."
echo "Foreground test:"
echo "  ${BIN_PATH}"
echo "Docs: linux.md"
