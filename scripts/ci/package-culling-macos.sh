#!/usr/bin/env bash
set -euo pipefail
app="staging/SimpleImageViewer.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$app/Contents/Frameworks"
cp target/aarch64-apple-darwin/ci/SimpleImageViewer "$app/Contents/MacOS/"
cp README.md README_CN.md CULLING_CN.md LICENSE staging/
python3 - "$app" <<'PY'
import plistlib, pathlib, sys
root = pathlib.Path(sys.argv[1])
with (root / 'Contents/Info.plist').open('wb') as f:
    plistlib.dump(dict(CFBundleName='SimpleImageViewer', CFBundleDisplayName='SimpleImageViewer',
        CFBundleIdentifier='io.github.iltg119823.simpleimageviewer', CFBundleVersion='3.1.6',
        CFBundleShortVersionString='3.1.6', CFBundleExecutable='SimpleImageViewer',
        CFBundlePackageType='APPL', LSMinimumSystemVersion='11.0', NSHighResolutionCapable=True), f)
PY
# Bundle the optional dynamic OpenMP fallback; static builds need no copy.
binary="$app/Contents/MacOS/SimpleImageViewer"
while IFS= read -r dependency; do
  if [[ "$dependency" == *libomp.dylib ]]; then
    cp "$(brew --prefix libomp)/lib/libomp.dylib" "$app/Contents/Frameworks/libomp.dylib"
    chmod u+w "$binary" "$app/Contents/Frameworks/libomp.dylib"
    install_name_tool -change "$dependency" '@executable_path/../Frameworks/libomp.dylib' "$binary"
    install_name_tool -id '@executable_path/../Frameworks/libomp.dylib' "$app/Contents/Frameworks/libomp.dylib"
    codesign --force --sign - "$app/Contents/Frameworks/libomp.dylib"
  fi
done < <(otool -L "$binary" | tail -n +2 | awk '{print $1}')
# Fail instead of shipping a package that depends on the builder's Homebrew paths.
for macho in "$binary" "$app"/Contents/Frameworks/*.dylib; do
  [[ -f "$macho" ]] || continue
  if otool -L "$macho" | tail -n +2 | grep -E '/opt/homebrew|/usr/local/|/Users/'; then
    echo 'Unbundled non-system dynamic dependency' >&2
    exit 1
  fi
done
codesign --force --sign - "$app"
codesign --verify --deep --strict "$app"
lipo -verify_arch arm64 "$binary"
ditto -c -k --sequesterRsrc --keepParent "$app" SimpleImageViewer-aarch64-apple-darwin.zip
zip -j SimpleImageViewer-aarch64-apple-darwin.zip README.md README_CN.md CULLING_CN.md LICENSE
