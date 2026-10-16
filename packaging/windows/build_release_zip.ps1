# Script para compilar e gerar pacote zip de release para Windows localmente
param (
    [string]$Tag = "v0.1.0"
)

Write-Host "=== Gerando Pacote Release para Windows ($Tag) ===" -ForegroundColor Cyan

$w64Path = "$env:USERPROFILE\tools\w64devkit\w64devkit\bin"
$cargoPath = "$env:USERPROFILE\.cargo\bin"

if (Test-Path $w64Path) {
    $env:PATH = "$w64Path;$cargoPath;$env:PATH"
} elseif (Test-Path $cargoPath) {
    $env:PATH = "$cargoPath;$env:PATH"
}

$RootDir = "$PSScriptRoot\..\.."
Set-Location $RootDir

Write-Host "Compilando binário em modo release..." -ForegroundColor Yellow
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERRO] Falha na compilação." -ForegroundColor Red
    exit $LASTEXITCODE
}

$DistDir = "$RootDir\dist"
$OutDir = "$DistDir\corretor_universal-${Tag}-windows-x86_64"

if (Test-Path $OutDir) {
    Remove-Item -Recurse -Force $OutDir
}
New-Item -ItemType Directory -Path $OutDir -Force | Out-Null

Copy-Item "target\release\corretor_universal.exe" -Destination $OutDir
Copy-Item "README.md" -Destination $OutDir
Copy-Item "LICENSE" -Destination $OutDir

Start-Sleep -Milliseconds 500

$ZipPath = "$DistDir\corretor_universal-${Tag}-windows-x86_64.zip"
if (Test-Path $ZipPath) {
    Remove-Item -Force $ZipPath
}

Write-Host "Compactando em $ZipPath..." -ForegroundColor Yellow
Compress-Archive -Path "$OutDir\*" -DestinationPath $ZipPath -Force

$Hash = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash
"$Hash  $ZipPath" | Out-File "${ZipPath}.sha256" -Encoding utf8

Write-Host "`n✅ Pacote Windows gerado com sucesso!" -ForegroundColor Green
Write-Host "Arquivo: $ZipPath" -ForegroundColor Green
Write-Host "SHA256:  $Hash" -ForegroundColor Green
