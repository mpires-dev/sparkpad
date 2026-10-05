#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
icon_workdir=$(mktemp -d)
trap 'rm -rf "$icon_workdir"' EXIT
iconset="$icon_workdir/Sparkpad.iconset"
mkdir -p "$iconset"
canvas="$icon_workdir/dock-icon.png"
# macOS app artwork occupies about 80% of its transparent square canvas.
swift - assets/app/icon.png "$canvas" <<'SWIFT'
import AppKit
let source = NSImage(contentsOfFile: CommandLine.arguments[1])!
let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 1024, pixelsHigh: 1024,
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
let context = NSGraphicsContext(bitmapImageRep: bitmap)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = context
context.cgContext.clear(CGRect(x: 0, y: 0, width: 1024, height: 1024))
context.imageInterpolation = .high
source.draw(in: NSRect(x: 100, y: 100, width: 824, height: 824),
    from: .zero, operation: .sourceOver, fraction: 1)
NSGraphicsContext.restoreGraphicsState()
try bitmap.representation(using: .png, properties: [:])!.write(
    to: URL(fileURLWithPath: CommandLine.arguments[2]))
SWIFT
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$canvas" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    retina=$((size * 2))
    sips -z "$retina" "$retina" "$canvas" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o assets/app/Sparkpad.icns
sips -z 54 54 assets/app/icon.png --out assets/app/status-icon.png >/dev/null
