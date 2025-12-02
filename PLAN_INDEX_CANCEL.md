# Plan d'Action : Bouton Stop pour l'Indexation

## Objectif

Ajouter un bouton "Stop" dans la modale de progression d'indexation (`IndexProgress.vue`) permettant à l'utilisateur d'arrêter l'indexation en cours tout en conservant l'index partiel déjà créé.

---

## 1. Analyse de l'Existant

### Architecture Actuelle

| Fichier | Rôle | Lignes clés |
|---------|------|-------------|
| `src-tauri/src/index_manager.rs` | Logique d'indexation | L50-259 : `initial_index_bucket()` |
| `src-tauri/src/commands.rs` | Commandes Tauri IPC | L1395-1528 : `start_initial_index` |
| `src-tauri/src/models.rs` | Structures de données | L863-882 : `IndexStatus`, `IndexProgressEvent` |
| `src/composables/useIndexManager.ts` | Composable Vue | État partagé, event listener |
| `src/components/IndexProgress.vue` | UI modale | Affichage progression |
| `src/services/tauri.ts` | Wrapper IPC | Fonctions invoke |

### Pattern de Cancellation Existant (Uploads/Downloads)

```rust
// commands.rs:23-26
pub struct UploadTask {
    handle: JoinHandle<()>,
    cancel_tx: broadcast::Sender<()>,  // Signal de cancellation
    file_name: String,
    file_size: u64,
}

// commands.rs:39-43
pub struct AppState {
    pub profiles: Mutex<ProfileStore>,
    pub active_uploads: Arc<Mutex<HashMap<String, UploadTask>>>,
    pub active_downloads: Arc<Mutex<HashMap<String, DownloadTask>>>,
}
```

### Flow Actuel d'Indexation

```
startIndexing() → start_initial_index command
       ↓
Loop (L101-157):
  - list_objects() sans delimiter
  - upsert_objects_batch()
  - emit index:progress
  - check max_requests ou is_truncated
       ↓
Emit Completed/Partial/Failed
```

---

## 2. Étude d'Impact

### Risques et Mitigations

| Composant | Impact | Risque | Mitigation |
|-----------|--------|--------|------------|
| **SQLite** | Objets insérés par batch | Faible | Les transactions sont atomiques, données cohérentes |
| **prefix_status** | Doit garder continuation_token | Moyen | Sauvegarder avant de quitter la boucle |
| **bucket_info** | `initial_index_completed = false` | Faible | Déjà géré pour les index partiels |
| **UI** | Nouveau statut "Cancelled" | Faible | Distinction visuelle claire |
| **Mémoire** | Ressources à libérer | Faible | Pattern existant (uploads) |
| **Reprise** | Possibilité de continuer | Moyen | continuation_token préservé |

### Ce qui est Préservé en Cas d'Arrêt

| Donnée | Préservé | Mécanisme |
|--------|----------|-----------|
| Objets indexés | ✅ Oui | Déjà commités en SQLite via `upsert_objects_batch` |
| Taille totale | ✅ Oui | Calculée depuis les objets existants |
| Continuation token | ✅ Oui | Sauvegardé dans `prefix_status` avant arrêt |
| Stats par prefix | ⚠️ Partiel | Recalculées à la prochaine navigation |
| bucket_info | ✅ Oui | `initial_index_completed = false` |

---

## 3. Modifications Requises

### A. Backend Rust

#### 3.1 Nouveau Statut - `models.rs`

**Fichier**: `src-tauri/src/models.rs` (ligne ~863)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexStatus {
    Starting,
    Indexing,
    Completed,
    Partial,
    Failed,
    Cancelled,  // ← NOUVEAU
}
```

#### 3.2 Structure IndexTask - `commands.rs`

**Fichier**: `src-tauri/src/commands.rs` (après ligne 36)

```rust
// Indexing task handle with metadata for cancellation
pub struct IndexTask {
    handle: JoinHandle<Result<InitialIndexResult, String>>,
    cancel_tx: broadcast::Sender<()>,
    bucket_name: String,
    started_at: std::time::Instant,
}
```

#### 3.3 Mise à jour AppState - `commands.rs`

**Fichier**: `src-tauri/src/commands.rs` (ligne ~39-53)

```rust
pub struct AppState {
    pub profiles: Mutex<ProfileStore>,
    pub active_uploads: Arc<Mutex<HashMap<String, UploadTask>>>,
    pub active_downloads: Arc<Mutex<HashMap<String, DownloadTask>>>,
    pub active_indexing: Arc<Mutex<HashMap<String, IndexTask>>>,  // ← NOUVEAU
}

impl AppState {
    pub fn new() -> Self {
        let profiles = ProfileStore::load().unwrap_or_default();
        Self {
            profiles: Mutex::new(profiles),
            active_uploads: Arc::new(Mutex::new(HashMap::new())),
            active_downloads: Arc::new(Mutex::new(HashMap::new())),
            active_indexing: Arc::new(Mutex::new(HashMap::new())),  // ← NOUVEAU
        }
    }
}
```

#### 3.4 Modification initial_index_bucket - `index_manager.rs`

**Fichier**: `src-tauri/src/index_manager.rs` (ligne ~50)

Ajouter un paramètre `cancel_rx` à la signature :

```rust
pub async fn initial_index_bucket<F>(
    &self,
    adapter: &S3Adapter,
    bucket_name: &str,
    config: &IndexingConfig,
    mut on_progress: F,
    mut cancel_rx: Option<broadcast::Receiver<()>>,  // ← NOUVEAU
) -> Result<InitialIndexResult, AppError>
where
    F: FnMut(u64, u32, u32),
{
    // ... code existant ...

    // Dans la boucle (ligne ~101), ajouter la vérification :
    loop {
        // Vérifier si annulation demandée
        if let Some(ref mut rx) = cancel_rx {
            if rx.try_recv().is_ok() {
                // Sauvegarder l'état actuel avant de sortir
                root_status.continuation_token = continuation_token.clone();
                root_status.last_indexed_key = last_key.clone();
                root_status.objects_count = total_indexed as i64;
                root_status.total_size = total_size;
                self.db.upsert_prefix_status(&root_status)?;

                // Mettre à jour bucket_info
                bucket_info.initial_index_requests = requests_made as i32;
                bucket_info.initial_index_completed = false;
                self.db.upsert_bucket_info(&bucket_info)?;

                return Ok(InitialIndexResult {
                    total_indexed,
                    is_complete: false,
                    requests_made,
                    continuation_token,
                    last_key,
                    total_size,
                    error: Some("Cancelled by user".to_string()),
                });
            }
        }

        // ... reste de la boucle existante ...
    }
}
```

#### 3.5 Nouvelle Commande cancel_indexing - `commands.rs`

**Fichier**: `src-tauri/src/commands.rs` (après `start_initial_index`)

```rust
/// Cancel an active indexing operation
#[tauri::command]
pub async fn cancel_indexing(
    app: AppHandle,
    profile_id: String,
    bucket_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let key = format!("{}-{}", profile_id, bucket_name);

    let task = {
        let mut indexing = state.active_indexing.lock().map_err(|e| e.to_string())?;
        indexing.remove(&key)
    };

    if let Some(task) = task {
        // Émettre l'événement cancelled
        let _ = app.emit_all(
            "index:progress",
            IndexProgressEvent {
                profile_id: profile_id.clone(),
                bucket_name: bucket_name.clone(),
                objects_indexed: 0,  // Sera mis à jour par la tâche
                requests_made: 0,
                max_requests: 0,
                is_complete: false,
                status: IndexStatus::Cancelled,
                error: Some("Indexing cancelled by user".to_string()),
            },
        );

        // Envoyer le signal de cancellation
        let _ = task.cancel_tx.send(());

        // Attendre un peu pour laisser la tâche se terminer proprement
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Abort si toujours en cours
        task.handle.abort();

        Ok(())
    } else {
        Err("No active indexing found for this bucket".to_string())
    }
}
```

#### 3.6 Modification start_initial_index - `commands.rs`

**Fichier**: `src-tauri/src/commands.rs` (ligne ~1395)

Modifier pour spawner une tâche et enregistrer dans `active_indexing` :

```rust
#[tauri::command]
pub async fn start_initial_index(
    app: AppHandle,
    profile_id: String,
    bucket_name: String,
    max_requests: Option<u32>,
    batch_size: Option<i32>,
    state: State<'_, AppState>,
) -> Result<InitialIndexResult, String> {
    let key = format!("{}-{}", profile_id, bucket_name);

    // Vérifier si une indexation est déjà en cours
    {
        let indexing = state.active_indexing.lock().map_err(|e| e.to_string())?;
        if indexing.contains_key(&key) {
            return Err("Indexing already in progress for this bucket".to_string());
        }
    }

    // Créer le channel de cancellation
    let (cancel_tx, cancel_rx) = broadcast::channel::<()>(1);

    // ... code existant pour récupérer profile, créer adapter, config ...

    // Cloner pour la tâche
    let app_clone = app.clone();
    let profile_id_clone = profile_id.clone();
    let bucket_name_clone = bucket_name.clone();
    let state_indexing = state.active_indexing.clone();
    let key_clone = key.clone();

    // Spawner la tâche d'indexation
    let handle = tokio::spawn(async move {
        let result = index_mgr
            .initial_index_bucket(
                &adapter,
                &bucket_name_clone,
                &config,
                |objects_indexed, requests_made, max_requests| {
                    // Callback de progression (existant)
                },
                Some(cancel_rx),  // ← Passer le receiver
            )
            .await;

        // Retirer de active_indexing à la fin
        if let Ok(mut indexing) = state_indexing.lock() {
            indexing.remove(&key_clone);
        }

        // Émettre l'événement final
        // ... (code existant pour Completed/Partial/Failed)

        result.map_err(|e| e.to_string())
    });

    // Enregistrer la tâche
    {
        let mut indexing = state.active_indexing.lock().map_err(|e| e.to_string())?;
        indexing.insert(
            key,
            IndexTask {
                handle,
                cancel_tx,
                bucket_name: bucket_name.clone(),
                started_at: std::time::Instant::now(),
            },
        );
    }

    // Attendre le résultat (ou utiliser un pattern async différent)
    // Note: Cette partie nécessite une réflexion sur le design
    // Option A: Retourner immédiatement un "job_id" et écouter les events
    // Option B: Await comme actuellement mais avec gestion cancel
}
```

#### 3.7 Enregistrer la commande - `main.rs`

**Fichier**: `src-tauri/src/main.rs` (ligne ~60)

```rust
// Index management commands
start_initial_index,
cancel_indexing,  // ← NOUVEAU
get_bucket_index_stats,
// ...
```

---

### B. Frontend TypeScript/Vue

#### 3.8 Service Tauri - `tauri.ts`

**Fichier**: `src/services/tauri.ts`

```typescript
// Index Management - Cancel
export async function cancelIndexing(
  profileId: string,
  bucketName: string
): Promise<void> {
  return await invoke('cancel_indexing', { profileId, bucketName })
}
```

#### 3.9 Composable useIndexManager - `useIndexManager.ts`

**Fichier**: `src/composables/useIndexManager.ts`

Ajouter dans l'event listener (ligne ~47) :

```typescript
} else if (event.payload.status === 'cancelled') {
  indexingBuckets.value[key] = false
  // Garder le dernier état pour afficher "X objets indexés"
}
```

Ajouter la méthode :

```typescript
/**
 * Cancel an active indexing operation
 * @param profileId Profile ID
 * @param bucketName Bucket name
 */
async function cancelIndexing(
  profileId: string,
  bucketName: string
): Promise<boolean> {
  try {
    await cancelIndexingService(profileId, bucketName)
    return true
  } catch (error) {
    logger.error('Failed to cancel indexing', error)
    return false
  }
}
```

Exporter dans le return :

```typescript
return {
  // Methods
  startIndexing,
  startFullIndexing,
  cancelIndexing,  // ← NOUVEAU
  // ...
}
```

#### 3.10 Types - `types/index.ts`

**Fichier**: `src/types/index.ts`

Mettre à jour `IndexProgressEvent.status` :

```typescript
export interface IndexProgressEvent {
  profile_id: string
  bucket_name: string
  objects_indexed: number
  requests_made: number
  max_requests: number
  is_complete: boolean
  status: 'starting' | 'indexing' | 'completed' | 'partial' | 'failed' | 'cancelled'
  error: string | null
}
```

#### 3.11 UI IndexProgress.vue

**Fichier**: `src/components/IndexProgress.vue`

```vue
<template>
  <Transition name="slide-up">
    <div
      v-if="hasActiveIndexing"
      class="fixed bottom-4 right-4 bg-background border border-border rounded-lg shadow-lg p-3 min-w-[280px] max-w-[350px] z-50"
    >
      <div class="flex items-center justify-between gap-2 mb-2">
        <div class="flex items-center gap-2">
          <div class="animate-spin text-blue-400">
            <!-- spinner icon -->
          </div>
          <span class="text-sm font-medium">{{ t('indexing') }}</span>
        </div>
      </div>

      <div v-for="(progress, key) in activeProgress" :key="key" class="mb-2 last:mb-0">
        <div class="flex items-center justify-between mb-1">
          <div class="text-xs text-muted-foreground truncate flex-1" :title="progress.bucket_name">
            {{ progress.bucket_name }}
          </div>
          <!-- NOUVEAU: Bouton Stop -->
          <button
            @click="handleCancel(progress.profile_id, progress.bucket_name)"
            class="ml-2 p-1 text-muted-foreground hover:text-red-500 hover:bg-red-500/10 rounded transition-colors"
            :title="t('cancelIndex')"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
            </svg>
          </button>
        </div>

        <!-- Progress bar existante -->
        <div class="flex items-center gap-2">
          <div class="flex-1 h-1.5 bg-muted rounded-full overflow-hidden">
            <!-- ... code existant ... -->
          </div>
          <span class="text-xs text-muted-foreground min-w-[60px] text-right">
            {{ formatNumber(progress.objects_indexed) }}
          </span>
        </div>
        <!-- ... reste du code existant ... -->
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { getIndexManager } from '../composables/useIndexManager'
import { useI18n } from '../composables/useI18n'
import type { IndexProgressEvent } from '../types'

const { indexProgress, indexingBuckets, cancelIndexing } = getIndexManager()
const { t } = useI18n()

// ... computed existants ...

async function handleCancel(profileId: string, bucketName: string) {
  await cancelIndexing(profileId, bucketName)
}
</script>
```

#### 3.12 Traductions - `translations.ts`

**Fichier**: `src/i18n/translations.ts`

Ajouter pour chaque langue :

```typescript
// English (ligne ~256)
cancelIndex: 'Stop indexing',
indexCancelled: 'Indexing stopped',
indexCancelledDesc: 'Indexing was stopped. {count} objects have been indexed.',

// French (ligne ~660)
cancelIndex: 'Arrêter l\'indexation',
indexCancelled: 'Indexation arrêtée',
indexCancelledDesc: 'L\'indexation a été arrêtée. {count} objets ont été indexés.',
```

---

## 4. Ordre d'Implémentation

### Phase 1 : Backend Rust (estimé: 2-3h)

1. [ ] Ajouter `Cancelled` à `IndexStatus` dans `models.rs`
2. [ ] Créer `IndexTask` struct dans `commands.rs`
3. [ ] Ajouter `active_indexing` à `AppState`
4. [ ] Modifier `initial_index_bucket()` pour accepter `cancel_rx`
5. [ ] Implémenter la vérification du signal dans la boucle
6. [ ] Créer la commande `cancel_indexing`
7. [ ] Modifier `start_initial_index` pour gérer la tâche async
8. [ ] Enregistrer `cancel_indexing` dans `main.rs`

### Phase 2 : Frontend TypeScript (estimé: 1-2h)

9. [ ] Ajouter `cancelIndexing` dans `tauri.ts`
10. [ ] Mettre à jour le type `IndexProgressEvent`
11. [ ] Ajouter `cancelIndexing` dans `useIndexManager.ts`
12. [ ] Gérer le statut `cancelled` dans l'event listener

### Phase 3 : UI Vue (estimé: 1h)

13. [ ] Ajouter le bouton stop dans `IndexProgress.vue`
14. [ ] Ajouter les traductions
15. [ ] Tests manuels

### Phase 4 : Tests et Polish (estimé: 1h)

16. [ ] Test: arrêt pendant indexation limitée
17. [ ] Test: arrêt pendant indexation full
18. [ ] Test: reprise après arrêt
19. [ ] Test: UI feedback correct

---

## 5. Points d'Attention

### 5.1 Race Conditions

- S'assurer que le signal cancel arrive **avant** la fin naturelle de la boucle
- Utiliser `try_recv()` (non-bloquant) plutôt que `recv().await`

### 5.2 Nettoyage des Ressources

- Toujours retirer de `active_indexing` même en cas d'erreur
- Utiliser `defer` pattern ou `Drop` trait si nécessaire

### 5.3 Design de `start_initial_index`

**Option A** (recommandée): Fire-and-forget
- `start_initial_index` retourne immédiatement un `job_id`
- L'UI écoute les events `index:progress`
- Plus simple, plus réactif

**Option B**: Await avec cancel
- `start_initial_index` attend la fin mais peut être interrompu
- Plus complexe à implémenter proprement
- Peut bloquer d'autres opérations

### 5.4 Reprise d'Indexation

L'implémentation actuelle permet déjà de "reprendre" :
- Le `continuation_token` est sauvegardé
- Un nouvel appel à `startIndexing` continuera là où on s'est arrêté
- Pas besoin de fonctionnalité explicite de "Resume"

---

## 6. Simulation du Flow Final

```
┌─────────────────────────────────────────────────────────────────┐
│                         FLOW AVEC CANCEL                         │
└─────────────────────────────────────────────────────────────────┘

User click "Index"
    ↓
startIndexing() → start_initial_index command
    ↓
Spawn async task + register in active_indexing
    ↓
Loop:                                     User click "Stop"
  ├─ check cancel_rx.try_recv()  ←───────── cancelIndexing()
  │      ↓ (if signal received)              ↓
  │  Save continuation_token            send cancel signal
  │  Save current stats                 emit Cancelled event
  │  Break loop
  │      ↓
  ├─ list_objects()
  ├─ upsert_objects_batch()
  └─ emit progress
    ↓
Remove from active_indexing
    ↓
Emit final status (Completed/Partial/Cancelled/Failed)
    ↓
UI shows result:
  - "Indexation terminée" (complete)
  - "Index partiel" (partial - limit reached)
  - "Indexation arrêtée - X objets indexés" (cancelled)
```

---

## 7. Validation Finale

- [ ] Le bouton stop apparaît dans la modale
- [ ] Cliquer sur stop arrête l'indexation dans les 500ms
- [ ] L'index partiel est conservé (vérifier en SQLite)
- [ ] Le `continuation_token` est sauvegardé
- [ ] L'UI affiche "Indexation arrêtée"
- [ ] Relancer l'indexation continue là où on s'est arrêté
- [ ] Pas de régression sur le flow normal
- [ ] Pas de fuite mémoire (vérifier avec `htop`)

---

## 8. Rollback Plan

En cas de problème :
1. Retirer `cancel_indexing` de `main.rs`
2. Commenter le bouton dans `IndexProgress.vue`
3. Le code backend reste compatible (paramètre `cancel_rx` optionnel)
