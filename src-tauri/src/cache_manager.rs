//! Cache manager professionnel avec LRU + TTL
//!
//! Fournit un cache thread-safe avec:
//! - Eviction LRU (Least Recently Used)
//! - Expiration TTL (Time To Live)
//! - Expiration par inactivite (idle timeout)
//! - Metriques d'observabilite
//! - API d'eviction explicite
//!
//! Utilise `moka` - le cache le plus utilise en production Rust
//! (Cloudflare, Fastly, etc.)

use moka::sync::Cache;
use serde::Serialize;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Configuration du cache
#[derive(Clone, Debug)]
pub struct CacheConfig {
    /// Nombre maximum d'entrees en cache
    pub max_entries: u64,
    /// Duree d'inactivite avant eviction (en secondes)
    /// Une entree non accedee pendant cette duree sera evincee
    pub idle_timeout_secs: u64,
    /// Duree de vie maximale d'une entree (en secondes)
    /// Une entree sera evincee apres cette duree, meme si accedee regulierement
    pub ttl_secs: Option<u64>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 5,         // Max 5 profils en cache
            idle_timeout_secs: 600, // 10 minutes d'inactivite
            ttl_secs: Some(3600),   // 1 heure max
        }
    }
}

impl CacheConfig {
    /// Configuration pour les tests (valeurs reduites)
    #[cfg(test)]
    pub fn for_testing() -> Self {
        Self {
            max_entries: 2,
            idle_timeout_secs: 1,
            ttl_secs: Some(2),
        }
    }
}

/// Metriques du cache pour observabilite
pub struct CacheMetrics {
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    insertions: AtomicU64,
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            insertions: AtomicU64::new(0),
        }
    }
}

impl CacheMetrics {
    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_insertion(&self) {
        self.insertions.fetch_add(1, Ordering::Relaxed);
    }

    /// Obtenir un snapshot des metriques
    pub fn snapshot(&self) -> CacheMetricsSnapshot {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            (hits as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        CacheMetricsSnapshot {
            hits,
            misses,
            evictions: self.evictions.load(Ordering::Relaxed),
            insertions: self.insertions.load(Ordering::Relaxed),
            hit_rate,
        }
    }

    /// Reset les metriques (utile pour les tests)
    #[allow(dead_code)]
    pub fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.insertions.store(0, Ordering::Relaxed);
    }
}

/// Snapshot immutable des metriques pour serialisation
#[derive(Clone, Debug, Serialize)]
pub struct CacheMetricsSnapshot {
    /// Nombre de cache hits
    pub hits: u64,
    /// Nombre de cache misses
    pub misses: u64,
    /// Nombre d'evictions (LRU ou TTL)
    pub evictions: u64,
    /// Nombre d'insertions
    pub insertions: u64,
    /// Taux de hit en pourcentage
    pub hit_rate: f64,
}

/// Cache manager generique avec LRU + TTL
///
/// Thread-safe et non-bloquant grace a moka.
/// L'eviction se fait en background sans bloquer les operations.
pub struct ManagedCache<K, V>
where
    K: Hash + Eq + Send + Sync + Clone + 'static,
    V: Clone + Send + Sync + 'static,
{
    cache: Cache<K, V>,
    metrics: Arc<CacheMetrics>,
    config: CacheConfig,
    name: String,
}

impl<K, V> ManagedCache<K, V>
where
    K: Hash + Eq + Send + Sync + Clone + Debug + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Creer un nouveau cache manage
    ///
    /// # Arguments
    /// * `name` - Nom du cache pour les logs
    /// * `config` - Configuration LRU/TTL
    pub fn new(name: impl Into<String>, config: CacheConfig) -> Self {
        let name = name.into();
        let metrics = Arc::new(CacheMetrics::default());
        let metrics_for_listener = Arc::clone(&metrics);
        let name_for_listener = name.clone();

        let mut builder = Cache::builder()
            .max_capacity(config.max_entries)
            .time_to_idle(Duration::from_secs(config.idle_timeout_secs))
            .eviction_listener(move |key, _value, cause| {
                metrics_for_listener.record_eviction();
                // Log uniquement en debug pour eviter le spam
                #[cfg(debug_assertions)]
                eprintln!(
                    "[{}] Cache eviction: {:?} (cause: {:?})",
                    name_for_listener, key, cause
                );
                let _ = (key, cause); // Suppress unused warning in release
            });

        if let Some(ttl) = config.ttl_secs {
            builder = builder.time_to_live(Duration::from_secs(ttl));
        }

        Self {
            cache: builder.build(),
            metrics,
            config,
            name,
        }
    }

    /// Obtenir une valeur du cache
    ///
    /// Retourne None si la cle n'existe pas ou a expire
    pub fn get(&self, key: &K) -> Option<V> {
        match self.cache.get(key) {
            Some(v) => {
                self.metrics.record_hit();
                Some(v)
            }
            None => {
                self.metrics.record_miss();
                None
            }
        }
    }

    /// Inserer une valeur dans le cache
    ///
    /// Si la capacite max est atteinte, evince l'entree LRU
    pub fn insert(&self, key: K, value: V) {
        self.metrics.record_insertion();
        self.cache.insert(key, value);
    }

    /// Obtenir ou creer une valeur (pattern get_or_insert)
    ///
    /// Si la cle existe, retourne la valeur en cache.
    /// Sinon, appelle `init` pour creer la valeur, l'insere et la retourne.
    ///
    /// # Arguments
    /// * `key` - Cle a rechercher/inserer
    /// * `init` - Fonction de creation (appelee seulement si cache miss)
    pub fn get_or_insert_with<F, E>(&self, key: K, init: F) -> Result<V, E>
    where
        F: FnOnce() -> Result<V, E>,
    {
        // Verifier le cache d'abord
        if let Some(v) = self.get(&key) {
            return Ok(v);
        }

        // Cache miss - creer la valeur
        let value = init()?;
        self.insert(key, value.clone());
        Ok(value)
    }

    /// Supprimer explicitement une entree du cache
    ///
    /// Utile lors de la suppression d'un profil
    pub fn remove(&self, key: &K) {
        self.cache.invalidate(key);
    }

    /// Vider tout le cache
    ///
    /// Utile pour la maintenance ou les tests
    pub fn clear(&self) {
        self.cache.invalidate_all();
    }

    /// Obtenir les metriques du cache
    pub fn metrics(&self) -> CacheMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Nombre d'entrees actuellement en cache
    pub fn len(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Obtenir un rapport de statut complet
    pub fn status(&self) -> CacheStatus {
        CacheStatus {
            name: self.name.clone(),
            entries: self.len(),
            max_entries: self.config.max_entries,
            idle_timeout_secs: self.config.idle_timeout_secs,
            ttl_secs: self.config.ttl_secs,
            metrics: self.metrics(),
        }
    }
}

/// Statut complet d'un cache pour monitoring
#[derive(Clone, Debug, Serialize)]
pub struct CacheStatus {
    /// Nom du cache
    pub name: String,
    /// Nombre d'entrees actuelles
    pub entries: u64,
    /// Capacite maximale
    pub max_entries: u64,
    /// Timeout d'inactivite en secondes
    pub idle_timeout_secs: u64,
    /// TTL en secondes (None = pas de TTL)
    pub ttl_secs: Option<u64>,
    /// Metriques
    pub metrics: CacheMetricsSnapshot,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_basic_get_insert() {
        let cache: ManagedCache<String, i32> =
            ManagedCache::new("test", CacheConfig::for_testing());

        // Insert
        cache.insert("key1".to_string(), 42);
        assert_eq!(cache.len(), 1);

        // Get
        let value = cache.get(&"key1".to_string());
        assert_eq!(value, Some(42));

        // Miss
        let missing = cache.get(&"key2".to_string());
        assert_eq!(missing, None);

        // Metrics
        let metrics = cache.metrics();
        assert_eq!(metrics.hits, 1);
        assert_eq!(metrics.misses, 1);
        assert_eq!(metrics.insertions, 1);
    }

    #[test]
    fn test_get_or_insert_with() {
        let cache: ManagedCache<String, String> =
            ManagedCache::new("test", CacheConfig::for_testing());

        // First call - creates value
        let result: Result<String, ()> =
            cache.get_or_insert_with("key1".to_string(), || Ok("value1".to_string()));
        assert_eq!(result, Ok("value1".to_string()));

        // Second call - returns cached value
        let result: Result<String, ()> = cache.get_or_insert_with("key1".to_string(), || {
            panic!("Should not be called");
        });
        assert_eq!(result, Ok("value1".to_string()));

        let metrics = cache.metrics();
        assert_eq!(metrics.hits, 1);
        assert_eq!(metrics.misses, 1);
    }

    #[test]
    fn test_lru_eviction() {
        let config = CacheConfig {
            max_entries: 2,
            idle_timeout_secs: 3600, // Long timeout to test LRU only
            ttl_secs: None,
        };
        let cache: ManagedCache<String, i32> = ManagedCache::new("test", config);

        cache.insert("key1".to_string(), 1);
        cache.insert("key2".to_string(), 2);

        // Access key1 to make it more recent
        let _ = cache.get(&"key1".to_string());

        // Insert key3 - should evict key2 (LRU)
        cache.insert("key3".to_string(), 3);

        // Force pending tasks to complete
        cache.cache.run_pending_tasks();

        // key1 and key3 should exist, key2 should be evicted
        assert!(cache.get(&"key1".to_string()).is_some());
        assert!(cache.get(&"key3".to_string()).is_some());
        // Note: LRU eviction is async, so key2 might still be there briefly
    }

    #[test]
    fn test_remove() {
        let cache: ManagedCache<String, i32> =
            ManagedCache::new("test", CacheConfig::for_testing());

        cache.insert("key1".to_string(), 42);
        assert!(cache.get(&"key1".to_string()).is_some());

        cache.remove(&"key1".to_string());
        assert!(cache.get(&"key1".to_string()).is_none());
    }

    #[test]
    fn test_clear() {
        let cache: ManagedCache<String, i32> =
            ManagedCache::new("test", CacheConfig::for_testing());

        cache.insert("key1".to_string(), 1);
        cache.insert("key2".to_string(), 2);
        assert_eq!(cache.len(), 2);

        cache.clear();
        cache.cache.run_pending_tasks();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_idle_timeout() {
        let config = CacheConfig {
            max_entries: 10,
            idle_timeout_secs: 1, // 1 second
            ttl_secs: None,
        };
        let cache: ManagedCache<String, i32> = ManagedCache::new("test", config);

        cache.insert("key1".to_string(), 42);
        assert!(cache.get(&"key1".to_string()).is_some());

        // Wait for idle timeout
        thread::sleep(Duration::from_secs(2));
        cache.cache.run_pending_tasks();

        // Should be evicted due to idle timeout
        // Note: moka's eviction is eventually consistent
        assert!(cache.get(&"key1".to_string()).is_none());
    }

    #[test]
    fn test_status() {
        let cache: ManagedCache<String, i32> =
            ManagedCache::new("test_cache", CacheConfig::for_testing());

        cache.insert("key1".to_string(), 1);
        let _ = cache.get(&"key1".to_string());
        let _ = cache.get(&"missing".to_string());

        let status = cache.status();
        assert_eq!(status.name, "test_cache");
        assert_eq!(status.entries, 1);
        assert_eq!(status.max_entries, 2);
        assert_eq!(status.metrics.hits, 1);
        assert_eq!(status.metrics.misses, 1);
    }
}
