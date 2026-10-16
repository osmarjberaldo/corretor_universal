#!/usr/bin/env bash
# Script de build multiplataforma do Corretor Universal.
set -euo pipefail

OS="$(uname -s 2>/dev/null || echo "Windows")"

case "$OS" in
    MINGW*|MSYS*|CYGWIN*)
        # Windows (Git Bash / MSYS2 / Cygwin)
        if [ -d "$HOME/tools/w64devkit/w64devkit/bin" ]; then
            export PATH="$HOME/tools/w64devkit/w64devkit/bin:$HOME/.cargo/bin:$PATH"
        else
            export PATH="$HOME/.cargo/bin:$PATH"
        fi
        BIN_NAME="corretor_universal.exe"
        ;;
    Linux*|Darwin*)
        # Linux ou macOS
        export PATH="$HOME/.cargo/bin:$PATH"
        BIN_NAME="corretor_universal"
        ;;
    *)
        export PATH="$HOME/.cargo/bin:$PATH"
        BIN_NAME="corretor_universal"
        ;;
esac

cd "$(dirname "$0")"
cargo build --release "$@"

echo
echo "binario: $(pwd)/target/release/${BIN_NAME}"

