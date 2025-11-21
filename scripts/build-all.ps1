# Script de build multi-plateforme pour S3 Browser (Windows)
# PowerShell version

$ErrorActionPreference = "Stop"

Write-Host "🚀 S3 Browser - Build Multi-Plateforme" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "🖥️  Plateforme: Windows" -ForegroundColor Yellow
Write-Host ""

# Vérification des dépendances
Write-Host "🔍 Vérification des dépendances..." -ForegroundColor Yellow

$dependencies = @{
    "node" = "Node.js"
    "npm" = "npm"
    "cargo" = "Rust"
}

$missingDeps = @()

foreach ($cmd in $dependencies.Keys) {
    try {
        $null = Get-Command $cmd -ErrorAction Stop
        Write-Host "  ✅ $($dependencies[$cmd])" -ForegroundColor Green
    }
    catch {
        Write-Host "  ❌ $($dependencies[$cmd]) n'est pas installé" -ForegroundColor Red
        $missingDeps += $dependencies[$cmd]
    }
}

if ($missingDeps.Count -gt 0) {
    Write-Host ""
    Write-Host "❌ Dépendances manquantes:" -ForegroundColor Red
    foreach ($dep in $missingDeps) {
        Write-Host "   - $dep" -ForegroundColor Red
    }
    Write-Host ""
    Write-Host "Installez les dépendances manquantes:" -ForegroundColor Yellow
    Write-Host "  - Node.js: https://nodejs.org/"
    Write-Host "  - Rust: https://rustup.rs/"
    exit 1
}

Write-Host ""

# Installation des dépendances npm
Write-Host "📦 Installation des dépendances npm..." -ForegroundColor Yellow
npm install

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Erreur lors de l'installation des dépendances" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "🎯 Plateformes disponibles pour build depuis Windows:" -ForegroundColor Cyan
Write-Host "  ✅ Windows x64" -ForegroundColor Green
Write-Host "  ⚠️  macOS (impossible depuis Windows)" -ForegroundColor Yellow
Write-Host "  ⚠️  Linux (nécessite WSL2 ou VM)" -ForegroundColor Yellow
Write-Host ""

# Vérifier WebView2
Write-Host "🔍 Vérification de WebView2..." -ForegroundColor Yellow
$webview2Path = "${env:ProgramFiles(x86)}\Microsoft\EdgeWebView\Application"
if (Test-Path $webview2Path) {
    Write-Host "  ✅ WebView2 Runtime installé" -ForegroundColor Green
} else {
    Write-Host "  ⚠️  WebView2 Runtime non détecté" -ForegroundColor Yellow
    Write-Host "     Téléchargez-le depuis:" -ForegroundColor Yellow
    Write-Host "     https://developer.microsoft.com/en-us/microsoft-edge/webview2/" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "🔨 Building pour Windows x64..." -ForegroundColor Cyan

# Installer le target Windows x64
rustup target add x86_64-pc-windows-msvc

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Erreur lors de l'installation du target Rust" -ForegroundColor Red
    exit 1
}

# Build
npm run tauri:build

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Erreur lors du build" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "✅ Build terminé!" -ForegroundColor Green
Write-Host "📁 Fichiers générés dans:" -ForegroundColor Cyan
Write-Host "   - src-tauri\target\release\bundle\msi\" -ForegroundColor White
Write-Host "   - src-tauri\target\release\bundle\nsis\" -ForegroundColor White
Write-Host ""

# Lister les fichiers générés
Write-Host "📦 Binaires créés:" -ForegroundColor Cyan
$bundlePath = "src-tauri\target\release\bundle"

if (Test-Path "$bundlePath\msi") {
    Get-ChildItem "$bundlePath\msi\*.msi" | ForEach-Object {
        $size = [math]::Round($_.Length / 1MB, 2)
        Write-Host "   MSI: $($_.Name) ($size MB)" -ForegroundColor White
    }
}

if (Test-Path "$bundlePath\nsis") {
    Get-ChildItem "$bundlePath\nsis\*.exe" | ForEach-Object {
        $size = [math]::Round($_.Length / 1MB, 2)
        Write-Host "   EXE: $($_.Name) ($size MB)" -ForegroundColor White
    }
}

Write-Host ""
Write-Host "💡 Pour builder pour toutes les plateformes:" -ForegroundColor Yellow
Write-Host "   Utilisez GitHub Actions en créant un tag:" -ForegroundColor Yellow
Write-Host "   git tag v0.1.0" -ForegroundColor White
Write-Host "   git push origin v0.1.0" -ForegroundColor White
Write-Host ""
Write-Host "📖 Consultez BUILD.md pour plus d'informations" -ForegroundColor Cyan
