@echo off
setlocal
echo Iniciando Corretor Universal em modo DEV...

if exist "%USERPROFILE%\tools\w64devkit\w64devkit\bin" (
    set "PATH=%USERPROFILE%\tools\w64devkit\w64devkit\bin;%USERPROFILE%\.cargo\bin;%PATH%"
) else (
    set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
)

cd /d "%~dp0"
cargo run
