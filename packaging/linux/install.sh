#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"

echo "=== Instalando Corretor Universal para Linux ==="

BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

if [ -f "$DIR/corretor_universal" ]; then
    cp "$DIR/corretor_universal" "$BIN_DIR/corretor_universal"
    chmod +x "$BIN_DIR/corretor_universal"
else
    echo "Erro: binário 'corretor_universal' não encontrado na pasta."
    exit 1
fi

if [ -f "$DIR/corretor-universal.png" ]; then
    cp "$DIR/corretor-universal.png" "$ICON_DIR/corretor-universal.png"
fi

if [ -f "$DIR/corretor-universal.desktop" ]; then
    cp "$DIR/corretor-universal.desktop" "$APP_DIR/corretor-universal.desktop"
    sed -i "s|Exec=corretor_universal|Exec=$BIN_DIR/corretor_universal|g" "$APP_DIR/corretor-universal.desktop" 2>/dev/null || true
fi

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" 2>/dev/null || true
fi

echo ""
echo "✅ Instalação concluída com sucesso!"
echo "O aplicativo foi instalado em: $BIN_DIR/corretor_universal"
echo "Você pode iniciá-lo pelo menu de aplicativos ou executando 'corretor_universal' no terminal."
echo "(Caso o comando não seja reconhecido de imediato, confirme que '$BIN_DIR' está no seu PATH)."
