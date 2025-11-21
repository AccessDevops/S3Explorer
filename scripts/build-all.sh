#!/bin/bash

# Script de build multi-plateforme pour S3 Browser
# Ce script aide à builder pour plusieurs targets, mais note que
# certaines cross-compilations ne sont pas possibles sans VM/Docker

set -e

echo "🚀 S3 Browser - Build Multi-Plateforme"
echo "======================================"
echo ""

# Détection de la plateforme
OS_TYPE="$(uname -s)"
case "${OS_TYPE}" in
    Linux*)     MACHINE=Linux;;
    Darwin*)    MACHINE=Mac;;
    CYGWIN*|MINGW*|MSYS*) MACHINE=Windows;;
    *)          MACHINE="UNKNOWN:${OS_TYPE}"
esac

echo "🖥️  Plateforme détectée: $MACHINE"
echo ""

# Vérification des dépendances
echo "🔍 Vérification des dépendances..."

if ! command -v node &> /dev/null; then
    echo "❌ Node.js n'est pas installé"
    exit 1
fi

if ! command -v npm &> /dev/null; then
    echo "❌ npm n'est pas installé"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "❌ Rust n'est pas installé"
    echo "   Installez Rust depuis: https://rustup.rs/"
    exit 1
fi

echo "✅ Toutes les dépendances sont présentes"
echo ""

# Installation des dépendances npm
echo "📦 Installation des dépendances npm..."
npm install

echo ""
echo "🎯 Plateformes disponibles pour build depuis $MACHINE:"
echo ""

case "${MACHINE}" in
    Mac)
        echo "  ✅ macOS (Universal Binary - Intel + Apple Silicon)"
        echo "  ⚠️  Linux (nécessite Docker ou VM)"
        echo "  ⚠️  Windows (nécessite Docker ou VM)"
        echo ""
        echo "🔨 Building pour macOS Universal Binary..."

        # Installer les targets macOS
        rustup target add aarch64-apple-darwin x86_64-apple-darwin

        # Build
        npm run tauri:build -- --target universal-apple-darwin

        echo ""
        echo "✅ Build terminé!"
        echo "📁 Fichiers générés dans:"
        echo "   - src-tauri/target/universal-apple-darwin/release/bundle/dmg/"
        echo "   - src-tauri/target/universal-apple-darwin/release/bundle/macos/"
        ;;

    Linux)
        echo "  ✅ Linux x64"
        echo "  ✅ Linux ARM64 (avec rustup target)"
        echo "  ⚠️  macOS (impossible depuis Linux)"
        echo "  ⚠️  Windows (nécessite Docker ou VM)"
        echo ""

        # Vérifier les dépendances Linux
        if ! dpkg -l | grep -q libgtk-3-dev; then
            echo "⚠️  Certaines dépendances système peuvent manquer"
            echo "   Installez-les avec:"
            echo "   sudo apt install -y libgtk-3-dev libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev patchelf"
            echo ""
        fi

        echo "🔨 Building pour Linux x64..."
        rustup target add x86_64-unknown-linux-gnu
        npm run tauri:build

        echo ""
        echo "✅ Build terminé!"
        echo "📁 Fichiers générés dans:"
        echo "   - src-tauri/target/release/bundle/deb/"
        echo "   - src-tauri/target/release/bundle/appimage/"
        echo "   - src-tauri/target/release/bundle/rpm/"
        ;;

    Windows)
        echo "  ✅ Windows x64"
        echo "  ⚠️  macOS (impossible depuis Windows)"
        echo "  ⚠️  Linux (nécessite WSL2 ou VM)"
        echo ""
        echo "🔨 Building pour Windows x64..."

        rustup target add x86_64-pc-windows-msvc
        npm run tauri:build

        echo ""
        echo "✅ Build terminé!"
        echo "📁 Fichiers générés dans:"
        echo "   - src-tauri/target/release/bundle/msi/"
        echo "   - src-tauri/target/release/bundle/nsis/"
        ;;

    *)
        echo "❌ Plateforme non supportée: $MACHINE"
        exit 1
        ;;
esac

echo ""
echo "💡 Pour builder pour toutes les plateformes:"
echo "   Utilisez GitHub Actions en créant un tag:"
echo "   git tag v0.1.0 && git push origin v0.1.0"
echo ""
echo "📖 Consultez BUILD.md pour plus d'informations"
