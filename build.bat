@echo off
setlocal
echo Compilando Corretor Universal no Windows...

:: Configura o PATH com o MinGW portátil se existir, ou o Cargo padrão
if exist "%USERPROFILE%\tools\w64devkit\w64devkit\bin" (
    set "PATH=%USERPROFILE%\tools\w64devkit\w64devkit\bin;%USERPROFILE%\.cargo\bin;%PATH%"
) else (
    set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
)

cd /d "%~dp0"
cargo build --release %*

if errorlevel 1 (
    echo.
    echo [ERRO] Falha na compilacao.
    exit /b %errorlevel%
)

echo.
echo [SUCESSO] Binario gerado em: target\release\corretor_universal.exe
