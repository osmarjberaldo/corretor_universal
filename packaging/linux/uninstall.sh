#!/usr/bin/env bash
set -e

echo "=== Desinstalando Corretor Universal ==="
rm -f "$HOME/.local/bin/corretor_universal"
rm -f "$HOME/.local/share/applications/corretor-universal.desktop"
rm -f "$HOME/.local/share/icons/hicolor/256x256/apps/corretor-universal.png"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
fi

echo "✅ Desinstalação concluída com sucesso."
