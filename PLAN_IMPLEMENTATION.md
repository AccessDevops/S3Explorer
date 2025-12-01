# Plan d'Implémentation - Corrections Système d'Indexation SQLite

## Vue d'ensemble

Ce plan corrige les 6 problèmes identifiés lors de la revue de code. L'objectif est de garantir la cohérence des données entre S3 et l'index SQLite tout en maintenant les performances.

---

## Phase 1: Corrections Critiques (Priorité Haute)

### 1.1 Passer `syncIndex=true` lors de la navigation

**Fichiers à modifier:**
- `src/stores/app.ts`

**Problème:** Le paramètre `syncIndex` n'est jamais passé à `listObjects`, donc la synchronisation des objets fantômes ne se fait jamais.

**Modification dans `loadObjects()` (ligne ~284):**

```typescript
// AVANT
const response = await tauriService.listObjects(
  currentProfile.value.id,
  currentBucket.value,
  currentPrefix.value || undefined,
  loadMore ? continuationToken.value : undefined,
  settingsStore.batchSize,
  true // useDelimiter
)

// APRÈS
const response = await tauriService.listObjects(
  currentProfile.value.id,
  currentBucket.value,
  currentPrefix.value || undefined,
  loadMore ? continuationToken.value : undefined,
  settingsStore.batchSize,
  true,           // useDelimiter
  !loadMore       // syncIndex = true uniquement pour la première page (pas loadMore)
)
```

**Modification dans `preloadNextPage()` (ligne ~211):**

Le preload ne doit PAS déclencher syncIndex (pour éviter les suppressions non désirées):
```typescript
const response = await tauriService.listObjects(
  currentProfile.value.id,
  currentBucket.value,
  currentPrefix.value || undefined,
  continuationToken.value,
  settingsStore.batchSize,
  true,   // useDelimiter
  false   // syncIndex = false pour le preload (explicite)
)
```

---

### 1.2 Corriger `copy_object` pour toujours indexer la destination

**Fichiers à modifier:**
- `src-tauri/src/commands.rs`

**Problème:** Si l'objet source n'est pas dans l'index, la destination n'est pas ajoutée.

**Modification dans `copy_object()` (ligne ~539-557):**

```rust
// AVANT
if result.is_ok() {
    if let Ok(index_mgr) = get_index_manager(&profile_id) {
        if let Ok(Some(src_obj)) = index_mgr.db.get_object(&source_bucket, &source_key) {
            let new_obj = S3Object { ... };
            let _ = index_mgr.add_object(&dest_bucket, &new_obj);
        }
    }
}

// APRÈS
if result.is_ok() {
    if let Ok(index_mgr) = get_index_manager(&profile_id) {
        // Essayer de récupérer les infos de la source, sinon utiliser des valeurs par défaut
        let (size, storage_class) = index_mgr
            .db
            .get_object(&source_bucket, &source_key)
            .ok()
            .flatten()
            .map(|o| (o.size, o.storage_class))
            .unwrap_or((0, "STANDARD".to_string()));

        let new_obj = S3Object {
            key: dest_key.clone(),
            size,
            last_modified: Some(chrono::Utc::now().to_rfc3339()),
            storage_class: Some(storage_class),
            e_tag: None,
            is_folder: dest_key.ends_with('/'),
        };
        let _ = index_mgr.add_object(&dest_bucket, &new_obj);
    }
}
```

---

## Phase 2: Amélioration de l'UX (Priorité Moyenne)

### 2.1 Implémenter l'affichage "-" pour les dossiers non explorés

**Fichiers à modifier:**
- `src/components/ObjectBrowser.vue`
- `src/services/tauri.ts` (vérifier que `isPrefixDiscoveredOnly` est exporté)

**Problème:** Les dossiers découverts mais non explorés affichent "0 B" au lieu de "-".

**Étape 1: Ajouter un état réactif pour les dossiers inconnus**

Dans la section `<script setup>` de ObjectBrowser.vue, ajouter:

```typescript
// État pour les dossiers dont la taille est inconnue (découverts mais non explorés)
const unknownFolders = ref<Set<string>>(new Set())

// Reset lors du changement de bucket/prefix
watch([() => appStore.currentBucket, () => appStore.currentPrefix], () => {
  unknownFolders.value.clear()
})
```

**Étape 2: Modifier la fonction de calcul de taille des dossiers**

Rechercher la fonction qui affiche la taille des dossiers (probablement `getFolderSize` ou similaire) et la modifier:

```typescript
async function loadFolderSizeFromIndex(folder: string) {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  try {
    // Vérifier si le dossier est "discovered only"
    const isDiscoveredOnly = await isPrefixDiscoveredOnly(
      appStore.currentProfile.id,
      appStore.currentBucket,
      folder
    )

    if (isDiscoveredOnly) {
      unknownFolders.value.add(folder)
      return
    }

    // Sinon, charger les stats normalement
    const stats = await getPrefixIndexStats(
      appStore.currentProfile.id,
      appStore.currentBucket,
      folder
    )

    if (stats) {
      folderSizes.value.set(folder, stats.total_size)
      folderSizeIsEstimate.value.set(folder, !stats.is_complete)
    }
  } catch {
    unknownFolders.value.add(folder)
  }
}

function getFolderSizeDisplay(folder: string): string {
  // Si dossier inconnu, afficher "-"
  if (unknownFolders.value.has(folder)) {
    return '-'
  }

  const size = folderSizes.value.get(folder)
  if (size === undefined) {
    // Déclencher le chargement en arrière-plan
    loadFolderSizeFromIndex(folder)
    return '...'
  }

  return formatSize(size)
}
```

**Étape 3: Modifier le template pour utiliser la nouvelle fonction**

Rechercher où la taille des dossiers est affichée et remplacer par `getFolderSizeDisplay(folder)`.

---

### 2.2 Afficher les stats en jaune quand l'index est incomplet

**Fichiers à modifier:**
- `src/components/BucketList.vue` ou composant affichant les stats

**Problème:** Les stats incomplètes doivent être visuellement distinctes.

Le backend retourne déjà `isEstimate` (ou `is_complete`). Il faut s'assurer que le frontend affiche:
- Stats complètes: couleur normale
- Stats incomplètes: couleur jaune/orange avec tooltip explicatif

```vue
<template>
  <span :class="{ 'text-amber-500': stats.isEstimate }"
        :title="stats.isEstimate ? 'Estimation (index incomplet)' : ''">
    {{ formatSize(stats.size) }}
  </span>
</template>
```

---

## Phase 3: Corrections de Cohérence (Priorité Basse)

### 3.1 Recalculer `objects_count` correctement dans `update_from_list_response`

**Fichiers à modifier:**
- `src-tauri/src/index_manager.rs`

**Problème:** `objects_count` stocke le nombre d'objets de la page, pas le total récursif.

**Modification dans `update_from_list_response()` (ligne ~271-276):**

```rust
// AVANT
// Recalculer les stats
let (obj_count, total_size) = self.db.calculate_prefix_stats(bucket_name, prefix)?;
status.objects_count = obj_count;
status.total_size = total_size;

// Note: calculate_prefix_stats est récursif (LIKE "prefix%")
// Cela compte tous les objets dans le prefix ET ses sous-prefixes
// C'est le comportement voulu pour afficher la taille totale du dossier
```

Ce code est déjà correct. Le problème est lors de la **création** d'un nouveau prefix_status (ligne 284-285):

```rust
// AVANT
objects_count: indexed_objects.len() as i64,
total_size: indexed_objects.iter().map(|o| o.size).sum(),

// APRÈS - Calculer les vraies stats (récursives)
let (real_count, real_size) = self.db.calculate_prefix_stats(bucket_name, prefix)?;
// ... dans PrefixStatus
objects_count: real_count,
total_size: real_size,
```

---

### 3.2 Nettoyage des `prefix_status` orphelins

**Fichiers à modifier:**
- `src-tauri/src/database.rs`
- `src-tauri/src/index_manager.rs`

**Problème:** Les entrées `prefix_status` de dossiers supprimés restent dans la DB.

**Nouvelle fonction dans `database.rs`:**

```rust
/// Supprimer les entrées prefix_status qui n'ont plus d'objets correspondants
pub fn cleanup_orphan_prefix_status(&self, bucket_name: &str) -> Result<i64, AppError> {
    let conn = self.get_connection()?;

    let deleted = conn.execute(
        r#"
        DELETE FROM prefix_status
        WHERE profile_id = ?1
          AND bucket_name = ?2
          AND prefix != ''  -- Ne pas supprimer la racine
          AND NOT EXISTS (
              SELECT 1 FROM objects o
              WHERE o.profile_id = prefix_status.profile_id
                AND o.bucket_name = prefix_status.bucket_name
                AND o.key LIKE prefix_status.prefix || '%'
          )
        "#,
        params![self.profile_id, bucket_name],
    )?;

    Ok(deleted as i64)
}
```

**Appeler cette fonction dans `sync_prefix_objects` après la synchronisation:**

```rust
// Dans index_manager.rs::sync_prefix_objects()
let deleted = self.db.sync_prefix_objects(bucket_name, prefix, current_keys)?;

// Nettoyer les prefix_status orphelins (optionnel, peut être coûteux)
if deleted > 0 {
    let _ = self.db.cleanup_orphan_prefix_status(bucket_name);
}
```

---

## Phase 4: Tests et Validation

### 4.1 Scénarios de test à exécuter

| Test | Description | Résultat attendu |
|------|-------------|------------------|
| T1 | Upload fichier puis navigation ailleurs et retour | Fichier visible, index cohérent |
| T2 | Suppression externe (via autre client) puis refresh | Fichier fantôme disparaît |
| T3 | Bucket > 20k objets, navigation dans dossier non indexé | Taille affiche "-" |
| T4 | Rename (copy+delete) sur fichier non indexé | Destination apparaît correctement |
| T5 | Refresh après suppression de dossier externe | prefix_status nettoyé |

### 4.2 Commandes de test

```bash
# Rust unit tests
cd src-tauri && cargo test

# Build check
cargo check
cargo clippy

# Frontend type check
npm run type-check
```

---

## Ordre d'implémentation recommandé

| Étape | Tâche | Fichiers | Temps estimé |
|-------|-------|----------|--------------|
| 1 | Passer syncIndex dans loadObjects | `src/stores/app.ts` | 10 min |
| 2 | Corriger copy_object | `src-tauri/src/commands.rs` | 15 min |
| 3 | Implémenter affichage "-" | `ObjectBrowser.vue` | 30 min |
| 4 | Afficher stats en jaune | `BucketList.vue` | 15 min |
| 5 | Corriger objects_count | `index_manager.rs` | 10 min |
| 6 | Cleanup prefix_status orphelins | `database.rs`, `index_manager.rs` | 20 min |
| 7 | Tests manuels | - | 30 min |

**Temps total estimé:** ~2h30

---

## Notes importantes

1. **Backwards compatibility:** Les modifications sont compatibles avec les index existants
2. **Performance:** `cleanup_orphan_prefix_status` peut être coûteux sur gros buckets - à appeler avec modération
3. **syncIndex:** Ne l'activer que sur la première page pour éviter les suppressions non désirées lors du pagination
