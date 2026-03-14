#!/usr/bin/env bash
# tools/ios/build.sh
# Build libhachimi.dylib for iOS (aarch64-apple-ios)
# Must be run on macOS with Xcode installed.
set -e

if [[ "$OSTYPE" != darwin* ]]; then
    echo "Error: iOS builds must be performed on macOS."
    echo "Use GitHub Actions (see .github/workflows/build_ios.yml) to build from Windows."
    exit 1
fi

if ! command -v xcrun &>/dev/null; then
    echo "Error: Xcode command line tools not found. Install with: xcode-select --install"
    exit 1
fi

# ── Target & profile ──────────────────────────────────────────────
TARGET="aarch64-apple-ios"
IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-14.0}"

if [ "${RELEASE:-0}" = "1" ]; then
    CARGO_PROFILE="--release"
    BUILD_TYPE="release"
else
    CARGO_PROFILE=""
    BUILD_TYPE="debug"
fi

# ── Ensure target is installed ────────────────────────────────────
rustup target add "$TARGET" 2>/dev/null || true

# ── SDK paths via xcrun ───────────────────────────────────────────
SDK_PATH=$(xcrun --sdk iphoneos --show-sdk-path)
CLANG=$(xcrun --find clang)
AR=$(xcrun --find ar)

export CC_aarch64_apple_ios="$CLANG"
export AR_aarch64_apple_ios="$AR"
export CARGO_TARGET_AARCH64_APPLE_IOS_LINKER="$CLANG"

export IPHONEOS_DEPLOYMENT_TARGET

export RUSTFLAGS="\
  -C link-arg=-isysroot -C link-arg=${SDK_PATH} \
  -C link-arg=-Wl,-install_name,@rpath/libhachimi.dylib \
  -C link-arg=-mios-version-min=${IPHONEOS_DEPLOYMENT_TARGET}"

# ── Build ──────────────────────────────────────────────────────────
mkdir -p build
echo "[iOS] Building for ${TARGET} (${BUILD_TYPE})..."
cargo build --target="$TARGET" --target-dir=build $CARGO_PROFILE

DYLIB="build/${TARGET}/${BUILD_TYPE}/libhachimi.dylib"

# ── Fake-sign with ldid ────────────────────────────────────────────
if command -v ldid &>/dev/null; then
    echo "[iOS] Fake-signing with ldid..."
    ldid -S "${DYLIB}"
else
    echo "[iOS] WARNING: ldid not found. Install with: brew install ldid"
    echo "     The dylib won't be fake-signed (needed for sideloading/LiveContainer)."
fi

# ── Copy output ────────────────────────────────────────────────────
mkdir -p build/ios
cp "${DYLIB}" build/ios/libhachimi.dylib

# ── Checksum ───────────────────────────────────────────────────────
SHA256=$(shasum -a 256 build/ios/libhachimi.dylib | awk '{print $1}')
cat <<EOF > build/ios/sha256.json
{
  "libhachimi.dylib": "${SHA256}"
}
EOF

echo ""
echo "[iOS] Build complete!"
echo "      Output : build/ios/libhachimi.dylib"
echo "      SHA256 : ${SHA256}"
echo ""
echo "Next steps:"
echo "  LiveContainer : copy build/ios/libhachimi.dylib to <LiveContainer>/Tweaks/"
echo "  IPA inject    : run tools/ios/inject_ipa.sh <input.ipa> build/ios/libhachimi.dylib"
