#!/usr/bin/env bash
# Emergency: stop keys-layer and give the keyboard back to macOS.
#
# Use when the keyboard is dead (mouse still works — open Terminal and run this).
#
#   ./scripts/keys-layer-emergency-stop.sh
#   # or after brew install (if linked):
#   keys-layer-emergency-stop
#
# Remaps stay OFF until you start again:
#   sudo launchctl bootstrap system /Library/LaunchDaemons/local.keys-layer.plist
#   sudo launchctl kickstart -k system/local.keys-layer
#   # or: keys-layer-setup

set -euo pipefail

echo "==> emergency stop: releasing keyboard from keys-layer"

sudo launchctl bootout system/local.keys-layer 2>/dev/null || true
sudo pkill -9 keys-layer 2>/dev/null || true

# Extra: if a foreground sudo keys-layer is running
pkill -9 -f '/keys-layer' 2>/dev/null || true

sleep 0.3
if pgrep -x keys-layer >/dev/null 2>&1; then
  echo "warning: keys-layer still running:" >&2
  pgrep -lf keys-layer || true
  echo "try: sudo kill -9 \$(pgrep -x keys-layer)" >&2
  exit 1
fi

echo "==> stopped. Keyboard should work via macOS now."
echo "    Remaps are OFF. To start again later:"
echo "      sudo launchctl bootstrap system /Library/LaunchDaemons/local.keys-layer.plist"
echo "      sudo launchctl kickstart -k system/local.keys-layer"
