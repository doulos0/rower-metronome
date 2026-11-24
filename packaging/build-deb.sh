#!/usr/bin/env bash
set -euo pipefail
umask 022

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

PKG_NAME="rower-metronome"
BIN_NAME="rower_metronome"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
ARCH="$(dpkg --print-architecture)"
OUT_DIR="$ROOT_DIR/target/debian"
STAGE_DIR="$ROOT_DIR/target/debian-stage"
DEB_PATH="$OUT_DIR/${PKG_NAME}_${VERSION}_${ARCH}.deb"

echo "Building release binary (${PKG_NAME} v${VERSION} for ${ARCH})..."
cargo build --release --bin "$BIN_NAME"

rm -rf "$STAGE_DIR"
mkdir -p \
  "$STAGE_DIR/DEBIAN" \
  "$STAGE_DIR/usr/bin" \
  "$STAGE_DIR/usr/share/applications" \
  "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps" \
  "$STAGE_DIR/usr/share/doc/${PKG_NAME}" \
  "$OUT_DIR"

# Install and strip binary
install -m 0755 "target/release/${BIN_NAME}" "$STAGE_DIR/usr/bin/${PKG_NAME}"
strip --strip-unneeded "$STAGE_DIR/usr/bin/${PKG_NAME}"

# Install desktop entry, icon, and documentation
install -m 0644 "packaging/rower-metronome.desktop" \
  "$STAGE_DIR/usr/share/applications/${PKG_NAME}.desktop"
install -m 0644 "packaging/rower-metronome.svg" \
  "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps/${PKG_NAME}.svg"
install -m 0644 "README.md" \
  "$STAGE_DIR/usr/share/doc/${PKG_NAME}/README.md"

# Detect shared library dependencies dynamically
SHLIB_DEPS=""
if command -v dpkg-shlibdeps >/dev/null 2>&1; then
  SHLIB_TMP="$(mktemp -d)"
  mkdir -p "$SHLIB_TMP/debian"
  printf 'Source: %s\n\nPackage: %s\nArchitecture: %s\n' "$PKG_NAME" "$PKG_NAME" "$ARCH" > "$SHLIB_TMP/debian/control"
  SHLIB_OUT="$(cd "$SHLIB_TMP" && dpkg-shlibdeps -O "$STAGE_DIR/usr/bin/${PKG_NAME}" 2>/dev/null || true)"
  rm -rf "$SHLIB_TMP"
  SHLIB_DEPS="${SHLIB_OUT#shlibs:Depends=}"
fi

RUNTIME_DEPS="libgl1, libegl1, libx11-6 | libwayland-client0, libxkbcommon0"
if [ -n "$SHLIB_DEPS" ]; then
  ALL_DEPS="${SHLIB_DEPS}, ${RUNTIME_DEPS}"
else
  ALL_DEPS="libc6, ${RUNTIME_DEPS}"
fi

INSTALLED_SIZE="$(du -sk "$STAGE_DIR/usr" | awk '{print $1}')"

cat > "$STAGE_DIR/DEBIAN/control" <<EOF
Package: ${PKG_NAME}
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Installed-Size: ${INSTALLED_SIZE}
Depends: ${ALL_DEPS}
Maintainer: Erg Metronome Maintainers <noreply@localhost>
Description: Concept2 rowing machine visual metronome (22-36 SPM)
 Displays an animated stick figure rowing a Concept2 RowErg in proper form
 (Legs-Body-Arms on drive; Arms-Body-Legs on recovery) with adjustable stroke
 rate from 22 to 36 strokes per minute.
EOF

cat > "$STAGE_DIR/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database -q /usr/share/applications || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi
EOF
chmod 0755 "$STAGE_DIR/DEBIAN/postinst"

cat > "$STAGE_DIR/DEBIAN/postrm" <<'EOF'
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database -q /usr/share/applications || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi
EOF
chmod 0755 "$STAGE_DIR/DEBIAN/postrm"

fakeroot dpkg-deb --build --root-owner-group "$STAGE_DIR" "$DEB_PATH"

echo ""
echo "Built Debian package: $DEB_PATH"
dpkg-deb --info "$DEB_PATH"
echo ""
echo "Package contents:"
dpkg-deb --contents "$DEB_PATH"
