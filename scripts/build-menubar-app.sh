#!/usr/bin/env bash
# Build Keys Layer.app (menu bar toggle) into dist/ or DEST.
#
#   ./scripts/build-menubar-app.sh
#   DEST=/opt/homebrew/opt/keys-layer ./scripts/build-menubar-app.sh

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKG="${ROOT}/apps/KeysLayerMenu"
DEST="${DEST:-${ROOT}/dist}"
APP_NAME="Keys Layer.app"
APP="${DEST}/${APP_NAME}"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "error: macOS only" >&2
  exit 1
fi

if ! command -v swift >/dev/null 2>&1; then
  echo "error: swift not found (install Xcode CLT)" >&2
  exit 1
fi

echo "==> building KeysLayerMenu"
cd "${PKG}"
swift build -c release --product KeysLayerMenu

BIN="$(swift build -c release --show-bin-path)/KeysLayerMenu"
if [[ ! -x "${BIN}" ]]; then
  echo "error: missing binary ${BIN}" >&2
  exit 1
fi

echo "==> assembling ${APP}"
rm -rf "${APP}"
mkdir -p "${APP}/Contents/MacOS" "${APP}/Contents/Resources"
cp "${BIN}" "${APP}/Contents/MacOS/KeysLayerMenu"
cp "${ROOT}/scripts/keys-layer-ctl" "${APP}/Contents/MacOS/keys-layer-ctl"
cp "${PKG}/Info.plist" "${APP}/Contents/Info.plist"
chmod 755 "${APP}/Contents/MacOS/KeysLayerMenu" "${APP}/Contents/MacOS/keys-layer-ctl"

# Ad-hoc sign so Gatekeeper is less noisy for local/Homebrew builds.
if command -v codesign >/dev/null 2>&1; then
  codesign --force --deep --sign - "${APP}" 2>/dev/null || true
fi

echo "==> done: ${APP}"
echo "    open \"${APP}\""
