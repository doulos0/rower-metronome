#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [ -x "$HOME/Android/jdk-17/bin/javac" ]; then
  export JAVA_HOME="$HOME/Android/jdk-17"
elif [ -z "${JAVA_HOME:-}" ] || [ ! -x "${JAVA_HOME}/bin/javac" ]; then
  echo "JDK 17+ with javac is required (expected at $HOME/Android/jdk-17 or JAVA_HOME)." >&2
  exit 1
fi
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
export PATH="$JAVA_HOME/bin:$PATH"

BUILD_TOOLS_VER="${BUILD_TOOLS_VER:-35.0.0}"
PLATFORM_VER="${PLATFORM_VER:-35}"
NDK_VER="${NDK_VER:-27.2.12479018}"

BUILD_TOOLS="$ANDROID_HOME/build-tools/$BUILD_TOOLS_VER"
ANDROID_JAR="$ANDROID_HOME/platforms/android-$PLATFORM_VER/android.jar"
NDK_HOME="$ANDROID_HOME/ndk/$NDK_VER"
TOOLCHAIN="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"

for req in "$BUILD_TOOLS/aapt2" "$BUILD_TOOLS/d8" "$BUILD_TOOLS/zipalign" "$BUILD_TOOLS/apksigner" "$ANDROID_JAR" "$TOOLCHAIN/aarch64-linux-android26-clang"; do
  if [ ! -e "$req" ]; then
    echo "Missing required Android tool/file: $req" >&2
    exit 1
  fi
done

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
WORK_DIR="$ROOT_DIR/target/android-build"
OUT_DIR="$ROOT_DIR/target/android"
FINAL_APK="$OUT_DIR/erg-metronome-${VERSION}-arm64.apk"

rm -rf "$WORK_DIR"
mkdir -p "$WORK_DIR/classes" "$WORK_DIR/dex" "$WORK_DIR/lib/arm64-v8a" "$OUT_DIR"

echo "=== 1. Compiling Rust native library (aarch64-linux-android, 16KB page-aligned) ==="
CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TOOLCHAIN/aarch64-linux-android26-clang" \
CARGO_TARGET_AARCH64_LINUX_ANDROID_AR="$TOOLCHAIN/llvm-ar" \
CC_aarch64_linux_android="$TOOLCHAIN/aarch64-linux-android26-clang" \
CXX_aarch64_linux_android="$TOOLCHAIN/aarch64-linux-android26-clang++" \
AR_aarch64_linux_android="$TOOLCHAIN/llvm-ar" \
RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384" \
  cargo build --release --lib --target aarch64-linux-android

cp "target/aarch64-linux-android/release/librower_metronome.so" "$WORK_DIR/lib/arm64-v8a/librower_metronome.so"
"$TOOLCHAIN/llvm-strip" --strip-unneeded "$WORK_DIR/lib/arm64-v8a/librower_metronome.so"

echo "=== 2. Compiling Android resources & manifest ==="
"$BUILD_TOOLS/aapt2" compile --dir "$ROOT_DIR/android/res" -o "$WORK_DIR/res.zip"
"$BUILD_TOOLS/aapt2" link \
  -o "$WORK_DIR/base.apk" \
  -I "$ANDROID_JAR" \
  --manifest "$ROOT_DIR/android/AndroidManifest.xml" \
  --min-sdk-version 26 \
  --target-sdk-version "$PLATFORM_VER" \
  --version-code 1 \
  --version-name "$VERSION" \
  "$WORK_DIR/res.zip"

echo "=== 3. Compiling MainActivity.java -> classes.dex ==="
javac --release 11 \
  -classpath "$ANDROID_JAR" \
  -d "$WORK_DIR/classes" \
  "$ROOT_DIR/android/src/com/ergmetronome/app/MainActivity.java"

"$BUILD_TOOLS/d8" \
  --min-api 26 \
  --lib "$ANDROID_JAR" \
  --output "$WORK_DIR/dex" \
  "$WORK_DIR/classes/com/ergmetronome/app/MainActivity.class"

echo "=== 4. Packing DEX & uncompressed 16KB-aligned native library ==="
python3 - "$WORK_DIR/base.apk" "$WORK_DIR/unaligned.apk" "$WORK_DIR/dex/classes.dex" "$WORK_DIR/lib/arm64-v8a/librower_metronome.so" <<'PY'
import sys, zipfile

base_apk, out_apk, dex_path, so_path = sys.argv[1:5]
with zipfile.ZipFile(base_apk, "r") as zin, zipfile.ZipFile(out_apk, "w") as zout:
    for item in zin.infolist():
        data = zin.read(item.filename)
        zout.writestr(item, data)
    zout.write(dex_path, "classes.dex", compress_type=zipfile.ZIP_DEFLATED)
    # extractNativeLibs="false" requires .so files to be stored uncompressed.
    zout.write(so_path, "lib/arm64-v8a/librower_metronome.so", compress_type=zipfile.ZIP_STORED)
PY

echo "=== 5. Zip-aligning (16KB page alignment for Pixel / Android 15+) ==="
"$BUILD_TOOLS/zipalign" -P 16 -f 4 "$WORK_DIR/unaligned.apk" "$WORK_DIR/aligned.apk"

echo "=== 6. Signing APK ==="
KEYSTORE="$HOME/.android/erg-metronome-debug.keystore"
mkdir -p "$HOME/.android"
if [ ! -f "$KEYSTORE" ]; then
  keytool -genkeypair -v \
    -keystore "$KEYSTORE" \
    -storepass android \
    -alias ergmetronome \
    -keypass android \
    -keyalg RSA \
    -keysize 2048 \
    -validity 10000 \
    -dname "CN=Erg Metronome,O=Local,C=US" >/dev/null 2>&1
fi

"$BUILD_TOOLS/apksigner" sign \
  --ks "$KEYSTORE" \
  --ks-key-alias ergmetronome \
  --ks-pass pass:android \
  --key-pass pass:android \
  --out "$FINAL_APK" \
  "$WORK_DIR/aligned.apk"

echo "=== 7. Verifying signature & 16KB page alignment ==="
"$BUILD_TOOLS/apksigner" verify --verbose "$FINAL_APK"
"$BUILD_TOOLS/zipalign" -c -P 16 4 "$FINAL_APK"
echo "16KB zip alignment: OK"
ls -lh "$FINAL_APK"
