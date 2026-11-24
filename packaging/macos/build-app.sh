#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
app_dir="$project_dir/dist/VNKey.app"
icon_source="$project_dir/assets/vnkey-icon.png"
iconset_dir="$project_dir/target/VNKey.iconset"

cd "$project_dir"
cargo build --release --package vnkey

rm -rf "$app_dir" "$iconset_dir"
mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources" "$iconset_dir"
cp target/release/vnkey "$app_dir/Contents/MacOS/vnkey"
cp packaging/macos/Info.plist "$app_dir/Contents/Info.plist"

# SwiftUI's macros ship with Xcode, not with the Command Line Tools alone.
if [ -z "${DEVELOPER_DIR:-}" ] && [ -d /Applications/Xcode.app/Contents/Developer ]; then
  export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
fi
xcrun swiftc -O -swift-version 5 -parse-as-library \
  -sdk "$(xcrun --sdk macosx --show-sdk-path)" \
  -target "$(uname -m)-apple-macos13.0" \
  -module-cache-path "$project_dir/target/swift-module-cache" \
  apps/macos-settings/Sources/*.swift apps/macos-settings/Sources/Views/*.swift \
  -o "$app_dir/Contents/MacOS/vnkey-settings"

for spec in "16 icon_16x16.png" "32 icon_16x16@2x.png" \
  "32 icon_32x32.png" "64 icon_32x32@2x.png" \
  "128 icon_128x128.png" "256 icon_128x128@2x.png" \
  "256 icon_256x256.png" "512 icon_256x256@2x.png" \
  "512 icon_512x512.png" "1024 icon_512x512@2x.png"; do
  size=${spec%% *}
  name=${spec#* }
  sips -z "$size" "$size" "$icon_source" --out "$iconset_dir/$name" >/dev/null
done

iconutil -c icns "$iconset_dir" -o "$app_dir/Contents/Resources/AppIcon.icns"
# A stable identity keeps the Accessibility permission across rebuilds; ad-hoc
# signatures change with every build. See create-signing-identity.sh.
identity=${VNKEY_SIGN_IDENTITY:-$(security find-identity -p codesigning 2>/dev/null |
  awk '/"VNKey Local Signing"/ { print $2; exit }')}
codesign --force --deep --sign "${identity:--}" "$app_dir"
codesign --verify --deep --strict --verbose=2 "$app_dir"
echo "$app_dir"
