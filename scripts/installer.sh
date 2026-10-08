#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
profile="${1:-release}"
bash scripts/bundle.sh "$profile"
staging="$(mktemp -d /tmp/sparkpad-installer.XXXXXX)"
trap 'rm -rf "$staging"' EXIT
cp -R dist/Sparkpad.app "$staging/"
ln -s /Applications "$staging/Applications"
cat > "$staging/Install.txt" <<'TXT'
Drag Sparkpad into Applications, then open it.
Choose local mode or connect to your own Sparkpad server.
This community build is ad-hoc signed, without Apple notarization.
If macOS blocks it, use System Settings > Privacy & Security > Open Anyway.
TXT
arch="$(uname -m)"
file="dist/Sparkpad-0.2.0-macos-$arch.dmg"
if [[ -e "$file" ]]; then rm "$file"; fi
hdiutil create -volname Sparkpad -srcfolder "$staging" -format UDZO "$file"
printf 'Installer: %s\n' "$PWD/$file"
