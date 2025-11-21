# Guide de Build Multi-Plateforme

Ce guide explique comment créer des binaires exécutables pour toutes les plateformes.

## 📦 Formats de distribution

### macOS
- **DMG** - Image disque pour installation drag-and-drop
- **App Bundle** - Application macOS native (.app)
- Architecture: Universal Binary (Intel x64 + Apple Silicon ARM64)

### Windows
- **MSI** - Installateur Windows standard
- **NSIS** - Installateur compact (.exe)
- Architecture: x64

### Linux
- **AppImage** - Portable, fonctionne sur toutes les distributions
- **DEB** - Pour Debian/Ubuntu
- **RPM** - Pour Fedora/RHEL/openSUSE
- Architecture: x64

## 🚀 Build Automatique (Recommandé)

### Via GitHub Actions

1. **Créer un tag Git** :
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

2. Le workflow GitHub Actions va automatiquement :
   - Builder sur macOS, Linux et Windows
   - Créer tous les formats d'installation
   - Créer une release GitHub avec les binaires

3. **Télécharger les binaires** :
   - Allez dans l'onglet "Actions" de votre repo GitHub
   - Ou dans "Releases" pour les releases publiées

### Build manuel via GitHub Actions

Vous pouvez aussi déclencher un build manuellement :
1. Allez dans l'onglet "Actions"
2. Sélectionnez "Build Multi-Platform Release"
3. Cliquez sur "Run workflow"

## 🛠️ Build Local

### Prérequis

#### macOS
```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Dependencies
xcode-select --install
```

#### Linux (Ubuntu/Debian)
```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Dependencies
sudo apt update
sudo apt install -y libgtk-3-dev libwebkit2gtk-4.1-dev \
  libappindicator3-dev librsvg2-dev patchelf
```

#### Windows
```bash
# Installer Rust depuis https://rustup.rs/
# Installer Visual Studio Build Tools
# Installer WebView2 Runtime
```

### Commandes de Build

#### Build pour votre plateforme actuelle
```bash
npm install
npm run tauri:build
```

Les binaires seront dans :
- macOS: `src-tauri/target/release/bundle/dmg/` et `src-tauri/target/release/bundle/macos/`
- Linux: `src-tauri/target/release/bundle/deb/`, `appimage/`, `rpm/`
- Windows: `src-tauri/target/release/bundle/msi/` et `nsis/`

#### Build pour une plateforme spécifique

**macOS Universal Binary** (fonctionne sur Intel et Apple Silicon) :
```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm run build:macos
```

**Linux x64** :
```bash
rustup target add x86_64-unknown-linux-gnu
npm run build:linux
```

**Windows x64** :
```bash
rustup target add x86_64-pc-windows-msvc
npm run build:windows
```

## 🎯 Cross-Compilation

### Limitations

Tauri/Rust a des limitations pour la cross-compilation :

❌ **Impossible** :
- Builder pour macOS depuis Windows/Linux
- Builder pour Windows depuis macOS/Linux (sans VM)

✅ **Possible** :
- Builder pour Linux ARM depuis Linux x64
- Builder pour Windows ARM depuis Windows x64

### Solution : Utiliser GitHub Actions

La meilleure solution est d'utiliser GitHub Actions qui fournit des runners natifs pour chaque plateforme.

## 📋 Checklist avant Release

- [ ] Tester l'application sur chaque plateforme
- [ ] Mettre à jour le numéro de version dans :
  - [ ] `package.json`
  - [ ] `src-tauri/tauri.conf.json`
  - [ ] `src-tauri/Cargo.toml`
- [ ] Vérifier que tous les tests passent
- [ ] Créer un changelog
- [ ] Créer et pousser le tag Git
- [ ] Vérifier que le workflow GitHub Actions réussit
- [ ] Télécharger et tester chaque binaire

## 🔍 Vérification des Binaires

### macOS
```bash
# Vérifier l'architecture
lipo -archs S3\ Browser.app/Contents/MacOS/s3-browser
# Devrait afficher: x86_64 arm64
```

### Linux
```bash
# Vérifier l'architecture
file s3browser_0.1.0_amd64.AppImage
# Devrait afficher: x86-64
```

### Windows
```powershell
# Dans PowerShell
dumpbin /headers S3Browser.exe | findstr machine
# Devrait afficher: 8664 machine (x64)
```

## 🐛 Troubleshooting

### Erreur de signature (macOS)
Si vous avez des erreurs de signature sur macOS :
```bash
# Désactiver la signature temporairement pour le dev
export TAURI_PRIVATE_KEY=""
export TAURI_KEY_PASSWORD=""
```

### Erreur WebView2 (Windows)
Installer WebView2 Runtime :
https://developer.microsoft.com/en-us/microsoft-edge/webview2/

### Erreur de dépendances (Linux)
```bash
# Ubuntu/Debian
sudo apt install -y libgtk-3-0 libwebkit2gtk-4.1-0

# Fedora
sudo dnf install gtk3 webkit2gtk4.1
```

## 📦 Tailles des Binaires

Tailles approximatives :
- **macOS DMG** : ~15-20 MB (Universal Binary)
- **Windows MSI** : ~12-15 MB
- **Linux AppImage** : ~18-22 MB
- **Linux DEB/RPM** : ~10-12 MB

## 🔐 Signature des Applications

### macOS (code signing)
Nécessite un Apple Developer Account (99$/an)
```bash
# Configurer dans tauri.conf.json
"bundle": {
  "macOS": {
    "signingIdentity": "Developer ID Application: Your Name"
  }
}
```

### Windows (code signing)
Nécessite un certificat de signature de code
```bash
# Configurer les variables d'environnement
TAURI_PRIVATE_KEY="path/to/cert.pfx"
TAURI_KEY_PASSWORD="password"
```

## 📝 Notes Importantes

1. **Universal Binary macOS** : Le build génère automatiquement un binaire universel qui fonctionne sur Intel et Apple Silicon
2. **AppImage Linux** : Format portable recommandé, fonctionne sur toutes les distributions modernes
3. **Taille des binaires** : Rust génère des binaires optimisés en mode release
4. **Updater** : Tauri supporte les mises à jour automatiques (à configurer séparément)

## 🌐 Distribution

### Méthodes recommandées :
1. **GitHub Releases** - Gratuit, simple, inclus dans le workflow
2. **Site web** - Héberger les binaires sur votre propre serveur
3. **Store** :
   - Mac App Store (nécessite Apple Developer)
   - Microsoft Store (nécessite compte développeur)
   - Snap Store / Flatpak (Linux)

## 🤝 Support

Pour plus d'informations :
- [Documentation Tauri](https://tauri.app/v1/guides/building/)
- [Tauri Bundle](https://tauri.app/v1/guides/building/linux)
- [GitHub Actions](https://github.com/tauri-apps/tauri-action)
