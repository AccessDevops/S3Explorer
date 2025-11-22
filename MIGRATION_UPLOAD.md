# Migration Upload - From JS to Rust

## Résumé des changements

### Architecture AVANT (JavaScript-based)
- Multipart géré côté JavaScript
- Lecture fichiers en mémoire JS
- Gestion concurrence complexe côté JS
- Événements manuels entre composants

### Architecture APRÈS (Rust-based)
- **TOUT géré côté Rust** avec Tokio tasks non-bloquantes
- Lecture fichiers directement par Rust (efficace mémoire)
- Gestion multipart S3 automatique (>50MB)
- Événements de progression via `app.emit()`
- Frontend minimal : juste écoute les événements

---

## Modifications à apporter dans ObjectBrowser.vue

### 1. Importsà modifier

**AVANT:**
```typescript
import { useUploadManager } from '../composables/useUploadManager'
import {
  uploadLargeFile,
  uploadLargeFileFromPath,
  shouldUseMultipartUpload,
  getConcurrencyForMultipleFiles,
} from '../utils/multipartUpload'
import { putObject } from '../services/tauri'
import { readBinaryFile } from '@tauri-apps/api/fs'
```

**APRÈS:**
```typescript
import { useRustUploadManager } from '../composables/useRustUploadManager'
// Supprimez tous les imports multipartUpload
```

### 2. Composable à remplacer

**AVANT:**
```typescript
const uploadManager = useUploadManager()
```

**APRÈS:**
```typescript
const rustUploadManager = useRustUploadManager()
```

### 3. Fonction handleFileDrop (Drag & Drop)

**AVANT:** ~100 lignes complexes avec gestion multipart, concurrence, etc.

**APRÈS:** Version simplifiée
```typescript
async function handleFileDrop(paths: string[]) {
  if (!appStore.currentProfile || !appStore.currentBucket) {
    logger.error('No profile or bucket selected')
    return
  }

  logger.debug(`Starting upload of ${paths.length} file(s) via drag & drop...`)

  // Helper to detect content type from extension
  const getContentType = (fileName: string): string | undefined => {
    const ext = fileName.split('.').pop()?.toLowerCase()
    if (!ext) return undefined

    const contentTypes: Record<string, string> = {
      jpg: 'image/jpeg',
      jpeg: 'image/jpeg',
      png: 'image/png',
      gif: 'image/gif',
      pdf: 'application/pdf',
      txt: 'text/plain',
      json: 'application/json',
      xml: 'application/xml',
      zip: 'application/zip',
      mp4: 'video/mp4',
    }

    return contentTypes[ext]
  }

  // Upload each file using Rust command
  for (const filePath of paths) {
    const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'file'
    const key = appStore.currentPrefix + fileName
    const contentType = getContentType(fileName)

    try {
      // Start upload (non-blocking, returns upload_id immediately)
      await rustUploadManager.startUpload(
        appStore.currentProfile.id,
        appStore.currentBucket,
        key,
        filePath,
        contentType
      )
      logger.debug(`✓ Started upload: ${fileName}`)
    } catch (e) {
      logger.error(`✗ Failed to start upload ${fileName}:`, e)
      toast.error(`Failed to upload ${fileName}: ${e}`)
    }
  }

  // Reload objects after initiating all uploads
  // Note: actual completion happens asynchronously via events
  await appStore.loadObjects()
  logger.debug(`Drag & drop upload initiated for ${paths.length} file(s)`)
}
```

### 4. Fonction uploadFilesHandler (Bouton Upload)

**AVANT:** Utilise `<input type="file">` et lit les fichiers en mémoire

**APRÈS:** Utilise Tauri dialog pour obtenir les paths
```typescript
import { open } from '@tauri-apps/api/dialog'

async function uploadFilesHandler() {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  // Use Tauri dialog to select files
  const selected = await open({
    multiple: true,
    title: 'Select files to upload',
  })

  if (!selected) return // User cancelled

  const filePaths = Array.isArray(selected) ? selected : [selected]

  // Close modal
  showUploadModal.value = false

  // Helper to detect content type
  const getContentType = (fileName: string): string | undefined => {
    const ext = fileName.split('.').pop()?.toLowerCase()
    if (!ext) return undefined

    const contentTypes: Record<string, string> = {
      jpg: 'image/jpeg',
      jpeg: 'image/jpeg',
      png: 'image/png',
      gif: 'image/gif',
      pdf: 'application/pdf',
      txt: 'text/plain',
      json: 'application/json',
      xml: 'application/xml',
      zip: 'application/zip',
      mp4: 'video/mp4',
    }

    return contentTypes[ext]
  }

  // Upload each file using Rust command
  for (const filePath of filePaths) {
    const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'file'
    const key = appStore.currentPrefix + fileName
    const contentType = getContentType(fileName)

    try {
      await rustUploadManager.startUpload(
        appStore.currentProfile.id,
        appStore.currentBucket,
        key,
        filePath,
        contentType
      )
      logger.debug(`✓ Started upload: ${fileName}`)
    } catch (e) {
      logger.error(`✗ Failed to start upload ${fileName}:`, e)
      toast.error(`Failed to upload ${fileName}: ${e}`)
    }
  }

  // Reload objects
  await appStore.loadObjects()
}
```

### 5. Modal Upload à simplifier

**AVANT:**
```vue
<input type="file" multiple @change="handleFileSelect" class="mb-4" />
<div v-if="uploadFiles.length > 0" class="space-y-2 max-h-64 overflow-y-auto mb-4">
  <!-- Liste des fichiers sélectionnés -->
</div>
```

**APRÈS:** Pas besoin de modal ! Le dialog Tauri suffit.
Simplifiez le bouton :
```vue
<Button @click="uploadFilesHandler">{{ t('upload') }}</Button>
```

### 6. Références à supprimer

Supprimez ces variables devenues inutiles :
```typescript
const uploadFiles = ref<File[]>([])
const uploadProgress = ref({ ... })
```

Supprimez ces fonctions :
```typescript
function handleFileSelect(event: Event) { ... }
function removeFile(index: number) { ... }
```

---

## Modifications dans App.vue

Remplacez le composant :
```vue
<!-- AVANT -->
<UploadProgress />

<!-- APRÈS -->
<RustUploadProgress />
```

Mettez à jour l'import :
```typescript
// AVANT
import UploadProgress from './components/UploadProgress.vue'

// APRÈS
import RustUploadProgress from './components/RustUploadProgress.vue'
```

---

## Bénéfices de la migration

✅ **Performance** : Rust gère tout, pas de charge JS
✅ **Mémoire** : Pas de chargement fichiers en mémoire JS
✅ **Simplicité** : Frontend réduit de ~100 lignes à ~20 lignes
✅ **Concurrent** : Tokio gère la concurrence nativement
✅ **Robustesse** : Gestion erreurs côté Rust
✅ **Uniformité** : Même code pour drag & drop et bouton upload

---

## Commandes Rust disponibles

```typescript
// Démarrer un upload (retourne upload_id)
const uploadId = await uploadFile(profileId, bucket, key, filePath, contentType?)

// Annuler un upload
await cancelUpload(uploadId)
```

## Événements écoutés

Le composable `useRustUploadManager` écoute automatiquement :
```typescript
event: 'upload:progress'
payload: UploadProgressEvent {
  upload_id: string
  file_name: string
  file_size: number
  uploaded_bytes: number
  uploaded_parts: number
  total_parts: number
  percentage: number
  status: 'pending' | 'starting' | 'uploading' | 'completed' | 'failed' | 'cancelled'
  error?: string
}
```

---

## Prochaines étapes

1. ✅ Backend Rust créé avec événements
2. ✅ Composable useRustUploadManager créé
3. ✅ Composant RustUploadProgress créé
4. ⏳ Modifier ObjectBrowser.vue
5. ⏳ Modifier App.vue
6. ⏳ Tester drag & drop
7. ⏳ Tester bouton upload
8. ⏳ Supprimer ancien code (multipartUpload.ts, useUploadManager.ts, UploadProgress.vue)
