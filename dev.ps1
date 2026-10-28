# Executa o Corretor Universal em modo DEV (com console de logs)
Write-Host "Iniciando Corretor Universal em modo DEV..." -ForegroundColor Cyan

$w64Path = "$env:USERPROFILE\tools\w64devkit\w64devkit\bin"
$cargoPath = "$env:USERPROFILE\.cargo\bin"

if (Test-Path $w64Path) {
    $env:PATH = "$w64Path;$cargoPath;$env:PATH"
} elseif (Test-Path $cargoPath) {
    $env:PATH = "$cargoPath;$env:PATH"
}

Set-Location $PSScriptRoot
cargo run
