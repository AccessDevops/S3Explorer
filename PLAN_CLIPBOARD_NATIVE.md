# Plan: Implémentation Native du Clipboard pour Windows et Linux

## Contexte

Actuellement, la lecture des fichiers copiés depuis le gestionnaire de fichiers fonctionne uniquement sur **macOS** via `NSPasteboard`. Sur Windows et Linux, le fallback utilise `arboard.get_text()` qui ne lit que le texte, pas les chemins de fichiers copiés depuis Explorer/Nautilus.

## Objectif

Implémenter la lecture native des fichiers du clipboard pour:
- **Windows**: Format CF_HDROP (ce que Explorer utilise pour "Copier")
- **Linux**: Format `text/uri-list` (X11 et Wayland)

---

## Étape 1: Mise à jour des dépendances (Cargo.toml)

Ajouter les dépendances spécifiques par plateforme:

```toml
# Windows-specific clipboard with CF_HDROP support
[target.'cfg(target_os = "windows")'.dependencies]
clipboard-win = "5"

# Linux - arboard with x11 feature already works, but we need to handle uri-list manually
# Note: arboard is already in dependencies, we keep it for text fallback
```

---

## Étape 2: Implémentation Windows (commands.rs)

Créer la fonction `read_clipboard_files_windows()`:

```rust
/// Windows-specific implementation using CF_HDROP format
#[cfg(target_os = "windows")]
fn read_clipboard_files_windows() -> Result<Vec<String>, String> {
    use clipboard_win::{formats, get_clipboard};

    // Try to get file list from clipboard (CF_HDROP format)
    match get_clipboard::<Vec<String>, _>(formats::FileList) {
        Ok(files) => {
            // Filter to only existing files
            let valid_files: Vec<String> = files
                .into_iter()
                .filter(|path| std::path::Path::new(path).exists())
                .collect();

            println!("[clipboard-win] Found {} files", valid_files.len());
            Ok(valid_files)
        }
        Err(e) => {
            println!("[clipboard-win] No files in clipboard: {}", e);
            // Fallback to text parsing
            read_clipboard_files_fallback()
        }
    }
}
```

---

## Étape 3: Implémentation Linux (commands.rs)

Créer la fonction `read_clipboard_files_linux()` qui gère le format `text/uri-list`:

```rust
/// Linux-specific implementation using text/uri-list format
#[cfg(target_os = "linux")]
fn read_clipboard_files_linux() -> Result<Vec<String>, String> {
    use std::process::Command;

    // Try xclip first (X11)
    let xclip_result = Command::new("xclip")
        .args(["-selection", "clipboard", "-t", "text/uri-list", "-o"])
        .output();

    if let Ok(output) = xclip_result {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let paths = parse_uri_list(&text);
            if !paths.is_empty() {
                println!("[clipboard-linux] Found {} files via xclip", paths.len());
                return Ok(paths);
            }
        }
    }

    // Try xsel as fallback (X11)
    let xsel_result = Command::new("xsel")
        .args(["--clipboard", "--output"])
        .output();

    if let Ok(output) = xsel_result {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let paths = parse_uri_list(&text);
            if !paths.is_empty() {
                println!("[clipboard-linux] Found {} files via xsel", paths.len());
                return Ok(paths);
            }
        }
    }

    // Try wl-paste for Wayland
    let wl_result = Command::new("wl-paste")
        .args(["--type", "text/uri-list"])
        .output();

    if let Ok(output) = wl_result {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let paths = parse_uri_list(&text);
            if !paths.is_empty() {
                println!("[clipboard-linux] Found {} files via wl-paste", paths.len());
                return Ok(paths);
            }
        }
    }

    // Fallback to arboard text
    read_clipboard_files_fallback()
}

/// Parse text/uri-list format into file paths
fn parse_uri_list(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| !line.starts_with('#')) // Skip comments
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("file://") {
                // Remove file:// prefix and decode URL
                let path = line.strip_prefix("file://").unwrap();
                let decoded = urlencoding_decode(path);
                if std::path::Path::new(&decoded).exists() {
                    return Some(decoded);
                }
            }
            None
        })
        .collect()
}
```

---

## Étape 4: Mise à jour du routeur principal

Modifier `read_clipboard_files()` pour router vers les implémentations natives:

```rust
#[tauri::command]
pub async fn read_clipboard_files() -> Result<Vec<String>, String> {
    spawn_blocking(|| {
        #[cfg(target_os = "macos")]
        {
            read_clipboard_files_macos()
        }
        #[cfg(target_os = "windows")]
        {
            read_clipboard_files_windows()
        }
        #[cfg(target_os = "linux")]
        {
            read_clipboard_files_linux()
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}
```

---

## Étape 5: Compilation conditionnelle du fallback

Garder `read_clipboard_files_fallback()` uniquement pour les cas non gérés:

```rust
/// Fallback implementation using arboard text
#[cfg(any(
    target_os = "linux",
    target_os = "windows",
    not(any(target_os = "macos", target_os = "windows", target_os = "linux"))
))]
fn read_clipboard_files_fallback() -> Result<Vec<String>, String> {
    // Code existant...
}
```

---

## Tests à effectuer

### Windows
1. Ouvrir Explorer, copier un fichier (Ctrl+C)
2. Dans l'application, vérifier que le fichier est détecté
3. Tester avec plusieurs fichiers sélectionnés

### Linux (X11)
1. Ouvrir Nautilus/Dolphin, copier un fichier
2. Vérifier que `xclip` ou `xsel` est installé
3. Tester la détection du fichier

### Linux (Wayland)
1. Sur Wayland (GNOME 40+, KDE Plasma Wayland)
2. Vérifier que `wl-paste` est installé (`wl-clipboard` package)
3. Tester la détection du fichier

---

## Dépendances système requises

### Linux (optionnel mais recommandé)
```bash
# X11
sudo apt install xclip xsel

# Wayland
sudo apt install wl-clipboard
```

Note: L'application fonctionnera même sans ces outils (fallback au texte), mais la fonctionnalité de copie de fichiers nécessite au moins un d'entre eux.

---

## Résumé des changements

| Fichier | Modification |
|---------|--------------|
| `Cargo.toml` | Ajouter `clipboard-win = "5"` pour Windows |
| `commands.rs` | Ajouter `read_clipboard_files_windows()` |
| `commands.rs` | Ajouter `read_clipboard_files_linux()` et `parse_uri_list()` |
| `commands.rs` | Modifier le routeur `read_clipboard_files()` |

---

## Estimation de complexité

- **Faible risque**: Les implémentations sont isolées par plateforme
- **Rétrocompatible**: Le fallback texte est conservé
- **Testable**: Chaque plateforme peut être testée indépendamment
