# Plan de Migration des Métriques : IndexedDB → SQLite

## Résumé

Migrer le stockage des métriques S3 du frontend (IndexedDB) vers le backend Rust (SQLite), en cohérence avec l'architecture de l'index local.

---

## État Actuel

### Architecture actuelle (à supprimer)
```
Frontend                           Backend
────────────────────────────────   ──────────────────
useMetrics.ts                      commands.rs
    │                                   │
    ▼                                   ▼
listen("metrics:s3-request") ◄──── emit_all() via MetricsContext
    │
    ▼
eventToRecord() → metricsStorage.recordRequest()
    │
    ▼
IndexedDB (s3explorer-metrics)
```

### Fichiers frontend à modifier/supprimer
| Fichier | Action |
|---------|--------|
| `src/services/metricsStorage.ts` | **SUPPRIMER** (833 lignes) |
| `src/composables/useMetrics.ts` | **RÉÉCRIRE** (appels Tauri au lieu d'IndexedDB) |
| `src/composables/useCacheMetrics.ts` | **RÉÉCRIRE** (appels Tauri au lieu d'IndexedDB) |
| `src/components/MetricsButton.vue` | **ADAPTER** (utiliser les nouvelles méthodes) |
| `src/types/metrics.ts` | **CONSERVER** (types partagés frontend/backend) |

### Tables SQLite déjà créées (schema_v1.sql)
- `metrics_requests` - requêtes individuelles
- `metrics_daily_stats` - statistiques journalières
- `metrics_cache_events` - événements de cache

---

## Architecture Cible

```
Frontend                           Backend (Rust)
────────────────────────────────   ──────────────────────────────────
useMetrics.ts                      commands.rs
    │                                   │
    ▼                                   │ (1) Operation S3
invoke("get_metrics_today")             │
invoke("get_metrics_history")           ▼
invoke("record_cache_event")       MetricsContext → emit_all()
    │                                   │
    │                                   ▼ (2) Stockage auto
    │                              metrics_storage.rs (NOUVEAU)
    │                                   │
    │                                   ▼
    └──────────────────────────────► SQLite (metrics_*)
```

**Changement clé**: Le backend stocke DIRECTEMENT les métriques dans SQLite au lieu d'émettre des événements que le frontend doit stocker.

---

## Plan d'Action Détaillé

### Phase 1 : Backend Rust (Nouveau module metrics_storage.rs)

#### 1.1 Créer `src-tauri/src/metrics_storage.rs`
Nouveau module pour gérer le stockage des métriques dans SQLite.

```rust
// Fonctions à implémenter
pub fn record_request(db: &Connection, event: &S3MetricsEvent) -> Result<(), AppError>
pub fn record_cache_event(db: &Connection, event: &CacheEvent) -> Result<(), AppError>

// Requêtes de lecture
pub fn get_today_stats(db: &Connection) -> Result<DailyStats, AppError>
pub fn get_stats_history(db: &Connection, days: u32) -> Result<Vec<DailyStats>, AppError>
pub fn get_hourly_stats(db: &Connection, date: &str) -> Result<Vec<HourlyStats>, AppError>
pub fn get_operation_stats(db: &Connection, days: u32) -> Result<Vec<OperationStats>, AppError>
pub fn get_error_stats(db: &Connection, days: u32) -> Result<Vec<ErrorStats>, AppError>
pub fn get_top_buckets(db: &Connection, days: u32, limit: u32) -> Result<Vec<BucketUsageStats>, AppError>
pub fn get_recent_requests(db: &Connection, limit: u32) -> Result<Vec<S3RequestRecord>, AppError>
pub fn get_failed_requests(db: &Connection, days: u32, limit: u32) -> Result<Vec<S3RequestRecord>, AppError>

// Maintenance
pub fn purge_old_data(db: &Connection, retention_days: u32) -> Result<u64, AppError>
pub fn get_storage_info(db: &Connection) -> Result<StorageInfo, AppError>
pub fn clear_all(db: &Connection) -> Result<(), AppError>
pub fn purge_cache_events(db: &Connection, retention_days: u32) -> Result<u64, AppError>

// Cache metrics
pub fn get_cache_summary(db: &Connection, days: u32) -> Result<CacheSummary, AppError>
pub fn get_today_cache_stats(db: &Connection) -> Result<DailyCacheStats, AppError>
```

#### 1.2 Modifier `MetricsContext` dans `metrics.rs`
Au lieu d'émettre un événement Tauri, stocker directement dans SQLite.

```rust
// Avant (actuel)
pub fn emit_success(self, app: &AppHandle) {
    emit_metrics(app, event);  // Émet au frontend
}

// Après (nouveau)
pub fn emit_success(self, app: &AppHandle) {
    // 1. Toujours émettre l'événement (pour updates UI temps réel)
    emit_metrics(app, event.clone());

    // 2. NOUVEAU: Stocker dans SQLite
    if let Ok(db) = get_metrics_db() {
        let _ = metrics_storage::record_request(&db, &event);
    }
}
```

#### 1.3 Nouvelles commandes Tauri dans `commands.rs`

```rust
#[tauri::command]
pub async fn get_metrics_today() -> Result<DailyStats, String>

#[tauri::command]
pub async fn get_metrics_history(days: u32) -> Result<Vec<DailyStats>, String>

#[tauri::command]
pub async fn get_metrics_hourly(date: Option<String>) -> Result<Vec<HourlyStats>, String>

#[tauri::command]
pub async fn get_metrics_by_operation(days: u32) -> Result<Vec<OperationStats>, String>

#[tauri::command]
pub async fn get_metrics_errors(days: u32) -> Result<Vec<ErrorStats>, String>

#[tauri::command]
pub async fn get_metrics_top_buckets(days: u32, limit: u32) -> Result<Vec<BucketUsageStats>, String>

#[tauri::command]
pub async fn get_metrics_recent(limit: u32) -> Result<Vec<S3RequestRecord>, String>

#[tauri::command]
pub async fn purge_metrics(retention_days: u32) -> Result<u64, String>

#[tauri::command]
pub async fn clear_metrics() -> Result<(), String>

#[tauri::command]
pub async fn get_metrics_storage_info() -> Result<StorageInfo, String>

// Cache metrics
#[tauri::command]
pub async fn record_cache_event(event: CacheEvent) -> Result<(), String>

#[tauri::command]
pub async fn get_cache_summary(days: u32) -> Result<CacheSummary, String>
```

#### 1.4 Ajouter les métriques manquantes

**`start_initial_index`** - Ajouter tracking des ListObjectsV2:
```rust
// Dans initial_index_bucket(), après chaque adapter.list_objects():
let ctx = MetricsContext::new(S3Operation::ListObjectsV2, RequestCategory::LIST)
    .with_profile(&profile_id, &profile_name)
    .with_bucket(bucket_name);
ctx.emit_success(&app);
```

**`download_file`** - Ajouter tracking HeadObject et GetObject:
```rust
// Après get_object_size:
let ctx = MetricsContext::new(S3Operation::HeadObject, RequestCategory::GET)
    .with_profile(&profile_id, &profile_name)
    .with_bucket(&bucket)
    .with_object_key(&key);
ctx.emit_success(&app);

// Après get_object_stream:
let mut ctx = MetricsContext::new(S3Operation::GetObject, RequestCategory::GET)
    .with_profile(&profile_id, &profile_name)
    .with_bucket(&bucket)
    .with_object_key(&key);
ctx.set_bytes(file_size);
ctx.emit_success(&app);
```

---

### Phase 2 : Frontend Vue (Réécriture)

#### 2.1 Supprimer `metricsStorage.ts`
Fichier entier à supprimer (833 lignes) - plus nécessaire.

#### 2.2 Réécrire `useMetrics.ts`

```typescript
// Nouvelle implémentation utilisant Tauri
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'

export function useMetrics() {
  // État réactif (inchangé)
  const todayStats = ref<DailyStats | null>(null)
  const isLoading = ref(false)

  // Listener pour mises à jour temps réel (optionnel, pour counter en direct)
  onMounted(async () => {
    await listen('metrics:s3-request', () => {
      realtimeRequestCount.value++
      // Invalider le cache local
      todayStats.value = null
    })
  })

  // Méthodes - maintenant via invoke()
  async function refreshTodayStats() {
    todayStats.value = await invoke('get_metrics_today')
  }

  async function getStatsHistory(days: number): Promise<DailyStats[]> {
    return invoke('get_metrics_history', { days })
  }

  async function getHourlyStats(): Promise<HourlyStats[]> {
    return invoke('get_metrics_hourly', { date: null })
  }

  async function getOperationStats(days: number): Promise<OperationStats[]> {
    return invoke('get_metrics_by_operation', { days })
  }

  async function getErrorStats(days: number): Promise<ErrorStats[]> {
    return invoke('get_metrics_errors', { days })
  }

  async function getTopBuckets(days: number, limit: number = 10): Promise<BucketUsageStats[]> {
    return invoke('get_metrics_top_buckets', { days, limit })
  }

  async function purgeData(retentionDays: number): Promise<number> {
    return invoke('purge_metrics', { retentionDays })
  }

  async function clearAllData(): Promise<void> {
    await invoke('clear_metrics')
    todayStats.value = null
  }

  async function getStorageInfo() {
    return invoke('get_metrics_storage_info')
  }

  // Export CSV supprimé - fonctionnalité non conservée
}
```

#### 2.3 Réécrire `useCacheMetrics.ts`

```typescript
import { invoke } from '@tauri-apps/api/tauri'

export function useCacheMetrics() {
  async function recordCacheHit(operation: CacheOperation, options?: CacheOptions) {
    await invoke('record_cache_event', {
      event: {
        id: generateId(),
        timestamp: Date.now(),
        date: getTodayDate(),
        operation,
        hit: true,
        ...options
      }
    })
  }

  async function recordCacheMiss(operation: CacheOperation, options?: CacheOptions) {
    await invoke('record_cache_event', {
      event: {
        id: generateId(),
        timestamp: Date.now(),
        date: getTodayDate(),
        operation,
        hit: false,
        ...options
      }
    })
  }

  async function getCacheSummary(days: number): Promise<CacheSummary> {
    return invoke('get_cache_summary', { days })
  }

  // ...
}
```

#### 2.4 Adapter `MetricsButton.vue`
Changements mineurs - les méthodes gardent les mêmes signatures.

#### 2.5 Ajouter wrappers Tauri dans `tauri.ts`

```typescript
// Metrics queries
export async function getMetricsToday(): Promise<DailyStats> {
  return invoke('get_metrics_today')
}

export async function getMetricsHistory(days: number): Promise<DailyStats[]> {
  return invoke('get_metrics_history', { days })
}

// ... autres fonctions
```

---

### Phase 3 : Gestion de la base de données

#### 3.1 Choix d'architecture: Base séparée ou partagée?

**Option A: Base séparée pour les métriques** (RECOMMANDÉ)
- Fichier: `~/.s3explorer/metrics.db`
- Avantages:
  - Isolation des données
  - Peut être purgée indépendamment
  - Pas d'impact sur les index

**Option B: Même base que l'index**
- Fichier: `~/.s3explorer/index_{profile_id}.db`
- Inconvénient: Les métriques sont par profile, mais le dashboard est global

#### 3.2 Créer `get_metrics_db()` dans `database.rs`

```rust
lazy_static! {
    static ref METRICS_DB: Mutex<Option<Connection>> = Mutex::new(None);
}

pub fn get_metrics_db() -> Result<Connection, AppError> {
    // Ouvre ou crée ~/.s3explorer/metrics.db
    // Applique le schéma metrics_* si besoin
}
```

---

### Phase 4 : Tests et Nettoyage

#### 4.1 Tests à effectuer
- [ ] Vérifier que les métriques sont stockées dans SQLite
- [ ] Vérifier le dashboard avec données historiques
- [ ] Vérifier les métriques de cache
- [ ] Tester la purge des données
- [ ] Vérifier les métriques pour `start_initial_index`
- [ ] Vérifier les métriques pour `download_file`

#### 4.2 Fichiers à supprimer
- `src/services/metricsStorage.ts`

#### 4.3 Code mort à nettoyer
- Supprimer les types IndexedDB-specific dans `types/metrics.ts` si inutilisés

---

## Estimation de Complexité

| Phase | Fichiers | Complexité |
|-------|----------|------------|
| 1.1 metrics_storage.rs | +1 nouveau (~400 lignes) | Moyenne |
| 1.2 metrics.rs | Modification mineure | Faible |
| 1.3 commands.rs | +12 commandes (~150 lignes) | Moyenne |
| 1.4 Métriques manquantes | 2 fichiers | Faible |
| 2.1-2.3 Frontend | 3 fichiers réécrits | Moyenne |
| 2.5 tauri.ts | +10 wrappers | Faible |
| 3.x Database | 1 fichier modifié | Faible |
| 4.x Tests | - | Moyenne |

**Total estimé**: ~600 lignes de nouveau code Rust, ~200 lignes de nouveau code TypeScript, suppression de ~900 lignes (metricsStorage.ts + ancien code useMetrics).

---

## Ordre d'Exécution Recommandé

1. **Backend d'abord**: Implémenter `metrics_storage.rs` et les nouvelles commandes
2. **Test backend**: Vérifier avec `cargo test` que le stockage fonctionne
3. **Frontend ensuite**: Réécrire les composables pour utiliser Tauri
4. **Supprimer l'ancien**: Supprimer `metricsStorage.ts`
5. **Tests E2E**: Vérifier le dashboard complet
6. **Ajouter métriques manquantes**: `download_file` et `start_initial_index`

---

## Décisions

| Question | Décision |
|----------|----------|
| Export CSV | ❌ **SUPPRIMER** - Pas nécessaire |
| Migration données IndexedDB | ❌ **NON** - On repart de zéro |
| Base de données | ✅ **DÉDIÉE** - `~/.s3explorer/metrics.db` |
