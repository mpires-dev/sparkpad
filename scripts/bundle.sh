#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
profile="${1:-debug}"
if [[ ! -f assets/app/Sparkpad.icns || assets/app/icon.png -nt assets/app/Sparkpad.icns || scripts/generate-icons.sh -nt assets/app/Sparkpad.icns ]]; then
    bash scripts/generate-icons.sh
fi
if [[ "$profile" == release ]]; then
    cargo build --locked --release
elif [[ "$profile" == debug ]]; then
    cargo build --locked
else
    echo 'Usage: scripts/bundle.sh [debug|release]' >&2
    exit 1
fi
app="$PWD/dist/Sparkpad.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "target/$profile/sparkpad" "$app/Contents/MacOS/sparkpad"
cp vendor/empire-ui/assets/iconoir/LICENSE "$app/Contents/Resources/Iconoir-LICENSE"
cp assets/emoji/Unicode-LICENSE.txt "$app/Contents/Resources/Unicode-Emoji-LICENSE.txt"
cp assets/emoji/Emojilib-LICENSE.txt "$app/Contents/Resources/Emojilib-LICENSE.txt"
cp assets/fonts/Libron-OFL.txt "$app/Contents/Resources/Libron-OFL.txt"
cp assets/fonts/JetBrains-Mono-OFL.txt "$app/Contents/Resources/JetBrains-Mono-OFL.txt"
cp assets/fonts/OFL.txt "$app/Contents/Resources/NV-Legible-Next-OFL.txt"
cp assets/app/Sparkpad.icns "$app/Contents/Resources/Sparkpad.icns"
cp LICENSE "$app/Contents/Resources/Sparkpad-LICENSE"
cat > "$app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Sparkpad</string>
<key>CFBundleDisplayName</key><string>Sparkpad</string>
<key>CFBundleIdentifier</key><string>dev.mpires.sparkpad</string>
<key>CFBundleVersion</key><string>2</string>
<key>CFBundleShortVersionString</key><string>0.2.0</string>
<key>CFBundleExecutable</key><string>sparkpad</string>
<key>CFBundleIconFile</key><string>Sparkpad.icns</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>LSUIElement</key><false/>
<key>NSHighResolutionCapable</key><true/>
<key>LSMinimumSystemVersion</key><string>12.0</string>
<key>NSPrincipalClass</key><string>NSApplication</string>
</dict></plist>
PLIST
codesign --force --sign - "$app"
printf 'Created %s\n' "$app"
