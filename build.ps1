# Build do Corretor Universal no PowerShell
Write-Host "Compilando Corretor Universal no Windows..." -ForegroundColor Cyan

$w64Path = "$env:USERPROFILE\tools\w64devkit\w64devkit\bin"
$cargoPath = "$env:USERPROFILE\.cargo\bin"

if (Test-Path $w64Path) {
    $env:PATH = "$w64Path;$cargoPath;$env:PATH"
} elseif (Test-Path $cargoPath) {
    $env:PATH = "$cargoPath;$env:PATH"
}

Set-Location $PSScriptRoot
cargo build --release $args

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n[SUCESSO] Binario gerado em: target\release\corretor_universal.exe" -ForegroundColor Green
} else {
    Write-Host "`n[ERRO] Falha na compilacao." -ForegroundColor Red
    exit $LASTEXITCODE
}
