#!/usr/bin/env bash
# Build Keys Layer.app (menu bar toggle) into dist/ or DEST.
#
# Uses `swiftc` directly (not SwiftPM) so Homebrew's build sandbox works.
# SwiftPM nests sandbox-exec and fails with: sandbox_apply: Operation not permitted
#
#   ./scripts/build-menubar-app.sh
#   DEST=/tmp/out ./scripts/build-menubar-app.sh

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKG="${ROOT}/apps/KeysLayerMenu"
DEST="${DEST:-${ROOT}/dist}"
APP_NAME="Keys Layer.app"
APP="${DEST}/${APP_NAME}"
BUILD_DIR="${PKG}/.build-homebrew"
BIN="${BUILD_DIR}/KeysLayerMenu"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "error: macOS only" >&2
  exit 1
fi

if ! command -v swiftc >/dev/null 2>&1; then
  echo "error: swiftc not found (install Xcode Command Line Tools)" >&2
  exit 1
fi

ARCH="$(uname -m)"
# MenuBarExtra needs macOS 13+.
TARGET="${ARCH}-apple-macosx13.0"

SOURCES=(
  "${PKG}/Sources/DaemonController.swift"
  "${PKG}/Sources/KeysLayerMenuApp.swift"
)

echo "==> building KeysLayerMenu (swiftc, target ${TARGET})"
rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}"

# Avoid SPM: Homebrew sandbox + SwiftPM nested sandbox_exec fails.
swiftc -O \
  -parse-as-library \
  -target "${TARGET}" \
  -sdk "$(xcrun --show-sdk-path)" \
  "${SOURCES[@]}" \
  -o "${BIN}" \
  -framework SwiftUI \
  -framework AppKit \
  -framework ServiceManagement \
  -framework Foundation

if [[ ! -x "${BIN}" ]]; then
  echo "error: missing binary ${BIN}" >&2
  exit 1
fi

echo "==> assembling ${APP}"
rm -rf "${APP}"
mkdir -p "${APP}/Contents/MacOS" "${APP}/Contents/Resources"
cp "${BIN}" "${APP}/Contents/MacOS/KeysLayerMenu"
cp "${PKG}/Info.plist" "${APP}/Contents/Info.plist"
chmod 755 "${APP}/Contents/MacOS/KeysLayerMenu"

CTL_SRC="${KEYS_LAYER_CTL_SRC:-${ROOT}/scripts/keys-layer-ctl}"
if [[ -f "${CTL_SRC}" ]]; then
  cp "${CTL_SRC}" "${APP}/Contents/MacOS/keys-layer-ctl"
  chmod 755 "${APP}/Contents/MacOS/keys-layer-ctl"
else
  echo "error: keys-layer-ctl not found at ${CTL_SRC}" >&2
  echo "    (Homebrew must build the app before pkgshare.install moves scripts/)" >&2
  exit 1
fi

if command -v codesign >/dev/null 2>&1; then
  codesign --force --deep --sign - "${APP}" 2>/dev/null || true
fi

echo "==> done: ${APP}"
echo "    open \"${APP}\""
