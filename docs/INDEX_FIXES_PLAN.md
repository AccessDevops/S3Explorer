# Plan d'Implémentation - Corrections du Système d'Indexation SQLite

## Vue d'Ensemble

Ce document détaille les corrections à apporter au système d'indexation SQLite pour S3Explorer.

**Priorités:**
- 🔴 Haute (bloquant/critique)
- 🟡 Moyenne (important)
- 🟢 Basse (amélioration)

---

## Phase 1: Paramètre `maxInitialIndexRequests` 🔴

### Objectif
Permettre à l'utilisateur de configurer le nombre maximum de requêtes pour l'indexation initiale.

### Fichiers à modifier

#### 1.1 `src/stores/settings.ts`

**Ajouter après ligne 24:**
```typescript
const maxInitialIndexRequests = ref(20) // requests - max requests for initial bucket indexing
```

**Ajouter dans `loadSettings()` après ligne 111:**
```typescript
const savedMaxInitialIndexRequests = localStorage.getItem('app-maxInitialIndexRequests')
if (savedMaxInitialIndexRequests) {
  const requests = parseInt(savedMaxInitialIndexRequests, 10)
  if (!isNaN(requests) && requests >= 1 && requests <= 100) {
    maxInitialIndexRequests.value = requests
  }
}
```

**Ajouter après `setIndexAutoBuildThreshold`:**
```typescript
// Save max initial index requests to localStorage
const setMaxInitialIndexRequests = (requests: number) => {
  if (isNaN(requests) || requests < 1 || requests > 100) {
    return // Silently ignore invalid values
  }
  maxInitialIndexRequests.value = requests
  localStorage.setItem('app-maxInitialIndexRequests', String(requests))
}
```

**Ajouter dans le return:**
```typescript
maxInitialIndexRequests,
setMaxInitialIndexRequests,
```

#### 1.2 `src/components/SettingsButton.vue`

**Ajouter dans la section Index Settings:**
```vue
<!-- Max Initial Index Requests -->
<div class="flex items-center justify-between">
  <div>
    <Label>{{ t('maxInitialIndexRequests') }}</Label>
    <p class="text-xs text-muted-foreground">
      {{ t('maxInitialIndexRequestsDesc') }}
    </p>
  </div>
  <Input
    type="number"
    :model-value="settingsStore.maxInitialIndexRequests"
    @update:model-value="(v) => settingsStore.setMaxInitialIndexRequests(Number(v))"
    class="w-20"
    min="1"
    max="100"
  />
</div>
```

#### 1.3 `src/i18n/translations.ts`

**Ajouter les traductions:**
```typescript
maxInitialIndexRequests: {
  en: 'Max Index Requests',
  fr: 'Requêtes max d\'indexation',
  // ... autres langues
},
maxInitialIndexRequestsDesc: {
  en: 'Maximum S3 requests for initial bucket indexing (1-100)',
  fr: 'Nombre maximum de requêtes S3 pour l\'indexation initiale (1-100)',
  // ... autres langues
},
```

#### 1.4 `src/composables/useIndexManager.ts`

**Modifier la fonction `startIndexing`:**
```typescript
async function startIndexing(
  profileId: string,
  bucketName: string,
  maxRequests?: number // Rendre optionnel, utiliser settings si non fourni
): Promise<InitialIndexResult | null> {
  const key = `${profileId}-${bucketName}`

  // Utiliser le paramètre ou les settings
  const effectiveMaxRequests = maxRequests ?? settingsStore.maxInitialIndexRequests

  try {
    indexingBuckets.value[key] = true
    const result = await startInitialIndex(profileId, bucketName, effectiveMaxRequests)
    // ...
  }
}
```

#### 1.5 `src/components/BucketList.vue`

**Modifier l'appel à `startIndexing`:**
```typescript
// Ligne 326 - utiliser les settings
indexManager.startIndexing(
  appStore.currentProfile.id,
  bucketName,
  settingsStore.maxInitialIndexRequests
)
```

---

## Phase 2: Propagation de l'incomplétude aux ancêtres 🔴

### Objectif
Quand un objet est ajouté/supprimé, marquer TOUS les préfixes ancêtres comme incomplets.

### Fichiers à modifier

#### 2.1 `src-tauri/src/database.rs`

**Ajouter une nouvelle fonction après `mark_prefix_incomplete`:**
```rust
/// Marquer un prefixe et tous ses ancêtres comme incomplets
pub fn mark_prefix_and_ancestors_incomplete(
    &self,
    bucket_name: &str,
    prefix: &str,
) -> Result<(), AppError> {
    let conn = self.get_connection()?;

    // Collecter tous les prefixes ancêtres
    let mut current = prefix.to_string();
    let mut prefixes_to_mark = vec![current.clone()];

    while let Some(pos) = current[..current.len().saturating_sub(1)].rfind('/') {
        current = current[..=pos].to_string();
        prefixes_to_mark.push(current.clone());
    }

    // Marquer aussi le prefix racine
    if !prefix.is_empty() {
        prefixes_to_mark.push(String::new());
    }

    // Marquer tous les prefixes comme incomplets
    for pfx in prefixes_to_mark {
        conn.execute(
            r#"
            UPDATE prefix_status
            SET is_complete = FALSE
            WHERE profile_id = ?1 AND bucket_name = ?2 AND prefix = ?3
            "#,
            params![self.profile_id, bucket_name, pfx],
        )?;
    }

    Ok(())
}
```

#### 2.2 `src-tauri/src/index_manager.rs`

**Modifier `add_object` (lignes 243-254):**
```rust
/// Ajouter un objet a l'index (apres put_object reussi)
pub fn add_object(&self, bucket_name: &str, obj: &S3Object) -> Result<(), AppError> {
    let indexed = IndexedObject::from_s3_object(obj, &self.profile_id, bucket_name);
    self.db.upsert_object(&indexed)?;

    // Marquer le prefixe parent ET tous les ancêtres comme incomplets
    let parent = &indexed.parent_prefix;
    if !parent.is_empty() {
        self.db.mark_prefix_and_ancestors_incomplete(bucket_name, parent)?;
    } else {
        // Objet à la racine - marquer le bucket comme incomplet
        self.db.mark_prefix_incomplete(bucket_name, "")?;
    }

    Ok(())
}
```

**Modifier `remove_object` (lignes 257-269):**
```rust
/// Supprimer un objet de l'index (apres delete_object reussi)
pub fn remove_object(&self, bucket_name: &str, key: &str) -> Result<(), AppError> {
    // Recuperer l'objet pour connaitre son parent_prefix
    if let Some(obj) = self.db.get_object(bucket_name, key)? {
        self.db.delete_object(bucket_name, key)?;

        // Marquer le prefixe parent ET tous les ancêtres comme incomplets
        if !obj.parent_prefix.is_empty() {
            self.db.mark_prefix_and_ancestors_incomplete(bucket_name, &obj.parent_prefix)?;
        } else {
            self.db.mark_prefix_incomplete(bucket_name, "")?;
        }
    }

    Ok(())
}
```

**Modifier `remove_folder` (lignes 272-283):**
```rust
/// Supprimer un dossier de l'index (apres delete_folder reussi)
pub fn remove_folder(&self, bucket_name: &str, prefix: &str) -> Result<i64, AppError> {
    let deleted = self.db.delete_objects_by_prefix(bucket_name, prefix)?;

    // Marquer le prefixe parent ET tous les ancêtres comme incomplets
    let parent = IndexedObject::extract_parent_prefix(prefix);
    self.db.mark_prefix_and_ancestors_incomplete(bucket_name, &parent)?;

    // Supprimer aussi le prefix_status du dossier supprimé
    self.db.delete_prefix_status(bucket_name, prefix)?;

    Ok(deleted)
}
```

#### 2.3 `src-tauri/src/database.rs`

**Ajouter la fonction `delete_prefix_status`:**
```rust
/// Supprimer le statut d'un prefixe
pub fn delete_prefix_status(&self, bucket_name: &str, prefix: &str) -> Result<(), AppError> {
    let conn = self.get_connection()?;

    conn.execute(
        "DELETE FROM prefix_status WHERE profile_id = ?1 AND bucket_name = ?2 AND prefix = ?3",
        params![self.profile_id, bucket_name, prefix],
    )?;

    Ok(())
}
```

---

## Phase 3: Vérification récursive de `is_complete` 🔴

### Objectif
Un préfixe n'est considéré complet que si TOUS ses sous-préfixes sont aussi complets.

### Fichiers à modifier

#### 3.1 `src-tauri/src/database.rs`

**Remplacer `is_prefix_complete` (lignes 556-572):**
```rust
/// Verifier si un prefixe est complet (incluant tous ses sous-prefixes)
pub fn is_prefix_complete(&self, bucket_name: &str, prefix: &str) -> Result<bool, AppError> {
    let conn = self.get_connection()?;

    // Vérifier d'abord si le prefix lui-même est marqué complet
    let self_complete: bool = conn
        .query_row(
            r#"
            SELECT COALESCE(is_complete, FALSE)
            FROM prefix_status
            WHERE profile_id = ?1 AND bucket_name = ?2 AND prefix = ?3
            "#,
            params![self.profile_id, bucket_name, prefix],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !self_complete {
        return Ok(false);
    }

    // Vérifier que tous les sous-prefixes sont aussi complets
    // Un sous-prefix incomplet rend le parent incomplet
    let has_incomplete_children: bool = conn
        .query_row(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM prefix_status
                WHERE profile_id = ?1
                  AND bucket_name = ?2
                  AND prefix LIKE ?3 || '%'
                  AND prefix != ?3
                  AND is_complete = FALSE
            )
            "#,
            params![self.profile_id, bucket_name, prefix],
            |row| row.get(0),
        )
        .unwrap_or(false);

    Ok(!has_incomplete_children)
}
```

#### 3.2 Ajouter une fonction pour vérification rapide (sans récursion)

```rust
/// Verifier si un prefixe est marqué complet (sans vérifier les enfants)
/// Utilisé pour l'affichage rapide
pub fn is_prefix_self_complete(&self, bucket_name: &str, prefix: &str) -> Result<bool, AppError> {
    let conn = self.get_connection()?;

    let result: bool = conn
        .query_row(
            r#"
            SELECT COALESCE(is_complete, FALSE)
            FROM prefix_status
            WHERE profile_id = ?1 AND bucket_name = ?2 AND prefix = ?3
            "#,
            params![self.profile_id, bucket_name, prefix],
            |row| row.get(0),
        )
        .unwrap_or(false);

    Ok(result)
}
```

---

## Phase 4: Création automatique des `prefix_status` parents 🟡

### Objectif
Quand on crée un `prefix_status` pour un préfixe, créer aussi les entrées pour tous les préfixes parents.

### Fichiers à modifier

#### 4.1 `src-tauri/src/database.rs`

**Modifier `upsert_prefix_status`:**
```rust
/// Mettre a jour ou creer le statut d'un prefixe (et ses parents)
pub fn upsert_prefix_status(&self, status: &PrefixStatus) -> Result<(), AppError> {
    let mut conn = self.get_connection()?;
    let tx = conn.transaction()?;

    // D'abord, créer les entrées pour tous les préfixes parents s'ils n'existent pas
    self.ensure_parent_prefixes_exist(&tx, &status.bucket_name, &status.prefix)?;

    // Ensuite, upsert le prefix actuel
    tx.execute(
        r#"
        INSERT INTO prefix_status (
            profile_id, bucket_name, prefix,
            is_complete, objects_count, total_size,
            continuation_token, last_indexed_key,
            last_sync_started_at, last_sync_completed_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        ON CONFLICT(profile_id, bucket_name, prefix)
        DO UPDATE SET
            is_complete = excluded.is_complete,
            objects_count = excluded.objects_count,
            total_size = excluded.total_size,
            continuation_token = excluded.continuation_token,
            last_indexed_key = excluded.last_indexed_key,
            last_sync_started_at = excluded.last_sync_started_at,
            last_sync_completed_at = excluded.last_sync_completed_at
        "#,
        params![
            status.profile_id,
            status.bucket_name,
            status.prefix,
            status.is_complete,
            status.objects_count,
            status.total_size,
            status.continuation_token,
            status.last_indexed_key,
            status.last_sync_started_at,
            status.last_sync_completed_at,
        ],
    )?;

    tx.commit()?;
    Ok(())
}

/// Créer les entrées prefix_status pour tous les préfixes parents
fn ensure_parent_prefixes_exist(
    &self,
    tx: &rusqlite::Transaction,
    bucket_name: &str,
    prefix: &str,
) -> Result<(), AppError> {
    let mut current = prefix.to_string();
    let now = chrono::Utc::now().timestamp_millis();

    while let Some(pos) = current[..current.len().saturating_sub(1)].rfind('/') {
        current = current[..=pos].to_string();

        // Insérer le parent s'il n'existe pas (avec is_complete = false)
        tx.execute(
            r#"
            INSERT OR IGNORE INTO prefix_status (
                profile_id, bucket_name, prefix,
                is_complete, objects_count, total_size,
                last_sync_started_at
            ) VALUES (?1, ?2, ?3, FALSE, 0, 0, ?4)
            "#,
            params![self.profile_id, bucket_name, current, now],
        )?;
    }

    // S'assurer que le prefix racine existe aussi
    tx.execute(
        r#"
        INSERT OR IGNORE INTO prefix_status (
            profile_id, bucket_name, prefix,
            is_complete, objects_count, total_size,
            last_sync_started_at
        ) VALUES (?1, ?2, '', FALSE, 0, 0, ?3)
        "#,
        params![self.profile_id, bucket_name, now],
    )?;

    Ok(())
}
```

---

## Phase 5: Mise à jour de l'index lors de `force_refresh` 🟡

### Objectif
Quand l'utilisateur force un refresh des stats, synchroniser l'index SQLite.

### Fichiers à modifier

#### 5.1 `src-tauri/src/commands.rs`

**Modifier `calculate_bucket_stats` (lignes 184-242):**
```rust
#[tauri::command]
pub async fn calculate_bucket_stats(
    app: AppHandle,
    profile_id: String,
    bucket_name: String,
    force_refresh: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(i64, i64, bool), String> {
    // Try index first unless force_refresh is requested
    if !force_refresh.unwrap_or(false) {
        if let Ok(index_mgr) = get_index_manager(&profile_id) {
            if let Ok(stats) = index_mgr.get_bucket_stats(&bucket_name) {
                return Ok((stats.total_size, stats.total_objects, !stats.is_complete));
            }
        }
    }

    // Fallback to S3 calculation
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let start_time = std::time::Instant::now();

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    // Utiliser une nouvelle fonction qui retourne aussi les objets
    let result = adapter.calculate_bucket_stats_with_objects(&bucket_name).await;

    let duration_ms = start_time.elapsed().as_millis() as u64;

    match &result {
        Ok((size, count, objects, request_count)) => {
            // Emit metrics...
            let avg_duration = duration_ms / (*request_count as u64).max(1);
            // ... (garder le code de métriques existant)

            // NOUVEAU: Mettre à jour l'index avec les objets récupérés
            if let Ok(index_mgr) = get_index_manager(&profile_id) {
                // Convertir et insérer les objets
                let indexed_objects: Vec<IndexedObject> = objects
                    .iter()
                    .map(|obj| IndexedObject::from_s3_object(obj, &profile_id, &bucket_name))
                    .collect();

                let _ = index_mgr.db.upsert_objects_batch(&indexed_objects);

                // Marquer le bucket comme complet si on a tout chargé
                let status = PrefixStatus {
                    profile_id: profile_id.clone(),
                    bucket_name: bucket_name.clone(),
                    prefix: String::new(),
                    is_complete: true, // force_refresh = données complètes
                    objects_count: *count,
                    total_size: *size,
                    continuation_token: None,
                    last_indexed_key: None,
                    last_sync_started_at: Some(chrono::Utc::now().timestamp_millis()),
                    last_sync_completed_at: Some(chrono::Utc::now().timestamp_millis()),
                    ..Default::default()
                };
                let _ = index_mgr.db.upsert_prefix_status(&status);
            }
        }
        Err(e) => {
            // ... (garder le code d'erreur existant)
        }
    }

    result.map(|(size, count, _, _)| (size, count, false)).map_err(|e| e.to_string())
}
```

#### 5.2 `src-tauri/src/s3_adapter.rs`

**Ajouter une nouvelle fonction:**
```rust
/// Calculate bucket stats AND return the objects (for index update)
pub async fn calculate_bucket_stats_with_objects(
    &self,
    bucket_name: &str,
) -> Result<(i64, i64, Vec<S3Object>, u32), S3Error> {
    let mut total_size: i64 = 0;
    let mut total_count: i64 = 0;
    let mut all_objects: Vec<S3Object> = Vec::new();
    let mut continuation_token: Option<String> = None;
    let mut request_count: u32 = 0;

    loop {
        let response = self
            .list_objects(bucket_name, None, continuation_token, Some(1000), false)
            .await?;

        request_count += 1;

        for obj in &response.objects {
            total_size += obj.size;
            total_count += 1;
            all_objects.push(obj.clone());
        }

        if !response.is_truncated {
            break;
        }

        continuation_token = response.continuation_token;
    }

    Ok((total_size, total_count, all_objects, request_count))
}
```

---

## Phase 6: Recalcul des stats après Load More/All 🟡

### Objectif
Mettre à jour l'affichage des stats du bucket après avoir chargé plus d'objets.

### Fichiers à modifier

#### 6.1 `src/components/ObjectBrowser.vue`

**Modifier `loadAllObjects` (lignes 3306-3337):**
```typescript
async function loadAllObjects() {
  if (!appStore.currentProfile || !appStore.currentBucket) return
  if (!appStore.continuationToken) return

  const toastId = toast.loading(t('loadingAllObjects'))
  let totalLoaded = appStore.objects.length

  try {
    while (appStore.continuationToken) {
      await appStore.loadObjects(true)
      totalLoaded = appStore.objects.length

      toast.updateToast(toastId, {
        message: `${t('loaded')} ${totalLoaded} ${t('objects')}...`,
      })
    }

    // NOUVEAU: Invalider le cache des stats pour forcer un recalcul
    bucketStatsComposable.invalidateStats(
      appStore.currentProfile.id,
      appStore.currentBucket
    )

    // Recharger les stats depuis l'index (maintenant à jour)
    await loadBucketStatsFromIndex()

    toast.completeToast(
      toastId,
      `${t('loadedAllObjects')}: ${totalLoaded} ${t('objects')}`,
      'success',
      3000
    )
  } catch (e) {
    toast.completeToast(toastId, `${t('errorLoadingObjects')}: ${e}`, 'error', 5000)
    logger.error('Failed to load all objects:', e)
  }
}

// Nouvelle fonction helper
async function loadBucketStatsFromIndex() {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  const stats = await indexManager.getIndexStats(
    appStore.currentProfile.id,
    appStore.currentBucket
  )

  if (stats) {
    // Mettre à jour le cache local
    bucketStatsComposable.updateCachedStats(
      appStore.currentProfile.id,
      appStore.currentBucket,
      {
        size: stats.total_size,
        count: stats.total_objects,
      }
    )
  }
}
```

#### 6.2 Idem pour Load More

**Ajouter à la fin de `loadMoreObjects` (si cette fonction existe) ou dans le handler du bouton Load More:**
```typescript
// Après avoir chargé plus d'objets
if (appStore.currentProfile && appStore.currentBucket) {
  // Invalider le cache pour forcer la mise à jour
  bucketStatsComposable.invalidateStats(
    appStore.currentProfile.id,
    appStore.currentBucket
  )
}
```

---

## Phase 7: Affichage distinct pour les nouveaux dossiers 🟢

### Objectif
Afficher `-` pour les dossiers qui n'ont jamais été parcourus, et la taille (même estimée) pour ceux qui ont des données dans l'index.

### Fichiers à modifier

#### 7.1 `src-tauri/src/commands.rs`

**Ajouter une nouvelle commande:**
```rust
/// Check if a prefix has ever been indexed (has any objects in the index)
#[tauri::command]
pub async fn is_prefix_known(
    profile_id: String,
    bucket_name: String,
    prefix: String,
) -> Result<bool, String> {
    let index_mgr = get_index_manager(&profile_id).map_err(|e| e.to_string())?;

    // Vérifier si on a au moins un objet avec ce prefix
    let (count, _) = index_mgr.db
        .calculate_prefix_stats(&bucket_name, &prefix)
        .map_err(|e| e.to_string())?;

    Ok(count > 0)
}
```

#### 7.2 `src/services/tauri.ts`

**Ajouter l'export:**
```typescript
export async function isPrefixKnown(
  profileId: string,
  bucketName: string,
  prefix: string
): Promise<boolean> {
  return await invoke('is_prefix_known', {
    profileId,
    bucketName,
    prefix,
  })
}
```

#### 7.3 `src/components/ObjectBrowser.vue`

**Modifier `getFolderSize`:**
```typescript
const unknownFolders = ref<Set<string>>(new Set())

async function getFolderSize(folder: string): Promise<string> {
  if (loadingFolderSizes.value.has(folder)) {
    return t('calculating')
  }

  // Si on sait que le dossier est inconnu
  if (unknownFolders.value.has(folder)) {
    return '-'
  }

  const size = folderSizes.value.get(folder)
  if (size === undefined) {
    // Vérifier si le dossier est connu dans l'index
    if (appStore.currentProfile && appStore.currentBucket) {
      const known = await isPrefixKnown(
        appStore.currentProfile.id,
        appStore.currentBucket,
        folder
      )
      if (!known) {
        unknownFolders.value.add(folder)
        return '-'
      }
    }
    return '-'
  }
  return formatSize(size)
}
```

---

## Phase 8: Batch size configurable pour l'indexation 🟢

### Objectif
Permettre de configurer le batch size utilisé lors de l'indexation (séparé du batch size de navigation).

### Fichiers à modifier

#### 8.1 `src/stores/settings.ts`

**Ajouter:**
```typescript
const indexBatchSize = ref(1000) // batch size for indexing (always max for performance)

// Dans loadSettings():
const savedIndexBatchSize = localStorage.getItem('app-indexBatchSize')
if (savedIndexBatchSize) {
  const size = parseInt(savedIndexBatchSize, 10)
  if (!isNaN(size) && size >= 100 && size <= 1000) {
    indexBatchSize.value = size
  }
}

// Setter:
const setIndexBatchSize = (size: number) => {
  if (isNaN(size) || size < 100 || size > 1000) return
  indexBatchSize.value = size
  localStorage.setItem('app-indexBatchSize', String(size))
}
```

#### 8.2 `src-tauri/src/commands.rs`

**Modifier `start_initial_index`:**
```rust
#[tauri::command]
pub async fn start_initial_index(
    app: AppHandle,
    profile_id: String,
    bucket_name: String,
    max_requests: Option<u32>,
    batch_size: Option<i32>,  // NOUVEAU paramètre
    state: State<'_, AppState>,
) -> Result<InitialIndexResult, String> {
    // ...
    let config = IndexingConfig {
        max_initial_requests: max_requests.unwrap_or(20),
        batch_size: batch_size.unwrap_or(1000),  // Utiliser le paramètre
        stale_ttl_hours: 24,
    };
    // ...
}
```

---

## Ordre d'Implémentation Recommandé

### Sprint 1 (Critique)
1. Phase 1: Paramètre `maxInitialIndexRequests`
2. Phase 2: Propagation aux ancêtres
3. Phase 3: Vérification récursive

### Sprint 2 (Important)
4. Phase 4: Création des prefix_status parents
5. Phase 5: Mise à jour index lors de force_refresh
6. Phase 6: Recalcul stats après Load More/All

### Sprint 3 (Amélioration)
7. Phase 7: Affichage nouveaux dossiers
8. Phase 8: Batch size configurable

---

## Tests à Effectuer

### Test 1: Propagation ancêtres
1. Indexer un bucket partiellement
2. Upload un fichier dans `deep/nested/folder/file.txt`
3. Vérifier que `deep/`, `deep/nested/`, `deep/nested/folder/` sont tous incomplets

### Test 2: Vérification récursive
1. Indexer complètement `folder/`
2. Ajouter un objet dans `folder/subfolder/`
3. Vérifier que `folder/` est maintenant incomplet

### Test 3: Force refresh
1. Indexer un bucket partiellement
2. Force refresh des stats
3. Vérifier que l'index contient maintenant tous les objets
4. Vérifier que le bucket est marqué complet

### Test 4: Load All
1. Naviguer vers un dossier avec pagination
2. Cliquer Load All
3. Vérifier que les stats affichées sont mises à jour

---

## Estimation de Temps

| Phase | Complexité | Estimation |
|-------|------------|------------|
| Phase 1 | Facile | 1-2h |
| Phase 2 | Moyenne | 2-3h |
| Phase 3 | Moyenne | 1-2h |
| Phase 4 | Moyenne | 2-3h |
| Phase 5 | Complexe | 3-4h |
| Phase 6 | Facile | 1h |
| Phase 7 | Moyenne | 2h |
| Phase 8 | Facile | 1h |

**Total estimé: 13-18 heures**
