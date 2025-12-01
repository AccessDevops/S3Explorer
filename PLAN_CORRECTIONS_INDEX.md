# Plan d'Action - Corrections du Système d'Indexation SQLite

## Vue d'ensemble

Ce plan corrige les 5 problèmes critiques identifiés et implémente les fonctionnalités manquantes.

---

## Phase 1: Corrections Backend Critiques

### 1.1 Indexer les common_prefixes lors de la navigation

**Fichier**: `src-tauri/src/index_manager.rs`

**Problème**: `update_from_list_response` n'indexe que les objets, pas les dossiers découverts.

**Modification dans `update_from_list_response`**:

```rust
pub fn update_from_list_response(
    &self,
    bucket_name: &str,
    prefix: &str,
    response: &ListObjectsResponse,
) -> Result<usize, AppError> {
    // ... code existant pour indexer les objets ...

    // NOUVEAU: Créer des entrées prefix_status pour les common_prefixes
    for folder_prefix in &response.common_prefixes {
        // Vérifier si ce prefix existe déjà
        if self.db.get_prefix_status(bucket_name, folder_prefix)?.is_none() {
            // Créer une entrée avec is_complete = false (dossier découvert mais pas exploré)
            let folder_status = PrefixStatus {
                profile_id: self.profile_id.clone(),
                bucket_name: bucket_name.to_string(),
                prefix: folder_prefix.clone(),
                is_complete: false,
                objects_count: 0,  // Inconnu
                total_size: 0,     // Inconnu
                continuation_token: None,
                last_indexed_key: None,
                last_sync_started_at: None,
                last_sync_completed_at: None,
                ..Default::default()
            };
            self.db.upsert_prefix_status(&folder_status)?;
        }
    }

    // ... reste du code ...
}
```

---

### 1.2 Fallback mode delimiter après indexation initiale

**Fichier**: `src-tauri/src/index_manager.rs`

**Problème**: Après les N requêtes sans delimiter, on ne connaît pas la structure des dossiers.

**Modification dans `initial_index_bucket`** (après la boucle principale):

```rust
// Après la boucle d'indexation sans delimiter
if !is_complete {
    // Le bucket est trop grand - faire 1 requête avec delimiter à la racine
    // pour découvrir les dossiers de premier niveau
    let root_response = adapter
        .list_objects(
            bucket_name,
            Some(""), // Racine
            None,     // Pas de continuation
            Some(config.batch_size),
            true,     // AVEC delimiter cette fois
        )
        .await
        .map_err(|e| AppError::S3Error(e.to_string()))?;

    // Créer des entrées prefix_status pour chaque dossier découvert
    for folder_prefix in &root_response.common_prefixes {
        if self.db.get_prefix_status(bucket_name, folder_prefix)?.is_none() {
            let folder_status = PrefixStatus {
                profile_id: self.profile_id.clone(),
                bucket_name: bucket_name.to_string(),
                prefix: folder_prefix.clone(),
                is_complete: false,
                objects_count: 0,
                total_size: 0,
                continuation_token: None,
                last_indexed_key: None,
                last_sync_started_at: None,
                last_sync_completed_at: None,
                ..Default::default()
            };
            self.db.upsert_prefix_status(&folder_status)?;
        }
    }

    requests_made += 1;
}
```

---

### 1.3 Corriger is_prefix_complete pour éviter les faux positifs

**Fichier**: `src-tauri/src/database.rs`

**Problème**: Un dossier jamais visité n'a pas d'entrée `prefix_status`, donc n'est pas détecté comme incomplet.

**Nouvelle logique**:

```rust
/// Verifier si un prefixe est complet (incluant tous ses sous-prefixes)
pub fn is_prefix_complete(&self, bucket_name: &str, prefix: &str) -> Result<bool, AppError> {
    let conn = self.get_connection()?;

    // 1. Vérifier si le prefix lui-même existe et est marqué complet
    let self_status: Option<(bool, i64)> = conn
        .query_row(
            r#"
            SELECT is_complete, objects_count
            FROM prefix_status
            WHERE profile_id = ?1 AND bucket_name = ?2 AND prefix = ?3
            "#,
            params![self.profile_id, bucket_name, prefix],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    match self_status {
        None => {
            // Pas d'entrée = jamais exploré = incomplet
            return Ok(false);
        }
        Some((is_complete, _)) if !is_complete => {
            return Ok(false);
        }
        _ => {}
    }

    // 2. Vérifier qu'il n'y a pas d'enfants incomplets
    let prefix_pattern = if prefix.is_empty() {
        "%".to_string()
    } else {
        format!("{}%", prefix)
    };

    let has_incomplete_children: bool = conn
        .query_row(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM prefix_status
                WHERE profile_id = ?1
                  AND bucket_name = ?2
                  AND prefix LIKE ?3
                  AND prefix != ?4
                  AND is_complete = FALSE
            )
            "#,
            params![self.profile_id, bucket_name, prefix_pattern, prefix],
            |row| row.get(0),
        )
        .unwrap_or(true); // En cas d'erreur, considérer incomplet

    // 3. NOUVEAU: Vérifier si des objets indexés ont des sous-prefixes non explorés
    // Exemple: si on a indexé "folder/sub/file.txt" mais "folder/sub/" n'existe pas dans prefix_status
    let has_unexplored_subprefixes: bool = conn
        .query_row(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM objects o
                WHERE o.profile_id = ?1
                  AND o.bucket_name = ?2
                  AND o.key LIKE ?3
                  AND o.parent_prefix != ?4
                  AND o.parent_prefix != ''
                  AND NOT EXISTS (
                      SELECT 1 FROM prefix_status ps
                      WHERE ps.profile_id = o.profile_id
                        AND ps.bucket_name = o.bucket_name
                        AND ps.prefix = o.parent_prefix
                  )
            )
            "#,
            params![self.profile_id, bucket_name, prefix_pattern, prefix],
            |row| row.get(0),
        )
        .unwrap_or(true);

    Ok(!has_incomplete_children && !has_unexplored_subprefixes)
}
```

---

### 1.4 Gérer la suppression d'objets non indexés

**Fichier**: `src-tauri/src/index_manager.rs`

**Problème**: Si l'objet n'est pas dans l'index, les ancêtres ne sont pas marqués incomplets.

**Modification de `remove_object`**:

```rust
pub fn remove_object(&self, bucket_name: &str, key: &str) -> Result<(), AppError> {
    // Calculer le parent_prefix depuis la clé (même si l'objet n'est pas indexé)
    let parent_prefix = IndexedObject::extract_parent_prefix(key);

    // Essayer de supprimer de l'index
    let was_deleted = self.db.delete_object(bucket_name, key)?;

    // Toujours marquer les ancêtres comme incomplets (même si objet pas trouvé)
    // Car l'objet existait sur S3 et a été supprimé
    if !parent_prefix.is_empty() {
        self.db.mark_prefix_and_ancestors_incomplete(bucket_name, &parent_prefix)?;
    } else {
        self.db.mark_prefix_incomplete(bucket_name, "")?;
    }

    if was_deleted {
        log::debug!("Removed indexed object: {}", key);
    } else {
        log::debug!("Object was not in index: {}", key);
    }

    Ok(())
}
```

---

### 1.5 Utiliser batchSize depuis settings pour l'indexation

**Fichier**: `src-tauri/src/commands.rs`

**Modification de `start_initial_index`**:

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
        batch_size: batch_size.unwrap_or(1000),  // Utiliser la valeur passée
        stale_ttl_hours: 24,
    };

    // ...
}
```

**Fichier**: `src/services/tauri.ts`

```typescript
export async function startInitialIndex(
  profileId: string,
  bucketName: string,
  maxRequests?: number,
  batchSize?: number  // NOUVEAU
): Promise<InitialIndexResult> {
  return invoke('start_initial_index', {
    profileId,
    bucketName,
    maxRequests,
    batchSize,
  })
}
```

---

## Phase 2: Amélioration de la Cohérence des Données

### 2.1 Détection et nettoyage des objets fantômes

**Fichier**: `src-tauri/src/database.rs`

**Nouvelle fonction**:

```rust
/// Supprimer les objets de l'index qui ne sont plus dans la liste S3
/// Appelé lors d'un refresh complet d'un prefix
pub fn sync_prefix_objects(
    &self,
    bucket_name: &str,
    prefix: &str,
    current_keys: &[String],
) -> Result<i64, AppError> {
    let conn = self.get_connection()?;

    // Créer une table temporaire avec les clés actuelles
    conn.execute("CREATE TEMP TABLE IF NOT EXISTS current_keys (key TEXT PRIMARY KEY)", [])?;
    conn.execute("DELETE FROM current_keys", [])?;

    for key in current_keys {
        conn.execute("INSERT INTO current_keys (key) VALUES (?1)", params![key])?;
    }

    // Supprimer les objets qui sont dans l'index mais pas dans current_keys
    // Limité au prefix exact (parent_prefix = prefix)
    let deleted = conn.execute(
        r#"
        DELETE FROM objects
        WHERE profile_id = ?1
          AND bucket_name = ?2
          AND parent_prefix = ?3
          AND key NOT IN (SELECT key FROM current_keys)
        "#,
        params![self.profile_id, bucket_name, prefix],
    )?;

    conn.execute("DROP TABLE IF EXISTS current_keys", [])?;

    Ok(deleted as i64)
}
```

**Utilisation** - appeler lors de `reloadCurrentView` avec une nouvelle option:

```rust
// Dans commands.rs - list_objects
if sync_mode.unwrap_or(false) {
    // Collecter toutes les clés retournées
    let current_keys: Vec<String> = response.objects.iter().map(|o| o.key.clone()).collect();
    index_mgr.db.sync_prefix_objects(&bucket, &prefix_str, &current_keys)?;
}
```

---

### 2.2 Ajouter un flag "discovered" pour les dossiers non explorés

**Fichier**: `src-tauri/src/sql/schema_v1.sql` (ou migration v2)

```sql
ALTER TABLE prefix_status ADD COLUMN discovered_only BOOLEAN DEFAULT FALSE;
```

**Logique**:
- `discovered_only = true` : dossier vu dans common_prefixes mais jamais navigué
- `discovered_only = false` : dossier exploré au moins une fois

---

## Phase 3: Améliorations Frontend

### 3.1 Afficher "-" pour les nouveaux dossiers

**Fichier**: `src/components/ObjectBrowser.vue`

**Ajouter une nouvelle fonction**:

```typescript
// Vérifier si un dossier est "découvert mais non exploré"
async function isFolderDiscoveredOnly(folder: string): Promise<boolean> {
  if (!appStore.currentProfile || !appStore.currentBucket) return true

  try {
    const stats = await indexManager.getFolderStats(
      appStore.currentProfile.id,
      appStore.currentBucket,
      folder
    )
    // Si pas de stats ou objects_count = 0 et non complet = découvert seulement
    return !stats || (stats.objects_count === 0 && !stats.is_complete)
  } catch {
    return true
  }
}

// Modifier getFolderSize
function getFolderSize(folder: string): string {
  // Si dossier dans unknownFolders, afficher "-"
  if (unknownFolders.value.has(folder)) {
    return '-'
  }

  const size = folderSizes.value.get(folder)
  if (size === undefined) {
    // Charger la taille en arrière-plan
    loadFolderSizeFromIndex(folder)
    return '...'
  }
  return formatSize(size)
}

// Charger la taille depuis l'index
async function loadFolderSizeFromIndex(folder: string) {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  try {
    const stats = await indexManager.getFolderStats(
      appStore.currentProfile.id,
      appStore.currentBucket,
      folder
    )

    if (!stats || (stats.objects_count === 0 && !stats.is_complete)) {
      // Dossier découvert mais non exploré
      unknownFolders.value.add(folder)
    } else {
      folderSizes.value.set(folder, stats.total_size)
      folderSizeIsEstimate.value.set(folder, !stats.is_complete)
    }
  } catch {
    unknownFolders.value.add(folder)
  }
}
```

---

### 3.2 Passer batchSize lors de l'indexation

**Fichier**: `src/composables/useIndexManager.ts`

```typescript
async function startIndexing(
  profileId: string,
  bucketName: string,
  maxRequests: number = 20
): Promise<InitialIndexResult | null> {
  const settingsStore = useSettingsStore()
  const key = `${profileId}-${bucketName}`

  try {
    indexingBuckets.value[key] = true
    const result = await startInitialIndex(
      profileId,
      bucketName,
      maxRequests,
      settingsStore.batchSize  // NOUVEAU: passer batchSize
    )
    // ...
  }
}
```

---

## Phase 4: Nouvelles Commandes IPC

### 4.1 Commande pour obtenir le statut d'un prefix

**Fichier**: `src-tauri/src/commands.rs`

```rust
#[tauri::command]
pub async fn get_prefix_status(
    profile_id: String,
    bucket_name: String,
    prefix: String,
) -> Result<Option<PrefixStatus>, String> {
    let index_mgr = get_index_manager(&profile_id).map_err(|e| e.to_string())?;
    index_mgr
        .get_prefix_status(&bucket_name, &prefix)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn is_prefix_discovered_only(
    profile_id: String,
    bucket_name: String,
    prefix: String,
) -> Result<bool, String> {
    let index_mgr = get_index_manager(&profile_id).map_err(|e| e.to_string())?;

    match index_mgr.get_prefix_status(&bucket_name, &prefix)? {
        None => Ok(true), // Pas dans l'index = découvert seulement
        Some(status) => {
            // discovered_only ou (objects_count = 0 et non complet)
            Ok(status.objects_count == 0 && !status.is_complete)
        }
    }
}
```

---

## Ordre d'Implémentation Recommandé

| Priorité | Tâche | Fichiers | Effort |
|----------|-------|----------|--------|
| 1 | 1.1 Indexer common_prefixes | index_manager.rs | 30 min |
| 2 | 1.4 Gérer suppression objets non indexés | index_manager.rs | 15 min |
| 3 | 1.3 Corriger is_prefix_complete | database.rs | 45 min |
| 4 | 1.2 Fallback delimiter | index_manager.rs | 30 min |
| 5 | 1.5 batchSize paramétrable | commands.rs, tauri.ts | 20 min |
| 6 | 3.1 Afficher "-" pour nouveaux dossiers | ObjectBrowser.vue | 30 min |
| 7 | 2.1 Nettoyage objets fantômes | database.rs | 45 min |
| 8 | 4.1 Nouvelles commandes IPC | commands.rs, tauri.ts | 30 min |

**Temps total estimé**: ~4 heures

---

## Tests à Effectuer

### Scénarios de test

1. **Bucket > 20,000 objets**
   - Indexation initiale → vérifier structure dossiers découverte
   - Stats affichées en jaune (~)

2. **Navigation dans dossier non indexé**
   - Affiche "-" pour la taille
   - Après navigation: taille calculée et affichée

3. **CRUD optimiste**
   - Upload → fichier apparaît immédiatement
   - Delete → fichier disparaît immédiatement
   - Vérifier que les stats sont mises à jour

4. **Suppression d'objet non indexé**
   - Supprimer un fichier qui n'est pas dans l'index
   - Vérifier que les ancêtres sont marqués incomplets

5. **Refresh complet**
   - Supprimer un fichier via autre client
   - Refresh → fichier fantôme supprimé de l'index

---

## Notes Importantes

- Les modifications du schéma SQLite nécessitent une migration (incrémenter SCHEMA_VERSION)
- Tester avec MinIO local avant de déployer
- Les modifications backend doivent être backwards-compatible avec les index existants
