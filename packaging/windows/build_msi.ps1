# Gera o instalador MSI (Windows Installer) do Corretor Universal.
#
# Pré-requisitos:
#   - Rust/cargo (o script compila em release);
#   - WiX Toolset v7:  winget install WiXToolset.WiXCLI
#     (na 1ª vez o WiX pede o aceite da EULA OSMF: `wix eula accept wix7`).
#
# Uso:
#   powershell -ExecutionPolicy Bypass -File packaging/windows/build_msi.ps1 [-Tag v1.1.0]
param (
    [string]$Tag = ""
)

$ErrorActionPreference = "Stop"

$w64Path = "$env:USERPROFILE\tools\w64devkit\w64devkit\bin"
$cargoPath = "$env:USERPROFILE\.cargo\bin"
if (Test-Path $w64Path) {
    $env:PATH = "$w64Path;$cargoPath;$env:PATH"
} elseif (Test-Path $cargoPath) {
    $env:PATH = "$cargoPath;$env:PATH"
}

$RootDir = (Resolve-Path "$PSScriptRoot\..\..").Path
Set-Location $RootDir

# Versão: usa o -Tag informado ou lê do Cargo.toml.
if (-not $Tag) {
    $linha = Select-String -Path "Cargo.toml" -Pattern '^\s*version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if (-not $linha) {
        Write-Host "[ERRO] Nao encontrei a versao no Cargo.toml." -ForegroundColor Red
        exit 1
    }
    $Tag = "v" + $linha.Matches[0].Groups[1].Value
}
$Versao = $Tag.TrimStart("v")

Write-Host "=== Gerando instalador MSI do Corretor Universal ($Tag) ===" -ForegroundColor Cyan

# Localiza o WiX Toolset.
$wix = (Get-Command wix -ErrorAction SilentlyContinue).Source
if (-not $wix) {
    foreach ($p in @(
        "C:\Program Files\WiX Toolset v7.0\bin\wix.exe",
        "${env:ProgramFiles(x86)}\WiX Toolset v3.14\bin\candle.exe"
    )) {
        if (Test-Path $p) { $wix = $p; break }
    }
}
if (-not $wix) {
    Write-Host "[ERRO] WiX Toolset nao encontrado." -ForegroundColor Red
    Write-Host "Instale com: winget install --id WiXToolset.WiXCLI -e" -ForegroundColor Yellow
    exit 1
}

Write-Host "Compilando binario em modo release..." -ForegroundColor Yellow
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERRO] Falha na compilacao." -ForegroundColor Red
    exit $LASTEXITCODE
}

# Extensão de UI do WiX (idempotente; ignora se já instalada).
& $wix extension add WixToolset.UI.wixext 2>$null | Out-Null

$DistDir = Join-Path $RootDir "dist"
New-Item -ItemType Directory -Path $DistDir -Force | Out-Null
$MsiPath = Join-Path $DistDir "corretor_universal-${Tag}-windows-x86_64.msi"
if (Test-Path $MsiPath) { Remove-Item -Force $MsiPath }

$Exe = Join-Path $RootDir "target\release\corretor_universal.exe"
$Readme = Join-Path $RootDir "README.md"
$Licenca = Join-Path $RootDir "LICENSE"
$Icone = Join-Path $RootDir "icone.ico"
$LicencaRtf = Join-Path $PSScriptRoot "licenca.rtf"
$Wxs = Join-Path $PSScriptRoot "corretor_universal.wxs"

Write-Host "Montando MSI com WiX..." -ForegroundColor Yellow
& $wix --acceptEula wix7 build $Wxs `
    -arch x64 `
    -ext WixToolset.UI.wixext `
    -d "Versao=$Versao" `
    -d "Exe=$Exe" `
    -d "Readme=$Readme" `
    -d "Licenca=$Licenca" `
    -d "Icone=$Icone" `
    -d "LicencaRtf=$LicencaRtf" `
    -o $MsiPath
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERRO] Falha ao gerar o MSI." -ForegroundColor Red
    exit $LASTEXITCODE
}

$Hash = (Get-FileHash -Path $MsiPath -Algorithm SHA256).Hash
"$Hash  $MsiPath" | Out-File "${MsiPath}.sha256" -Encoding utf8

Write-Host "`nPacote MSI gerado com sucesso!" -ForegroundColor Green
Write-Host "Arquivo: $MsiPath" -ForegroundColor Green
Write-Host "SHA256:  $Hash" -ForegroundColor Green
