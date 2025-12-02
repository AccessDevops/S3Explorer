# Plan d'implémentation : Upload par Coller (CTRL+V / CMD+V)

## Résumé de la fonctionnalité

Permettre à l'utilisateur de coller du contenu depuis le clipboard directement dans l'ObjectBrowser pour l'uploader automatiquement dans le répertoire S3 actuel.

---

## Spécifications validées

| Spécification | Décision |
|---------------|----------|
| Confirmation avant upload | ✅ Oui, dialogue de confirmation |
| Nommage screenshots | `screenshot_YYYY-MM-DD_HH-mm-ss.png` |
| Nommage texte | `pasted_text_YYYY-MM-DD_HH-mm-ss.txt` |
| Conflits de noms | Ajouter suffixe `(1)`, `(2)`, etc. |
| Texte dans clipboard | Créer fichier `.txt` |
| Fichiers multiples | Oui (pour fichiers copiés depuis OS) |
| Feedback | Modal RustUploadProgress existante |
| Désactiver sur inputs | Oui, ignorer si un input/textarea a le focus |
| Scope | Uniquement dans ObjectBrowser |

---

## Architecture technique

```
┌─────────────────────────────────────────────────────────────┐
│                     ObjectBrowser.vue                        │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │              useClipboardUpload.ts                      │ │
│  │                                                          │ │
│  │  • Écoute événement 'paste' sur document                │ │
│  │  • Vérifie que le focus n'est pas sur un input          │ │
│  │  • Lit le clipboard (images, texte, fichiers)           │ │
│  │  • Génère les noms de fichiers                          │ │
│  │  • Vérifie les conflits de noms                         │ │
│  │  • Affiche dialogue de confirmation                     │ │
│  │  • Lance l'upload via le système existant               │ │
│  └────────────────────────────────────────────────────────┘ │
│                              │                               │
│                              ▼                               │
│  ┌────────────────────────────────────────────────────────┐ │
│  │           ClipboardUploadConfirm.vue                    │ │
│  │                                                          │ │
│  │  • Modal de confirmation                                │ │
│  │  • Liste des fichiers à uploader                        │ │
│  │  • Prévisualisation images                              │ │
│  │  • Boutons Confirmer / Annuler                          │ │
│  └────────────────────────────────────────────────────────┘ │
│                              │                               │
│                              ▼                               │
│  ┌────────────────────────────────────────────────────────┐ │
│  │         useRustUploadManager.ts (existant)              │ │
│  │                                                          │ │
│  │  • Queue management                                     │ │
│  │  • Progress tracking                                    │ │
│  │  • RustUploadProgress.vue (modal existante)             │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Tauri Backend                           │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  upload_from_bytes() - NOUVEAU                          │ │
│  │                                                          │ │
│  │  • Pour images/texte (données en mémoire)               │ │
│  │  • Écrit dans fichier temp puis upload                  │ │
│  │  • Réutilise la logique multipart si > 50MB             │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  read_clipboard_files() - NOUVEAU                       │ │
│  │                                                          │ │
│  │  • Lit les chemins de fichiers depuis clipboard OS      │ │
│  │  • Retourne liste de chemins pour upload standard       │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  upload_file() - EXISTANT                               │ │
│  │                                                          │ │
│  │  • Pour fichiers copiés depuis OS (via chemin)          │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## Plan d'implémentation

### Phase 1 : Backend Rust

#### 1.1 Nouveau command `upload_from_bytes`

**Fichier** : `src-tauri/src/commands.rs`

```rust
#[tauri::command]
pub async fn upload_from_bytes(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    data: Vec<u8>,
    content_type: String,
    state: State<'_, AppState>,
) -> Result<String, String>
```

**Logique** :
1. Générer un upload_id unique
2. Créer un fichier temporaire avec les données
3. Réutiliser `perform_upload()` existant
4. Nettoyer le fichier temporaire après upload
5. Retourner l'upload_id pour le tracking

#### 1.2 Nouveau command `read_clipboard_files`

**Fichier** : `src-tauri/src/commands.rs`

```rust
#[tauri::command]
pub async fn read_clipboard_files() -> Result<Vec<String>, String>
```

**Logique** :
1. Accéder au clipboard système via `arboard` ou API native
2. Extraire les chemins de fichiers (si présents)
3. Vérifier que les fichiers existent
4. Retourner la liste des chemins valides

#### 1.3 Dépendances Cargo

**Fichier** : `src-tauri/Cargo.toml`

Ajouter si nécessaire :
```toml
arboard = "3"  # Accès clipboard cross-platform
```

---

### Phase 2 : Service Tauri Frontend

#### 2.1 Nouveaux bindings IPC

**Fichier** : `src/services/tauri.ts`

```typescript
export async function uploadFromBytes(
  profileId: string,
  bucket: string,
  key: string,
  data: Uint8Array,
  contentType: string
): Promise<string>

export async function readClipboardFiles(): Promise<string[]>
```

---

### Phase 3 : Composable useClipboardUpload

#### 3.1 Créer le composable

**Fichier** : `src/composables/useClipboardUpload.ts`

```typescript
interface ClipboardItem {
  type: 'image' | 'text' | 'file'
  name: string
  data: Uint8Array | null  // Pour image/texte
  path: string | null       // Pour fichiers OS
  contentType: string
  size: number
  preview?: string          // Data URL pour prévisualisation image
}

interface UseClipboardUploadReturn {
  // État
  isProcessing: Ref<boolean>
  pendingItems: Ref<ClipboardItem[]>
  showConfirmDialog: Ref<boolean>

  // Actions
  handlePaste: (event: ClipboardEvent) => Promise<void>
  confirmUpload: () => Promise<void>
  cancelUpload: () => void

  // Lifecycle
  setupPasteListener: () => void
  cleanupPasteListener: () => void
}
```

**Logique du handlePaste** :
1. Vérifier si un input/textarea a le focus → ignorer
2. Vérifier si on est dans ObjectBrowser avec bucket actif → sinon ignorer
3. Lire le clipboard :
   - Images : `clipboardData.items` avec type `image/*`
   - Texte : `clipboardData.getData('text/plain')`
   - Fichiers OS : appeler `readClipboardFiles()` Rust
4. Générer les noms de fichiers avec timestamp
5. Vérifier les conflits de noms contre `appStore.objects`
6. Ajouter suffixe `(1)`, `(2)` si nécessaire
7. Afficher le dialogue de confirmation

**Génération des noms** :
```typescript
function generateFileName(type: 'image' | 'text', extension: string): string {
  const now = new Date()
  const timestamp = format(now, 'yyyy-MM-dd_HH-mm-ss')

  if (type === 'image') {
    return `screenshot_${timestamp}.${extension}`
  } else {
    return `pasted_text_${timestamp}.txt`
  }
}
```

**Gestion des conflits** :
```typescript
function resolveNameConflict(baseName: string, existingKeys: string[]): string {
  const prefix = appStore.currentPrefix
  let finalName = baseName
  let counter = 1

  while (existingKeys.includes(prefix + finalName)) {
    const ext = baseName.lastIndexOf('.')
    if (ext > 0) {
      finalName = `${baseName.slice(0, ext)} (${counter})${baseName.slice(ext)}`
    } else {
      finalName = `${baseName} (${counter})`
    }
    counter++
  }

  return finalName
}
```

---

### Phase 4 : Composant de confirmation

#### 4.1 Créer le dialogue

**Fichier** : `src/components/ClipboardUploadConfirm.vue`

**Structure** :
```vue
<template>
  <div v-if="show" class="modal-overlay">
    <div class="modal-content">
      <h3>📋 Coller et uploader</h3>

      <p class="destination">
        Destination : <code>{{ currentBucket }}/{{ currentPrefix || '/' }}</code>
      </p>

      <div class="items-list">
        <div v-for="item in items" :key="item.name" class="item">
          <!-- Prévisualisation image -->
          <img v-if="item.preview" :src="item.preview" class="preview" />

          <!-- Icône pour texte/fichier -->
          <span v-else class="icon">{{ getIcon(item.type) }}</span>

          <div class="item-info">
            <span class="name">{{ item.name }}</span>
            <span class="size">{{ formatSize(item.size) }}</span>
          </div>
        </div>
      </div>

      <div class="actions">
        <button @click="cancel" class="btn-secondary">Annuler</button>
        <button @click="confirm" class="btn-primary">
          Uploader {{ items.length }} fichier(s)
        </button>
      </div>
    </div>
  </div>
</template>
```

**Props** :
```typescript
interface Props {
  show: boolean
  items: ClipboardItem[]
  currentBucket: string
  currentPrefix: string
}

interface Emits {
  (e: 'confirm'): void
  (e: 'cancel'): void
}
```

---

### Phase 5 : Intégration dans ObjectBrowser

#### 5.1 Modifier ObjectBrowser.vue

**Imports** :
```typescript
import { useClipboardUpload } from '@/composables/useClipboardUpload'
import ClipboardUploadConfirm from '@/components/ClipboardUploadConfirm.vue'
```

**Setup** :
```typescript
const {
  isProcessing,
  pendingItems,
  showConfirmDialog,
  confirmUpload,
  cancelUpload,
  setupPasteListener,
  cleanupPasteListener,
} = useClipboardUpload()

onMounted(() => {
  setupPasteListener()
  // ... existing code
})

onUnmounted(() => {
  cleanupPasteListener()
  // ... existing code
})
```

**Template** :
```vue
<!-- Ajouter dans le template -->
<ClipboardUploadConfirm
  :show="showConfirmDialog"
  :items="pendingItems"
  :current-bucket="appStore.currentBucket"
  :current-prefix="appStore.currentPrefix"
  @confirm="confirmUpload"
  @cancel="cancelUpload"
/>
```

---

### Phase 6 : Traductions i18n

#### 6.1 Ajouter les clés

**Fichier** : `src/i18n/translations.ts`

```typescript
// Français
clipboard: {
  pasteUpload: 'Coller et uploader',
  destination: 'Destination',
  confirmUpload: 'Uploader {count} fichier(s)',
  cancel: 'Annuler',
  processing: 'Traitement du clipboard...',
  noContent: 'Aucun contenu à coller',
  uploadStarted: 'Upload démarré',
  screenshotPrefix: 'screenshot',
  pastedTextPrefix: 'pasted_text',
}

// English
clipboard: {
  pasteUpload: 'Paste and upload',
  destination: 'Destination',
  confirmUpload: 'Upload {count} file(s)',
  cancel: 'Cancel',
  processing: 'Processing clipboard...',
  noContent: 'No content to paste',
  uploadStarted: 'Upload started',
  screenshotPrefix: 'screenshot',
  pastedTextPrefix: 'pasted_text',
}
```

---

## Fichiers à créer/modifier

| Fichier | Action | Description |
|---------|--------|-------------|
| `src-tauri/src/commands.rs` | Modifier | Ajouter `upload_from_bytes`, `read_clipboard_files` |
| `src-tauri/Cargo.toml` | Modifier | Ajouter dépendance `arboard` si nécessaire |
| `src/services/tauri.ts` | Modifier | Ajouter bindings IPC |
| `src/composables/useClipboardUpload.ts` | Créer | Logique principale clipboard |
| `src/components/ClipboardUploadConfirm.vue` | Créer | Modal de confirmation |
| `src/components/ObjectBrowser.vue` | Modifier | Intégrer le composable |
| `src/i18n/translations.ts` | Modifier | Ajouter traductions |
| `src/types/index.ts` | Modifier | Ajouter types ClipboardItem |

---

## Cas de test

### Tests manuels

1. **Screenshot** : Faire une capture d'écran → CTRL+V dans ObjectBrowser → Confirmer → Vérifier upload
2. **Texte** : Copier du texte → CTRL+V → Confirmer → Vérifier fichier .txt créé
3. **Fichier unique** : Copier un fichier depuis Finder/Explorer → CTRL+V → Confirmer
4. **Fichiers multiples** : Copier plusieurs fichiers → CTRL+V → Confirmer tous
5. **Conflit de nom** : Uploader 2 screenshots à la même seconde → Vérifier suffixe (1)
6. **Input focus** : Focus sur un champ texte → CTRL+V → Vérifier que ça ne déclenche PAS l'upload
7. **Annulation** : CTRL+V → Cliquer Annuler → Vérifier qu'aucun upload n'est lancé
8. **Hors ObjectBrowser** : Sur l'écran des profils → CTRL+V → Vérifier que rien ne se passe

### Tests unitaires

```typescript
// useClipboardUpload.test.ts
describe('useClipboardUpload', () => {
  it('should ignore paste when input is focused')
  it('should ignore paste when no bucket is selected')
  it('should generate correct screenshot filename')
  it('should generate correct text filename')
  it('should resolve name conflicts with suffix')
  it('should handle multiple files')
})
```

---

## Estimation

| Phase | Temps estimé |
|-------|--------------|
| Phase 1 : Backend Rust | 45 min |
| Phase 2 : Service Tauri | 15 min |
| Phase 3 : Composable | 1h |
| Phase 4 : Composant confirmation | 30 min |
| Phase 5 : Intégration ObjectBrowser | 20 min |
| Phase 6 : Traductions | 10 min |
| Tests & Debug | 30 min |
| **Total** | **~3h30** |

---

## Risques et mitigations

| Risque | Probabilité | Mitigation |
|--------|-------------|------------|
| API Clipboard non supportée (vieux navigateur) | Faible | Tauri utilise Chromium récent |
| Permissions clipboard refusées | Faible | Vérifier permissions au démarrage |
| Fichiers OS non accessibles (permissions) | Moyenne | Afficher erreur claire |
| Performance avec gros fichiers | Faible | Réutilise multipart existant |

---

## Prêt pour implémentation ✅

Toutes les spécifications sont définies. L'implémentation peut commencer.
