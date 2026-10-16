#!/usr/bin/env bash
set -e

TAG="${1:-v0.1.0}"
APP_NAME="Corretor Universal"
BUNDLE_DIR="${APP_NAME}.app"
CONTENTS_DIR="${BUNDLE_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

echo "=== Criando macOS Universal App Bundle (${TAG}) ==="

rm -rf "${BUNDLE_DIR}"
mkdir -p "${MACOS_DIR}" "${RESOURCES_DIR}"

# Cria o binário Universal combinando Apple Silicon (aarch64) e Intel (x86_64) se ambos existirem
if [ -f "target/aarch64-apple-darwin/release/corretor_universal" ] && [ -f "target/x86_64-apple-darwin/release/corretor_universal" ]; then
    echo "Combinando aarch64 + x86_64 com lipo..."
    lipo -create \
        target/aarch64-apple-darwin/release/corretor_universal \
        target/x86_64-apple-darwin/release/corretor_universal \
        -output "${MACOS_DIR}/corretor_universal"
elif [ -f "target/release/corretor_universal" ]; then
    echo "Usando binário release nativo..."
    cp "target/release/corretor_universal" "${MACOS_DIR}/corretor_universal"
elif [ -f "target/aarch64-apple-darwin/release/corretor_universal" ]; then
    cp "target/aarch64-apple-darwin/release/corretor_universal" "${MACOS_DIR}/corretor_universal"
elif [ -f "target/x86_64-apple-darwin/release/corretor_universal" ]; then
    cp "target/x86_64-apple-darwin/release/corretor_universal" "${MACOS_DIR}/corretor_universal"
else
    echo "Erro: binário do corretor_universal não encontrado."
    exit 1
fi
chmod +x "${MACOS_DIR}/corretor_universal"

# Info.plist
cp packaging/macos/Info.plist "${CONTENTS_DIR}/Info.plist"
VERSION_CLEAN=$(echo "$TAG" | sed 's/^v//')
sed -i '' "s/0.1.0/${VERSION_CLEAN}/g" "${CONTENTS_DIR}/Info.plist" 2>/dev/null || true

# Ícone .icns a partir do icone.png
if [ -f "icone.png" ] && command -v sips >/dev/null 2>&1 && command -v iconutil >/dev/null 2>&1; then
    echo "Gerando AppIcon.icns de alta resolução..."
    ICONSET="AppIcon.iconset"
    mkdir -p "${ICONSET}"
    sips -z 16 16     icone.png --out "${ICONSET}/icon_16x16.png" >/dev/null 2>&1 || true
    sips -z 32 32     icone.png --out "${ICONSET}/icon_16x16@2x.png" >/dev/null 2>&1 || true
    sips -z 32 32     icone.png --out "${ICONSET}/icon_32x32.png" >/dev/null 2>&1 || true
    sips -z 64 64     icone.png --out "${ICONSET}/icon_32x32@2x.png" >/dev/null 2>&1 || true
    sips -z 128 128   icone.png --out "${ICONSET}/icon_128x128.png" >/dev/null 2>&1 || true
    sips -z 256 256   icone.png --out "${ICONSET}/icon_128x128@2x.png" >/dev/null 2>&1 || true
    sips -z 256 256   icone.png --out "${ICONSET}/icon_256x256.png" >/dev/null 2>&1 || true
    sips -z 512 512   icone.png --out "${ICONSET}/icon_256x256@2x.png" >/dev/null 2>&1 || true
    sips -z 512 512   icone.png --out "${ICONSET}/icon_512x512.png" >/dev/null 2>&1 || true
    sips -z 1024 1024 icone.png --out "${ICONSET}/icon_512x512@2x.png" >/dev/null 2>&1 || true
    iconutil -c icns "${ICONSET}" -o "${RESOURCES_DIR}/AppIcon.icns" 2>/dev/null || true
    rm -rf "${ICONSET}"
fi

# Assinatura ad-hoc
if command -v codesign >/dev/null 2>&1; then
    codesign --force --deep --sign - "${BUNDLE_DIR}" 2>/dev/null || true
fi

ZIP_NAME="corretor_universal-${TAG}-macos-universal.zip"
zip -r -y "${ZIP_NAME}" "${BUNDLE_DIR}" README.md LICENSE
if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "${ZIP_NAME}" > "${ZIP_NAME}.sha256"
fi

echo "✅ App bundle gerado com sucesso: ${ZIP_NAME}"
