#!/usr/bin/env bash
# Build do corretor_universal no Windows.
#
# O rustc para o target x86_64-pc-windows-gnu precisa do dlltool.exe (binutils)
# no PATH alem do linker configurado em .cargo/config.toml. Ambos veem do
# MinGW-w64 portatil (w64devkit) instalado em %USERPROFILE%\tools.
set -euo pipefail

export PATH="$HOME/tools/w64devkit/w64devkit/bin:$HOME/.cargo/bin:$PATH"

cd "$(dirname "$0")"
cargo build --release "$@"

echo
echo "binario: $(pwd)/target/release/corretor_universal.exe"
