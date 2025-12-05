//! Gestionnaire d'indexation S3
//!
//! Gere la logique d'indexation initiale, la mise a jour incrementale,
//! et le calcul des statistiques depuis l'index SQLite.
//!
//! Utilise un cache LRU+TTL pour limiter la memoire:
//! - Max 5 profils en cache simultanement
//! - Eviction apres 10 min d'inactivite
//! - TTL max de 1 heure

use std::sync::Arc;
use tokio::sync::broadcast;

use crate::cache_manager::{CacheConfig, CacheStatus, ManagedCache};
use crate::database::{get_db_manager, DatabaseManager};
use crate::errors::AppError;
use crate::models::{
    BucketIndexMetadata, BucketIndexStats, BucketInfo, IndexedObject, IndexingConfig,
    InitialIndexResult, ListObjectsResponse, PrefixStats, PrefixStatus, S3Object,
    StorageClassStats,
};
use crate::s3_adapter::S3Adapter;

/// Gestionnaire d'indexation pour un profil
pub struct IndexManager {
    pub db: Arc<DatabaseManager>,
    profile_id: String,
}

impl IndexManager {
    /// Creer un nouveau gestionnaire d'indexation
    pub fn new(profile_id: &str) -> Result<Self, AppError> {
        let db = get_db_manager(profile_id)?;
        Ok(Self {
            db,
            profile_id: profile_id.to_string(),
        })
    }

    // ========================================================================
    // Indexation Initiale
    // ========================================================================

    /// Effectuer l'indexation initiale d'un bucket
    ///
    /// Fait jusqu'a `config.max_initial_requests` requetes sans delimiter
    /// pour charger le maximum d'objets. Si le bucket est plus grand,
    /// passe en mode navigation avec delimiter.
    ///
    /// Le callback `on_progress` est appele apres chaque batch avec:
    /// (objects_indexed, requests_made, max_requests)
    ///
    /// Si `cancel_rx` est fourni, l'indexation peut etre interrompue.
    /// L'index partiel est conserve et peut etre repris plus tard.
    pub async fn initial_index_bucket<F>(
        &self,
        adapter: &S3Adapter,
        bucket_name: &str,
        config: &IndexingConfig,
        mut on_progress: F,
        mut cancel_rx: Option<broadcast::Receiver<()>>,
    ) -> Result<InitialIndexResult, AppError>
    where
        F: FnMut(u64, u32, u32),
    {
        let now = chrono::Utc::now().timestamp_millis();

        // Marquer le debut de l'indexation
        let mut bucket_info = self.db.get_bucket_info(bucket_name)?.unwrap_or(BucketInfo {
            id: None,
            profile_id: self.profile_id.clone(),
            bucket_name: bucket_name.to_string(),
            versioning_enabled: None,
            encryption_enabled: None,
            default_encryption: None,
            acl: None,
            acl_cached_at: None,
            region: None,
            initial_index_requests: 0,
            initial_index_completed: false,
            last_checked_at: Some(now),
        });

        // Creer le statut du prefixe racine
        let mut root_status = PrefixStatus {
            profile_id: self.profile_id.clone(),
            bucket_name: bucket_name.to_string(),
            prefix: String::new(),
            is_complete: false,
            objects_count: 0,
            total_size: 0,
            continuation_token: None,
            last_indexed_key: None,
            last_sync_started_at: Some(now),
            last_sync_completed_at: None,
            ..Default::default()
        };

        let mut total_indexed: u64 = 0;
        let mut total_size: i64 = 0;
        let mut requests_made: u32 = 0;
        let mut continuation_token: Option<String> = None;
        let mut last_key: Option<String> = None;
        let mut is_complete = false;

        // Flag pour savoir si l'indexation a ete annulee
        let mut was_cancelled = false;

        // Boucle d'indexation
        loop {
            // Verifier si l'annulation a ete demandee
            if let Some(ref mut rx) = cancel_rx {
                if rx.try_recv().is_ok() {
                    was_cancelled = true;
                    break;
                }
            }

            // Si max_initial_requests > 0, limiter le nombre de requetes
            // Si max_initial_requests = 0, pas de limite (indexation complete)
            if config.max_initial_requests > 0 && requests_made >= config.max_initial_requests {
                break;
            }

            // Requete S3 sans delimiter pour lister tous les objets
            let response = adapter
                .list_objects(
                    bucket_name,
                    None, // Pas de prefix - tout le bucket
                    continuation_token.clone(),
                    Some(config.batch_size),
                    false, // PAS de delimiter
                )
                .await
                .map_err(|e| AppError::S3Error(e.to_string()))?;

            requests_made += 1;

            // Convertir et indexer les objets
            let indexed_objects: Vec<IndexedObject> = response
                .objects
                .iter()
                .map(|obj| IndexedObject::from_s3_object(obj, &self.profile_id, bucket_name))
                .collect();

            if !indexed_objects.is_empty() {
                // Mettre a jour la derniere cle
                last_key = indexed_objects.last().map(|o| o.key.clone());

                // Calculer la taille totale
                let batch_size: i64 = indexed_objects.iter().map(|o| o.size).sum();
                total_size += batch_size;

                // Inserer en batch
                let count = self.db.upsert_objects_batch(&indexed_objects)?;
                total_indexed += count as u64;
            }

            // Emettre la progression apres chaque requete
            on_progress(total_indexed, requests_made, config.max_initial_requests);

            // Verifier si on a tout charge
            if !response.is_truncated {
                is_complete = true;
                break;
            }

            continuation_token = response.continuation_token;

            // Si pas de continuation token, on arrete
            if continuation_token.is_none() {
                break;
            }
        }

        // Gestion des prefix_status selon que l'indexation est complete ou non
        if is_complete {
            // Bucket complètement indexé: créer des entrées prefix_status pour tous
            // les parent_prefix uniques des objets indexés, marqués comme complets.
            // Cela permet à is_prefix_complete() de retourner true pour les dossiers.
            //
            // OPTIMISATION: Utilise une seule requête GROUP BY au lieu de N requêtes
            // individuelles (évite le problème N+1 queries).
            // Performance: 50K préfixes en ~3 secondes au lieu de ~4 minutes.
            let all_stats = self.db.calculate_all_prefix_stats_batch(bucket_name)?;
            let now = chrono::Utc::now().timestamp_millis();

            // Construire tous les PrefixStatus en mémoire
            let statuses: Vec<PrefixStatus> = all_stats
                .into_iter()
                .filter(|(prefix, _)| !prefix.is_empty())
                .map(|(prefix, (count, size))| PrefixStatus {
                    profile_id: self.profile_id.clone(),
                    bucket_name: bucket_name.to_string(),
                    prefix,
                    is_complete: true, // Marqué complet car tout le bucket est indexé
                    objects_count: count,
                    total_size: size,
                    continuation_token: None,
                    last_indexed_key: None,
                    last_sync_started_at: Some(now),
                    last_sync_completed_at: Some(now),
                    ..Default::default()
                })
                .collect();

            // Batch upsert en une seule transaction
            self.db.batch_upsert_prefix_status(&statuses)?;
        } else {
            // Bucket trop grand (indexation incomplete): faire une requete
            // avec delimiter a la racine pour decouvrir les dossiers de premier niveau.
            // Cela permet a l'utilisateur de naviguer meme si l'indexation n'est pas complete.
            let root_response = adapter
                .list_objects(
                    bucket_name,
                    Some(""), // Racine
                    None,     // Pas de continuation
                    Some(config.batch_size),
                    true, // AVEC delimiter pour voir les dossiers
                )
                .await
                .map_err(|e| AppError::S3Error(e.to_string()))?;

            requests_made += 1;

            // Creer des entrees prefix_status pour chaque dossier decouvert
            for folder_prefix in &root_response.common_prefixes {
                if self
                    .db
                    .get_prefix_status(bucket_name, folder_prefix)?
                    .is_none()
                {
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
        }

        // Mettre a jour le statut
        root_status.is_complete = is_complete;
        root_status.objects_count = total_indexed as i64;
        root_status.total_size = total_size;
        root_status.continuation_token = continuation_token.clone();
        root_status.last_indexed_key = last_key.clone();
        root_status.last_sync_completed_at = if is_complete {
            Some(chrono::Utc::now().timestamp_millis())
        } else {
            None
        };

        self.db.upsert_prefix_status(&root_status)?;

        // Mettre a jour les infos du bucket
        bucket_info.initial_index_requests = requests_made as i32;
        bucket_info.initial_index_completed = is_complete;
        bucket_info.last_checked_at = Some(chrono::Utc::now().timestamp_millis());
        self.db.upsert_bucket_info(&bucket_info)?;

        Ok(InitialIndexResult {
            total_indexed,
            is_complete,
            requests_made,
            continuation_token,
            last_key,
            total_size,
            error: if was_cancelled {
                Some("Cancelled by user".to_string())
            } else {
                None
            },
        })
    }

    // ========================================================================
    // Mise a Jour Incrementale
    // ========================================================================

    /// Mettre a jour l'index avec une reponse ListObjects
    ///
    /// Appele apres chaque requete list_objects pour garder l'index synchronise.
    /// Indexe les objets ET les dossiers (common_prefixes) decouverts.
    pub fn update_from_list_response(
        &self,
        bucket_name: &str,
        prefix: &str,
        response: &ListObjectsResponse,
    ) -> Result<usize, AppError> {
        // Convertir les objets S3 en IndexedObjects
        let indexed_objects: Vec<IndexedObject> = response
            .objects
            .iter()
            .map(|obj| IndexedObject::from_s3_object(obj, &self.profile_id, bucket_name))
            .collect();

        // Inserer/mettre a jour en batch
        let count = self.db.upsert_objects_batch(&indexed_objects)?;

        // Creer des entrees prefix_status pour les common_prefixes (dossiers decouverts)
        // Cela permet de savoir quels dossiers existent meme si on ne les a pas explores
        for folder_prefix in &response.common_prefixes {
            // Verifier si ce prefix existe deja dans l'index
            if self
                .db
                .get_prefix_status(bucket_name, folder_prefix)?
                .is_none()
            {
                // Creer une entree avec is_complete = false (dossier decouvert mais pas explore)
                let folder_status = PrefixStatus {
                    profile_id: self.profile_id.clone(),
                    bucket_name: bucket_name.to_string(),
                    prefix: folder_prefix.clone(),
                    is_complete: false,
                    objects_count: 0, // Inconnu - sera calcule quand le dossier sera explore
                    total_size: 0,    // Inconnu - sera calcule quand le dossier sera explore
                    continuation_token: None,
                    last_indexed_key: None,
                    last_sync_started_at: None, // Jamais synchronise
                    last_sync_completed_at: None,
                    ..Default::default()
                };
                self.db.upsert_prefix_status(&folder_status)?;
            }
        }

        // Mettre a jour le statut du prefixe courant
        if let Some(mut status) = self.db.get_prefix_status(bucket_name, prefix)? {
            // Mise a jour du continuation token
            status.continuation_token = response.continuation_token.clone();

            // Si la liste n'est pas tronquee, le prefixe est complet
            if !response.is_truncated {
                status.is_complete = true;
                status.last_sync_completed_at = Some(chrono::Utc::now().timestamp_millis());
            }

            // Recalculer les stats
            let (obj_count, total_size) = self.db.calculate_prefix_stats(bucket_name, prefix)?;
            status.objects_count = obj_count;
            status.total_size = total_size;

            self.db.upsert_prefix_status(&status)?;
        } else {
            // Creer un nouveau statut pour ce prefixe
            // Utiliser calculate_prefix_stats pour avoir les stats recursives correctes
            let (obj_count, total_size) = self.db.calculate_prefix_stats(bucket_name, prefix)?;

            let status = PrefixStatus {
                profile_id: self.profile_id.clone(),
                bucket_name: bucket_name.to_string(),
                prefix: prefix.to_string(),
                is_complete: !response.is_truncated,
                objects_count: obj_count,
                total_size,
                continuation_token: response.continuation_token.clone(),
                last_indexed_key: indexed_objects.last().map(|o| o.key.clone()),
                last_sync_started_at: Some(chrono::Utc::now().timestamp_millis()),
                last_sync_completed_at: if !response.is_truncated {
                    Some(chrono::Utc::now().timestamp_millis())
                } else {
                    None
                },
                ..Default::default()
            };
            self.db.upsert_prefix_status(&status)?;
        }

        Ok(count)
    }

    // ========================================================================
    // Mise a Jour Apres Operations CRUD
    // ========================================================================

    /// Ajouter un objet a l'index (apres put_object reussi)
    pub fn add_object(&self, bucket_name: &str, obj: &S3Object) -> Result<(), AppError> {
        let indexed = IndexedObject::from_s3_object(obj, &self.profile_id, bucket_name);
        self.db.upsert_object(&indexed)?;

        // Marquer le prefixe parent ET tous les ancetres comme incomplets
        // Cela garantit la coherence des stats a tous les niveaux
        let parent = &indexed.parent_prefix;
        if !parent.is_empty() {
            self.db
                .mark_prefix_and_ancestors_incomplete(bucket_name, parent)?;
        } else {
            // Objet a la racine - marquer le bucket comme incomplet
            self.db.mark_prefix_incomplete(bucket_name, "")?;
        }

        Ok(())
    }

    /// Supprimer un objet de l'index (apres delete_object reussi)
    ///
    /// Meme si l'objet n'est pas dans l'index (bucket partiellement indexe),
    /// on marque les ancetres comme incomplets pour garantir la coherence des stats.
    pub fn remove_object(&self, bucket_name: &str, key: &str) -> Result<(), AppError> {
        // Calculer le parent_prefix depuis la cle (meme si l'objet n'est pas indexe)
        let parent_prefix = IndexedObject::extract_parent_prefix(key);

        // Essayer de supprimer de l'index
        let was_deleted = self.db.delete_object(bucket_name, key)?;

        // Toujours marquer les ancetres comme incomplets (meme si objet pas trouve)
        // Car l'objet existait sur S3 et a ete supprime - les stats doivent etre recalculees
        if !parent_prefix.is_empty() {
            self.db
                .mark_prefix_and_ancestors_incomplete(bucket_name, &parent_prefix)?;
        } else {
            // Objet a la racine - marquer le bucket comme incomplet
            self.db.mark_prefix_incomplete(bucket_name, "")?;
        }

        // Note: was_deleted indicates if the object was in the index
        // In both cases, ancestors have been marked as incomplete
        let _ = was_deleted; // Suppress unused warning

        Ok(())
    }

    /// Supprimer un dossier de l'index (apres delete_folder reussi)
    pub fn remove_folder(&self, bucket_name: &str, prefix: &str) -> Result<i64, AppError> {
        let deleted = self.db.delete_objects_by_prefix(bucket_name, prefix)?;

        // Marquer le prefixe parent ET tous les ancetres comme incomplets
        let parent = IndexedObject::extract_parent_prefix(prefix);
        self.db
            .mark_prefix_and_ancestors_incomplete(bucket_name, &parent)?;

        // Supprimer aussi le prefix_status du dossier supprime
        self.db.delete_prefix_status(bucket_name, prefix)?;

        Ok(deleted)
    }

    /// Synchroniser les objets d'un prefix avec la liste actuelle de S3
    ///
    /// Supprime de l'index les objets qui ne sont plus dans current_keys.
    /// Cela permet de nettoyer les "objets fantomes" supprimes sur S3 par un autre client.
    /// Nettoie egalement les entrees prefix_status orphelines.
    pub fn sync_prefix_objects(
        &self,
        bucket_name: &str,
        prefix: &str,
        current_keys: &[String],
    ) -> Result<i64, AppError> {
        let deleted = self
            .db
            .sync_prefix_objects(bucket_name, prefix, current_keys)?;

        // Si des objets ont ete supprimes, marquer les ancetres comme incomplets
        // et nettoyer les prefix_status orphelins
        if deleted > 0 {
            self.db
                .mark_prefix_and_ancestors_incomplete(bucket_name, prefix)?;

            // Nettoyer les prefix_status qui n'ont plus d'objets
            // (cas ou un dossier entier a ete supprime sur S3)
            let _ = self.db.cleanup_orphan_prefix_status(bucket_name);
        }

        Ok(deleted)
    }

    // ========================================================================
    // Calcul de Statistiques
    // ========================================================================

    /// Obtenir les statistiques d'un bucket depuis l'index
    pub fn get_bucket_stats(&self, bucket_name: &str) -> Result<BucketIndexStats, AppError> {
        let (total_objects, total_size, is_complete) =
            self.db.calculate_bucket_stats(bucket_name)?;

        // Recuperer les stats par storage class
        let storage_breakdown = self.db.get_storage_class_stats(bucket_name)?;
        let storage_class_breakdown: Vec<StorageClassStats> = storage_breakdown
            .into_iter()
            .map(|(class, count, size)| StorageClassStats {
                storage_class: class,
                object_count: count,
                total_size: size,
            })
            .collect();

        // Recuperer le timestamp de derniere indexation
        let last_indexed_at = self
            .db
            .get_prefix_status(bucket_name, "")?
            .and_then(|s| s.last_sync_completed_at);

        // Calculer la taille estimee de l'index pour ce bucket
        let estimated_index_size = self.db.calculate_bucket_index_size(bucket_name)?;

        Ok(BucketIndexStats {
            bucket_name: bucket_name.to_string(),
            total_objects,
            total_size,
            is_complete,
            storage_class_breakdown,
            last_indexed_at,
            estimated_index_size,
        })
    }

    /// Obtenir les statistiques d'un prefixe (dossier)
    pub fn get_prefix_stats(
        &self,
        bucket_name: &str,
        prefix: &str,
    ) -> Result<PrefixStats, AppError> {
        let (objects_count, total_size) = self.db.calculate_prefix_stats(bucket_name, prefix)?;
        let is_complete = self.db.is_prefix_complete(bucket_name, prefix)?;

        let last_sync_at = self
            .db
            .get_prefix_status(bucket_name, prefix)?
            .and_then(|s| s.last_sync_completed_at);

        Ok(PrefixStats {
            prefix: prefix.to_string(),
            objects_count,
            total_size,
            is_complete,
            last_sync_at,
        })
    }

    /// Calculer la taille d'un dossier depuis l'index
    ///
    /// Retourne (taille, is_complete)
    pub fn calculate_folder_size(
        &self,
        bucket_name: &str,
        prefix: &str,
    ) -> Result<(i64, bool), AppError> {
        let (_, total_size) = self.db.calculate_prefix_stats(bucket_name, prefix)?;
        let is_complete = self.db.is_prefix_complete(bucket_name, prefix)?;

        Ok((total_size, is_complete))
    }

    // ========================================================================
    // Recherche
    // ========================================================================

    /// Rechercher des objets dans l'index
    pub fn search_objects(
        &self,
        bucket_name: &str,
        query: &str,
        prefix: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<S3Object>, AppError> {
        self.db.search_objects(bucket_name, query, prefix, limit)
    }

    /// Obtenir tous les index de buckets
    pub fn get_all_bucket_indexes(&self) -> Result<Vec<BucketIndexMetadata>, AppError> {
        self.db.get_all_bucket_indexes()
    }

    // ========================================================================
    // Utilitaires
    // ========================================================================

    /// Verifier si un bucket a deja ete indexe
    pub fn is_bucket_indexed(&self, bucket_name: &str) -> Result<bool, AppError> {
        Ok(self.db.get_bucket_info(bucket_name)?.is_some())
    }

    /// Verifier si l'index d'un bucket est complet
    pub fn is_bucket_complete(&self, bucket_name: &str) -> Result<bool, AppError> {
        self.db.is_prefix_complete(bucket_name, "")
    }

    /// Vider l'index d'un bucket
    pub fn clear_bucket_index(&self, bucket_name: &str) -> Result<(), AppError> {
        self.db.clear_bucket_index(bucket_name)
    }

    /// Obtenir le statut d'un prefixe
    pub fn get_prefix_status(
        &self,
        bucket_name: &str,
        prefix: &str,
    ) -> Result<Option<PrefixStatus>, AppError> {
        self.db.get_prefix_status(bucket_name, prefix)
    }
}

// ============================================================================
// Global Index Manager Cache with LRU + TTL
// ============================================================================

lazy_static::lazy_static! {
    /// Cache LRU+TTL des gestionnaires d'indexation
    ///
    /// Configuration:
    /// - Max 5 profils en cache (LRU eviction au-dela)
    /// - Eviction apres 10 min d'inactivite
    /// - TTL max de 1 heure
    static ref INDEX_MANAGERS: ManagedCache<String, Arc<IndexManager>> = {
        ManagedCache::new(
            "IndexManagers",
            CacheConfig {
                max_entries: 5,           // Max 5 profils en cache
                idle_timeout_secs: 600,   // 10 minutes d'inactivite
                ttl_secs: Some(3600),     // 1 heure max
            },
        )
    };
}

/// Obtenir ou creer un gestionnaire d'indexation pour un profil
///
/// Utilise un cache LRU+TTL pour limiter la memoire.
/// Si le cache est plein, le profil le moins recemment utilise est evince.
pub fn get_index_manager(profile_id: &str) -> Result<Arc<IndexManager>, AppError> {
    INDEX_MANAGERS.get_or_insert_with(profile_id.to_string(), || {
        Ok(Arc::new(IndexManager::new(profile_id)?))
    })
}

/// Prechauffer le cache pour un profil (warmup)
///
/// Utile pour precreeer le manager avant que l'utilisateur en ait besoin,
/// par exemple lors du survol d'un profil dans l'UI.
pub fn warmup_index_manager(profile_id: &str) -> Result<(), AppError> {
    let _ = get_index_manager(profile_id)?;
    Ok(())
}

/// Fermer et retirer un gestionnaire d'indexation du cache
///
/// A appeler lors de la suppression d'un profil pour liberer les ressources.
pub fn close_index_manager(profile_id: &str) {
    INDEX_MANAGERS.remove(&profile_id.to_string());
}

/// Vider tout le cache des gestionnaires d'indexation
///
/// Utile pour la maintenance ou les tests.
pub fn clear_all_index_managers() {
    INDEX_MANAGERS.clear();
}

/// Obtenir le statut du cache des gestionnaires d'indexation
///
/// Retourne les metriques (hits, misses, evictions) et la configuration.
pub fn get_index_cache_status() -> CacheStatus {
    INDEX_MANAGERS.status()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_manager_creation() {
        let test_id = format!("test-idx-{}", uuid::Uuid::new_v4());
        let manager = IndexManager::new(&test_id);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_add_and_remove_object() {
        let test_id = format!("test-idx-{}", uuid::Uuid::new_v4());
        let manager = IndexManager::new(&test_id).unwrap();

        let obj = S3Object {
            key: "test/file.txt".to_string(),
            size: 1024,
            last_modified: Some("2024-01-01T00:00:00Z".to_string()),
            storage_class: Some("STANDARD".to_string()),
            e_tag: Some("abc123".to_string()),
            is_folder: false,
        };

        // Ajouter l'objet
        manager.add_object("test-bucket", &obj).unwrap();

        // Verifier les stats
        let stats = manager.get_prefix_stats("test-bucket", "test/").unwrap();
        assert_eq!(stats.objects_count, 1);
        assert_eq!(stats.total_size, 1024);

        // Supprimer l'objet
        manager
            .remove_object("test-bucket", "test/file.txt")
            .unwrap();

        // Verifier que les stats sont a 0
        let stats = manager.get_prefix_stats("test-bucket", "test/").unwrap();
        assert_eq!(stats.objects_count, 0);
    }

    #[test]
    fn test_bucket_stats() {
        let test_id = format!("test-idx-{}", uuid::Uuid::new_v4());
        let manager = IndexManager::new(&test_id).unwrap();

        // Ajouter plusieurs objets
        for i in 0..10 {
            let obj = S3Object {
                key: format!("data/file{}.txt", i),
                size: 100,
                last_modified: None,
                storage_class: Some("STANDARD".to_string()),
                e_tag: None,
                is_folder: false,
            };
            manager.add_object("stats-bucket", &obj).unwrap();
        }

        // Verifier les stats du bucket
        let stats = manager.get_bucket_stats("stats-bucket").unwrap();
        assert_eq!(stats.total_objects, 10);
        assert_eq!(stats.total_size, 1000);
        assert!(!stats.is_complete); // Pas marque comme complet
    }

    #[test]
    fn test_folder_size_calculation() {
        let test_id = format!("test-idx-{}", uuid::Uuid::new_v4());
        let manager = IndexManager::new(&test_id).unwrap();

        // Ajouter des objets dans differents dossiers
        for i in 0..5 {
            let obj = S3Object {
                key: format!("folder1/file{}.txt", i),
                size: 200,
                last_modified: None,
                storage_class: Some("STANDARD".to_string()),
                e_tag: None,
                is_folder: false,
            };
            manager.add_object("size-bucket", &obj).unwrap();
        }

        for i in 0..3 {
            let obj = S3Object {
                key: format!("folder2/file{}.txt", i),
                size: 300,
                last_modified: None,
                storage_class: Some("STANDARD".to_string()),
                e_tag: None,
                is_folder: false,
            };
            manager.add_object("size-bucket", &obj).unwrap();
        }

        // Verifier la taille de folder1
        let (size1, _) = manager
            .calculate_folder_size("size-bucket", "folder1/")
            .unwrap();
        assert_eq!(size1, 1000); // 5 * 200

        // Verifier la taille de folder2
        let (size2, _) = manager
            .calculate_folder_size("size-bucket", "folder2/")
            .unwrap();
        assert_eq!(size2, 900); // 3 * 300
    }
}
