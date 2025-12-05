//! Module de gestion de la base de donnees SQLite
//!
//! Gere les connexions, migrations et operations CRUD sur l'index.
//! Une base de donnees par profil est creee dans le repertoire de donnees de l'application.
//!
//! Utilise un cache LRU+TTL pour limiter la memoire:
//! - Max 5 profils en cache simultanement
//! - Eviction apres 10 min d'inactivite
//! - TTL max de 1 heure

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension};

use crate::cache_manager::{CacheConfig, CacheStatus, ManagedCache};
use crate::errors::AppError;
use crate::models::{BucketIndexMetadata, BucketInfo, IndexedObject, PrefixStatus, S3Object};

/// Type alias pour le pool de connexions SQLite
pub type DbPool = Pool<SqliteConnectionManager>;
pub type DbConnection = PooledConnection<SqliteConnectionManager>;

/// Version actuelle du schema de base de donnees
const SCHEMA_VERSION: i32 = 1;

/// Gestionnaire de base de donnees pour un profil
pub struct DatabaseManager {
    pool: Arc<DbPool>,
    profile_id: String,
}

impl DatabaseManager {
    /// Creer ou ouvrir une base de donnees pour un profil
    pub fn new(profile_id: &str) -> Result<Self, AppError> {
        let db_path = Self::get_db_path(profile_id)?;

        // Creer le repertoire parent si necessaire
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Creer le gestionnaire de connexions
        let manager = SqliteConnectionManager::file(&db_path);

        // Creer le pool avec configuration optimisee pour limiter la memoire
        // Reduit de 10 a 4 connexions max car on utilise maintenant un cache LRU
        // avec max 5 profils, donc 5 * 4 = 20 connexions max totales
        let pool = Pool::builder()
            .max_size(4) // Max 4 connexions simultanees (reduit de 10)
            .min_idle(Some(1)) // Garder 1 connexion en idle (reduit de 2)
            .idle_timeout(Some(Duration::from_secs(120))) // Fermer connexions idle apres 2 min
            .connection_timeout(Duration::from_secs(5)) // Timeout 5s pour obtenir une connexion
            .build(manager)?;

        let db_manager = Self {
            pool: Arc::new(pool),
            profile_id: profile_id.to_string(),
        };

        // Initialiser le schema
        db_manager.initialize_schema()?;

        Ok(db_manager)
    }

    /// Obtenir le chemin de la base de donnees pour un profil
    pub fn get_db_path(profile_id: &str) -> Result<PathBuf, AppError> {
        let data_dir = dirs::data_local_dir()
            .ok_or_else(|| AppError::ConfigError("Cannot find data directory".to_string()))?;

        // Sanitize profile_id pour eviter les injections de chemin
        let safe_id = profile_id
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect::<String>();

        if safe_id.is_empty() {
            return Err(AppError::ValidationError("Invalid profile ID".to_string()));
        }

        Ok(data_dir
            .join("s3explorer")
            .join("indexes")
            .join(format!("{}.db", safe_id)))
    }

    /// Obtenir la taille du fichier de base de donnees sur le disque (en bytes)
    /// Retourne 0 si le fichier n'existe pas
    pub fn get_db_file_size(profile_id: &str) -> Result<u64, AppError> {
        let db_path = Self::get_db_path(profile_id)?;

        // Verifier si le fichier existe
        if !db_path.exists() {
            return Ok(0);
        }

        // Obtenir la taille du fichier principal
        let main_size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);

        // Ajouter la taille du fichier WAL si present
        let wal_path = db_path.with_extension("db-wal");
        let wal_size = if wal_path.exists() {
            std::fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        // Ajouter la taille du fichier SHM si present
        let shm_path = db_path.with_extension("db-shm");
        let shm_size = if shm_path.exists() {
            std::fs::metadata(&shm_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        Ok(main_size + wal_size + shm_size)
    }

    /// Obtenir une connexion du pool
    pub fn get_connection(&self) -> Result<DbConnection, AppError> {
        self.pool
            .get()
            .map_err(|e| AppError::PoolError(e.to_string()))
    }

    /// Initialiser le schema de la base de donnees
    fn initialize_schema(&self) -> Result<(), AppError> {
        let conn = self.get_connection()?;

        // Activer les optimisations SQLite
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA cache_size = -64000;
             PRAGMA temp_store = MEMORY;",
        )?;

        // Verifier la version du schema
        let current_version = self.get_schema_version(&conn)?;

        if current_version == 0 {
            // Nouvelle base de donnees - creer le schema complet
            self.create_schema(&conn)?;
        } else if current_version < SCHEMA_VERSION {
            // Migration necessaire
            self.migrate_schema(&conn, current_version)?;
        }

        Ok(())
    }

    /// Obtenir la version actuelle du schema
    fn get_schema_version(&self, conn: &Connection) -> Result<i32, AppError> {
        let result: Result<i32, _> =
            conn.query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                row.get(0)
            });

        match result {
            Ok(version) => Ok(version),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(rusqlite::Error::SqliteFailure(_, _)) => Ok(0), // Table n'existe pas
            Err(e) => Err(AppError::DatabaseError(e.to_string())),
        }
    }

    /// Creer le schema initial
    fn create_schema(&self, conn: &Connection) -> Result<(), AppError> {
        conn.execute_batch(include_str!("sql/schema_v1.sql"))?;
        Ok(())
    }

    /// Migrer le schema vers une version plus recente
    fn migrate_schema(&self, conn: &Connection, from_version: i32) -> Result<(), AppError> {
        // Migrations futures seront ajoutees ici
        if from_version < 1 {
            self.create_schema(conn)?;
        }

        // Mettre a jour la version
        conn.execute(
            "UPDATE schema_version SET version = ?, updated_at = ?",
            params![SCHEMA_VERSION, chrono::Utc::now().timestamp_millis()],
        )?;

        Ok(())
    }

    // ========================================================================
    // CRUD Operations - Objects
    // ========================================================================

    /// Inserer ou mettre a jour un objet dans l'index
    pub fn upsert_object(&self, obj: &IndexedObject) -> Result<i64, AppError> {
        let conn = self.get_connection()?;

        // Utiliser INSERT OR REPLACE pour gerer les doublons
        // Les index uniques partiels gerent la contrainte d'unicite
        conn.execute(
            r#"
            INSERT OR REPLACE INTO objects (
                profile_id, bucket_name, key, version_id,
                size, last_modified, e_tag, storage_class,
                owner_id, owner_display_name, checksum_algorithm,
                restore_status, restore_expiry_date,
                content_type, server_side_encryption, sse_kms_key_id,
                parent_prefix, basename, extension, depth, is_folder,
                indexed_at, metadata_loaded
            ) VALUES (
                ?1, ?2, ?3, ?4,
                ?5, ?6, ?7, ?8,
                ?9, ?10, ?11,
                ?12, ?13,
                ?14, ?15, ?16,
                ?17, ?18, ?19, ?20, ?21,
                ?22, ?23
            )
            "#,
            params![
                obj.profile_id,
                obj.bucket_name,
                obj.key,
                obj.version_id,
                obj.size,
                obj.last_modified,
                obj.e_tag,
                obj.storage_class,
                obj.owner_id,
                obj.owner_display_name,
                obj.checksum_algorithm,
                obj.restore_status,
                obj.restore_expiry_date,
                obj.content_type,
                obj.server_side_encryption,
                obj.sse_kms_key_id,
                obj.parent_prefix,
                obj.basename,
                obj.extension,
                obj.depth,
                obj.is_folder,
                obj.indexed_at,
                obj.metadata_loaded,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Inserer ou mettre a jour plusieurs objets en batch
    pub fn upsert_objects_batch(&self, objects: &[IndexedObject]) -> Result<usize, AppError> {
        if objects.is_empty() {
            return Ok(0);
        }

        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;

        let mut count = 0;
        for obj in objects {
            tx.execute(
                r#"
                INSERT OR REPLACE INTO objects (
                    profile_id, bucket_name, key, version_id,
                    size, last_modified, e_tag, storage_class,
                    owner_id, owner_display_name, checksum_algorithm,
                    restore_status, restore_expiry_date,
                    content_type, server_side_encryption, sse_kms_key_id,
                    parent_prefix, basename, extension, depth, is_folder,
                    indexed_at, metadata_loaded
                ) VALUES (
                    ?1, ?2, ?3, ?4,
                    ?5, ?6, ?7, ?8,
                    ?9, ?10, ?11,
                    ?12, ?13,
                    ?14, ?15, ?16,
                    ?17, ?18, ?19, ?20, ?21,
                    ?22, ?23
                )
                "#,
                params![
                    obj.profile_id,
                    obj.bucket_name,
                    obj.key,
                    obj.version_id,
                    obj.size,
                    obj.last_modified,
                    obj.e_tag,
                    obj.storage_class,
                    obj.owner_id,
                    obj.owner_display_name,
                    obj.checksum_algorithm,
                    obj.restore_status,
                    obj.restore_expiry_date,
                    obj.content_type,
                    obj.server_side_encryption,
                    obj.sse_kms_key_id,
                    obj.parent_prefix,
                    obj.basename,
                    obj.extension,
                    obj.depth,
                    obj.is_folder,
                    obj.indexed_at,
                    obj.metadata_loaded,
                ],
            )?;
            count += 1;
        }

        tx.commit()?;
        Ok(count)
    }

    /// Supprimer un objet de l'index
    pub fn delete_object(&self, bucket_name: &str, key: &str) -> Result<bool, AppError> {
        let conn = self.get_connection()?;

        let deleted = conn.execute(
            "DELETE FROM objects WHERE profile_id = ?1 AND bucket_name = ?2 AND key = ?3",
            params![self.profile_id, bucket_name, key],
        )?;

        Ok(deleted > 0)
    }

    /// Supprimer tous les objets d'un prefixe (recursif)
    pub fn delete_objects_by_prefix(
        &self,
        bucket_name: &str,
        prefix: &str,
    ) -> Result<i64, AppError> {
        let conn = self.get_connection()?;

        let deleted = conn.execute(
            "DELETE FROM objects WHERE profile_id = ?1 AND bucket_name = ?2 AND key LIKE ?3",
            params![self.profile_id, bucket_name, format!("{}%", prefix)],
        )?;

        Ok(deleted as i64)
    }

    /// Synchroniser les objets d'un prefix entre l'index et S3
    ///
    /// Supprime de l'index les objets qui ne sont plus dans la liste S3.
    /// Cela permet de nettoyer les "objets fantomes" qui ont ete supprimes
    /// sur S3 par un autre client.
    ///
    /// Note: Cette fonction ne supprime que les objets au niveau exact du prefix
    /// (parent_prefix = prefix), pas les objets dans les sous-dossiers.
    pub fn sync_prefix_objects(
        &self,
        bucket_name: &str,
        prefix: &str,
        current_keys: &[String],
    ) -> Result<i64, AppError> {
        if current_keys.is_empty() {
            // Si pas d'objets dans S3, supprimer tous les objets de ce prefix
            let conn = self.get_connection()?;
            let deleted = conn.execute(
                r#"
                DELETE FROM objects
                WHERE profile_id = ?1
                  AND bucket_name = ?2
                  AND parent_prefix = ?3
                "#,
                params![self.profile_id, bucket_name, prefix],
            )?;
            return Ok(deleted as i64);
        }

        let conn = self.get_connection()?;

        // Utiliser une transaction pour l'atomicite
        let tx = conn.unchecked_transaction()?;

        // Creer une table temporaire avec les cles actuelles
        tx.execute(
            "CREATE TEMP TABLE IF NOT EXISTS sync_current_keys (key TEXT PRIMARY KEY)",
            [],
        )?;
        tx.execute("DELETE FROM sync_current_keys", [])?;

        // Inserer les cles actuelles par batch (optimise: multi-value INSERT)
        // SQLite limite a 999 parametres, on utilise des batches de 500
        const BATCH_SIZE: usize = 500;

        for chunk in current_keys.chunks(BATCH_SIZE) {
            if chunk.is_empty() {
                continue;
            }

            // Construire INSERT avec multiple VALUES: INSERT INTO t VALUES (?1), (?2), ...
            let placeholders: String = (1..=chunk.len())
                .map(|i| format!("(?{})", i))
                .collect::<Vec<_>>()
                .join(",");

            let sql = format!(
                "INSERT OR IGNORE INTO sync_current_keys (key) VALUES {}",
                placeholders
            );

            tx.execute(&sql, rusqlite::params_from_iter(chunk.iter()))?;
        }

        // Supprimer les objets qui sont dans l'index mais pas dans current_keys
        // Limite au prefix exact (parent_prefix = prefix)
        let deleted = tx.execute(
            r#"
            DELETE FROM objects
            WHERE profile_id = ?1
              AND bucket_name = ?2
              AND parent_prefix = ?3
              AND key NOT IN (SELECT key FROM sync_current_keys)
            "#,
            params![self.profile_id, bucket_name, prefix],
        )?;

        tx.execute("DROP TABLE IF EXISTS sync_current_keys", [])?;
        tx.commit()?;

        Ok(deleted as i64)
    }

    /// Recuperer un objet par sa cle
    pub fn get_object(
        &self,
        bucket_name: &str,
        key: &str,
    ) -> Result<Option<IndexedObject>, AppError> {
        let conn = self.get_connection()?;

        let result = conn
            .query_row(
                r#"
            SELECT
                id, profile_id, bucket_name, key, version_id,
                size, last_modified, e_tag, storage_class,
                owner_id, owner_display_name, checksum_algorithm,
                restore_status, restore_expiry_date,
                content_type, server_side_encryption, sse_kms_key_id,
                parent_prefix, basename, extension, depth, is_folder,
                indexed_at, metadata_loaded
            FROM objects
            WHERE profile_id = ?1 AND bucket_name = ?2 AND key = ?3
            "#,
                params![self.profile_id, bucket_name, key],
                |row| Self::row_to_indexed_object(row),
            )
            .optional()?;

        Ok(result)
    }

    /// Convertir une ligne SQLite en IndexedObject
    fn row_to_indexed_object(row: &rusqlite::Row) -> rusqlite::Result<IndexedObject> {
        Ok(IndexedObject {
            id: row.get(0)?,
            profile_id: row.get(1)?,
            bucket_name: row.get(2)?,
            key: row.get(3)?,
            version_id: row.get(4)?,
            size: row.get(5)?,
            last_modified: row.get(6)?,
            e_tag: row.get(7)?,
            storage_class: row.get(8)?,
            owner_id: row.get(9)?,
            owner_display_name: row.get(10)?,
            checksum_algorithm: row.get(11)?,
            restore_status: row.get(12)?,
            restore_expiry_date: row.get(13)?,
            content_type: row.get(14)?,
            server_side_encryption: row.get(15)?,
            sse_kms_key_id: row.get(16)?,
            parent_prefix: row.get(17)?,
            basename: row.get(18)?,
            extension: row.get(19)?,
            depth: row.get(20)?,
            is_folder: row.get(21)?,
            indexed_at: row.get(22)?,
            metadata_loaded: row.get(23)?,
        })
    }

    // ========================================================================
    // CRUD Operations - Prefix Status
    // ========================================================================

    /// Mettre a jour ou creer le statut d'un prefixe (et creer les parents s'ils n'existent pas)
    pub fn upsert_prefix_status(&self, status: &PrefixStatus) -> Result<(), AppError> {
        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;

        // D'abord, creer les entrees pour tous les prefixes parents s'ils n'existent pas
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

    /// Creer les entrees prefix_status pour tous les prefixes parents (s'ils n'existent pas)
    fn ensure_parent_prefixes_exist(
        &self,
        tx: &rusqlite::Transaction,
        bucket_name: &str,
        prefix: &str,
    ) -> Result<(), AppError> {
        let now = chrono::Utc::now().timestamp_millis();
        let mut current = prefix.to_string();

        // Remonter la hierarchie des prefixes
        while let Some(pos) = current.trim_end_matches('/').rfind('/') {
            current = format!("{}/", &current[..pos]);

            // Inserer le parent s'il n'existe pas (avec is_complete = false)
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

    /// Recuperer le statut d'un prefixe
    pub fn get_prefix_status(
        &self,
        bucket_name: &str,
        prefix: &str,
    ) -> Result<Option<PrefixStatus>, AppError> {
        let conn = self.get_connection()?;

        let result = conn
            .query_row(
                r#"
            SELECT
                id, profile_id, bucket_name, prefix,
                is_complete, objects_count, total_size,
                continuation_token, last_indexed_key,
                last_sync_started_at, last_sync_completed_at
            FROM prefix_status
            WHERE profile_id = ?1 AND bucket_name = ?2 AND prefix = ?3
            "#,
                params![self.profile_id, bucket_name, prefix],
                |row| {
                    Ok(PrefixStatus {
                        id: row.get(0)?,
                        profile_id: row.get(1)?,
                        bucket_name: row.get(2)?,
                        prefix: row.get(3)?,
                        is_complete: row.get(4)?,
                        objects_count: row.get(5)?,
                        total_size: row.get(6)?,
                        continuation_token: row.get(7)?,
                        last_indexed_key: row.get(8)?,
                        last_sync_started_at: row.get(9)?,
                        last_sync_completed_at: row.get(10)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// Marquer un prefixe comme incomplet (apres modification)
    pub fn mark_prefix_incomplete(&self, bucket_name: &str, prefix: &str) -> Result<(), AppError> {
        let conn = self.get_connection()?;

        conn.execute(
            r#"
            UPDATE prefix_status
            SET is_complete = FALSE
            WHERE profile_id = ?1 AND bucket_name = ?2 AND prefix = ?3
            "#,
            params![self.profile_id, bucket_name, prefix],
        )?;

        Ok(())
    }

    /// Marquer un prefixe ET tous ses ancetres comme incomplets
    /// Cela garantit la coherence des stats lors d'ajout/suppression d'objets
    ///
    /// OPTIMIZED: Uses a single UPDATE with IN clause instead of N separate UPDATEs.
    /// For a path like "a/b/c/d/", this marks ["a/b/c/d/", "a/b/c/", "a/b/", "a/", ""]
    /// in a single SQL query instead of 5 separate queries (~5x faster).
    pub fn mark_prefix_and_ancestors_incomplete(
        &self,
        bucket_name: &str,
        prefix: &str,
    ) -> Result<(), AppError> {
        let conn = self.get_connection()?;

        // Collecter tous les prefixes ancetres
        let mut prefixes_to_mark: Vec<String> = vec![prefix.to_string()];
        let mut current = prefix.to_string();

        // Remonter la hierarchie des prefixes
        while let Some(pos) = current.trim_end_matches('/').rfind('/') {
            current = format!("{}/", &current[..pos]);
            prefixes_to_mark.push(current.clone());
        }

        // Ajouter le prefix racine (chaine vide)
        if !prefix.is_empty() {
            prefixes_to_mark.push(String::new());
        }

        // Marquer tous les prefixes en une seule requete avec IN clause
        // Construire les placeholders: ?3, ?4, ?5, ... (1 et 2 sont profile_id et bucket_name)
        let placeholders: String = (0..prefixes_to_mark.len())
            .map(|i| format!("?{}", i + 3))
            .collect::<Vec<_>>()
            .join(",");

        let sql = format!(
            r#"
            UPDATE prefix_status
            SET is_complete = FALSE
            WHERE profile_id = ?1 AND bucket_name = ?2 AND prefix IN ({})
            "#,
            placeholders
        );

        // Construire les paramètres: [profile_id, bucket_name, prefix1, prefix2, ...]
        let mut params_vec: Vec<&dyn rusqlite::ToSql> =
            Vec::with_capacity(2 + prefixes_to_mark.len());
        params_vec.push(&self.profile_id);
        params_vec.push(&bucket_name);
        for pfx in &prefixes_to_mark {
            params_vec.push(pfx);
        }

        conn.execute(&sql, params_vec.as_slice())?;

        Ok(())
    }

    /// Supprimer le statut d'un prefixe
    pub fn delete_prefix_status(&self, bucket_name: &str, prefix: &str) -> Result<(), AppError> {
        let conn = self.get_connection()?;

        conn.execute(
            "DELETE FROM prefix_status WHERE profile_id = ?1 AND bucket_name = ?2 AND prefix = ?3",
            params![self.profile_id, bucket_name, prefix],
        )?;

        Ok(())
    }

    /// Nettoyer les entrees prefix_status orphelines
    ///
    /// Supprime les entrees prefix_status qui n'ont plus d'objets correspondants
    /// dans la table objects. Cela permet de nettoyer les dossiers supprimes
    /// sur S3 par un autre client.
    ///
    /// Note: Ne supprime pas le prefix racine (prefix = '') pour preserver les stats du bucket.
    pub fn cleanup_orphan_prefix_status(&self, bucket_name: &str) -> Result<i64, AppError> {
        let conn = self.get_connection()?;

        let deleted = conn.execute(
            r#"
            DELETE FROM prefix_status
            WHERE profile_id = ?1
              AND bucket_name = ?2
              AND prefix != ''
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

    // ========================================================================
    // Statistics Queries
    // ========================================================================

    /// Calculer les statistiques d'un prefixe depuis l'index (pour navigation individuelle)
    pub fn calculate_prefix_stats(
        &self,
        bucket_name: &str,
        prefix: &str,
    ) -> Result<(i64, i64), AppError> {
        let conn = self.get_connection()?;

        let (count, size): (i64, i64) = conn.query_row(
            r#"
            SELECT
                COUNT(*) as count,
                COALESCE(SUM(size), 0) as total_size
            FROM objects
            WHERE profile_id = ?1
              AND bucket_name = ?2
              AND key LIKE ?3
              AND is_folder = FALSE
            "#,
            params![self.profile_id, bucket_name, format!("{}%", prefix)],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        Ok((count, size))
    }

    /// Calculer les statistiques de TOUS les préfixes en une seule requête (optimisation N+1)
    ///
    /// Au lieu de faire N requêtes (une par préfixe), cette fonction fait une seule
    /// requête avec GROUP BY et retourne toutes les stats d'un coup.
    ///
    /// Performance: 50,000 préfixes en ~2 secondes au lieu de ~4 minutes
    pub fn calculate_all_prefix_stats_batch(
        &self,
        bucket_name: &str,
    ) -> Result<std::collections::HashMap<String, (i64, i64)>, AppError> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT
                parent_prefix,
                COUNT(*) as count,
                COALESCE(SUM(size), 0) as total_size
            FROM objects
            WHERE profile_id = ?1
              AND bucket_name = ?2
              AND parent_prefix != ''
              AND is_folder = FALSE
            GROUP BY parent_prefix
            "#,
        )?;

        let stats = stmt
            .query_map(params![self.profile_id, bucket_name], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    (row.get::<_, i64>(1)?, row.get::<_, i64>(2)?),
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(stats)
    }

    /// Insérer/mettre à jour plusieurs PrefixStatus en une seule transaction (batch upsert)
    ///
    /// Beaucoup plus efficace que des upserts individuels car:
    /// - Une seule transaction (pas de fsync entre chaque insert)
    /// - Statement préparé réutilisé pour tous les inserts
    ///
    /// Performance: 50,000 upserts en ~1 seconde au lieu de ~30 secondes
    pub fn batch_upsert_prefix_status(&self, statuses: &[PrefixStatus]) -> Result<(), AppError> {
        if statuses.is_empty() {
            return Ok(());
        }

        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;

        {
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO prefix_status (
                    profile_id, bucket_name, prefix,
                    is_complete, objects_count, total_size,
                    continuation_token, last_indexed_key,
                    last_sync_started_at, last_sync_completed_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                ON CONFLICT (profile_id, bucket_name, prefix) DO UPDATE SET
                    is_complete = excluded.is_complete,
                    objects_count = excluded.objects_count,
                    total_size = excluded.total_size,
                    continuation_token = excluded.continuation_token,
                    last_indexed_key = excluded.last_indexed_key,
                    last_sync_started_at = excluded.last_sync_started_at,
                    last_sync_completed_at = excluded.last_sync_completed_at
                "#,
            )?;

            for status in statuses {
                stmt.execute(params![
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
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Calculer les statistiques du bucket entier
    pub fn calculate_bucket_stats(&self, bucket_name: &str) -> Result<(i64, i64, bool), AppError> {
        let conn = self.get_connection()?;

        let (count, size): (i64, i64) = conn.query_row(
            r#"
            SELECT
                COUNT(*) as count,
                COALESCE(SUM(size), 0) as total_size
            FROM objects
            WHERE profile_id = ?1 AND bucket_name = ?2 AND is_folder = FALSE
            "#,
            params![self.profile_id, bucket_name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        // Verifier si le bucket est complet en utilisant bucket_info.initial_index_completed
        // Note: On ne peut pas utiliser prefix_status.is_complete pour le prefix racine car
        // il est mis a jour lors de la navigation avec delimiter et ne reflète pas
        // le statut d'indexation complet du bucket.
        let is_complete: bool = conn
            .query_row(
                r#"
            SELECT COALESCE(initial_index_completed, FALSE)
            FROM bucket_info
            WHERE profile_id = ?1 AND bucket_name = ?2
            "#,
                params![self.profile_id, bucket_name],
                |row| row.get(0),
            )
            .unwrap_or(false);

        Ok((count, size, is_complete))
    }

    /// Calculer la taille estimee de l'index pour un bucket specifique
    /// Utilise la meme formule que get_all_bucket_indexes:
    /// - ~200 bytes overhead per row (SQLite row structure + B-tree overhead)
    /// - Plus actual data lengths (key, e_tag, storage_class, parent_prefix, basename)
    pub fn calculate_bucket_index_size(&self, bucket_name: &str) -> Result<i64, AppError> {
        let conn = self.get_connection()?;

        let size: i64 = conn
            .query_row(
                r#"
                SELECT COALESCE(
                    COUNT(*) * 200 +
                    SUM(LENGTH(key)) +
                    SUM(LENGTH(COALESCE(e_tag, ''))) +
                    SUM(LENGTH(COALESCE(storage_class, ''))) +
                    SUM(LENGTH(COALESCE(parent_prefix, ''))) +
                    SUM(LENGTH(COALESCE(basename, '')))
                , 0)
                FROM objects
                WHERE profile_id = ?1 AND bucket_name = ?2
                "#,
                params![self.profile_id, bucket_name],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(size)
    }

    /// Obtenir les statistiques par classe de stockage
    pub fn get_storage_class_stats(
        &self,
        bucket_name: &str,
    ) -> Result<Vec<(String, i64, i64)>, AppError> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT
                storage_class,
                COUNT(*) as count,
                SUM(size) as total_size
            FROM objects
            WHERE profile_id = ?1 AND bucket_name = ?2 AND is_folder = FALSE
            GROUP BY storage_class
            ORDER BY total_size DESC
            "#,
        )?;

        let rows = stmt.query_map(params![self.profile_id, bucket_name], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// Verifier si un prefixe est marque complet (sans verifier les enfants)
    /// Utilise pour l'affichage rapide
    pub fn is_prefix_self_complete(
        &self,
        bucket_name: &str,
        prefix: &str,
    ) -> Result<bool, AppError> {
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

    /// Verifier si un prefixe est complet (incluant tous ses sous-prefixes)
    ///
    /// Un prefixe est complet si:
    /// 1. Le bucket entier a ete indexe completement (initial_index_completed = true)
    /// OU
    /// 2. L'indexation partielle a depasse ce prefixe (last_indexed_key > prefix)
    ///    Ce qui signifie que tous les objets de ce prefixe ont ete indexes
    /// OU
    /// 3. Le prefix lui-meme est marque complet dans prefix_status
    ///    ET tous ses sous-prefixes sont aussi complets
    ///    ET tous les objets indexes ont leur parent_prefix dans prefix_status
    pub fn is_prefix_complete(&self, bucket_name: &str, prefix: &str) -> Result<bool, AppError> {
        let conn = self.get_connection()?;

        // Raccourci 1: si le bucket entier a ete indexe, tous les prefixes sont complets
        let bucket_fully_indexed: bool = conn
            .query_row(
                r#"
                SELECT COALESCE(initial_index_completed, FALSE)
                FROM bucket_info
                WHERE profile_id = ?1 AND bucket_name = ?2
                "#,
                params![self.profile_id, bucket_name],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if bucket_fully_indexed {
            return Ok(true);
        }

        // Raccourci 2: Si l'indexation partielle a depasse ce prefixe, il est complet
        // Les objets S3 sont retournes en ordre alphabetique, donc si last_indexed_key
        // commence par un prefixe alphabetiquement superieur, notre prefixe est complet.
        // Ex: prefix = "archive/", last_indexed_key = "data/file.txt"
        //     -> "data/" > "archive/" donc "archive/" est complet
        if !prefix.is_empty() {
            let last_indexed_key: Option<String> = conn
                .query_row(
                    r#"
                    SELECT last_indexed_key
                    FROM prefix_status
                    WHERE profile_id = ?1 AND bucket_name = ?2 AND prefix = ''
                    "#,
                    params![self.profile_id, bucket_name],
                    |row| row.get(0),
                )
                .optional()?
                .flatten();

            if let Some(last_key) = last_indexed_key {
                // Comparer alphabetiquement: si le dernier objet indexe est apres notre prefixe,
                // alors tous les objets de notre prefixe ont ete indexes
                // Note: on compare avec le prefixe sans le "/" final pour la comparaison correcte
                let prefix_for_compare = prefix.trim_end_matches('/');

                // Extraire le premier segment du last_key pour comparaison
                // Ex: "data/2024/file.txt" -> "data"
                let last_key_first_segment = last_key.split('/').next().unwrap_or(&last_key);

                if last_key_first_segment > prefix_for_compare {
                    return Ok(true);
                }

                // Si on est dans le meme prefixe de premier niveau, verifier plus en detail
                // Ex: prefix = "data/2023/", last_key = "data/2024/file.txt"
                // On doit verifier que last_key est strictement apres prefix
                if last_key.as_str() > prefix {
                    // Le dernier objet indexe est apres notre prefixe
                    // Mais on doit aussi verifier qu'il n'est pas DANS notre prefixe
                    // (sinon le prefixe pourrait etre partiellement indexe)
                    if !last_key.starts_with(prefix) {
                        return Ok(true);
                    }
                }
            }
        }

        // 1. Verifier si le prefix lui-meme existe et est marque complet
        let self_status: Option<bool> = conn
            .query_row(
                r#"
                SELECT is_complete
                FROM prefix_status
                WHERE profile_id = ?1 AND bucket_name = ?2 AND prefix = ?3
                "#,
                params![self.profile_id, bucket_name, prefix],
                |row| row.get(0),
            )
            .optional()?;

        match self_status {
            None => {
                // Pas d'entree dans prefix_status = jamais explore = incomplet
                return Ok(false);
            }
            Some(is_complete) if !is_complete => {
                return Ok(false);
            }
            _ => {}
        }

        // 2. Verifier qu'il n'y a pas d'enfants incomplets dans prefix_status
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
            .unwrap_or(true); // En cas d'erreur, considerer incomplet

        if has_incomplete_children {
            return Ok(false);
        }

        // 3. Verifier si des objets indexes ont des parent_prefix non explores
        // Exemple: si on a indexe "folder/sub/file.txt" mais "folder/sub/" n'existe pas
        // dans prefix_status, cela signifie qu'on a indexe sans delimiter mais qu'on n'a
        // pas encore explore ce sous-dossier
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
            .unwrap_or(true); // En cas d'erreur, considerer incomplet

        Ok(!has_unexplored_subprefixes)
    }

    // ========================================================================
    // Search Operations
    // ========================================================================

    /// Rechercher des objets par pattern dans la clé (LIKE query, case-insensitive)
    /// Si limit est None, retourne tous les résultats (pas de limite)
    pub fn search_objects(
        &self,
        bucket_name: &str,
        query: &str,
        prefix: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<S3Object>, AppError> {
        let conn = self.get_connection()?;

        // Build the search pattern - case insensitive using LIKE
        let search_pattern = format!("%{}%", query.to_lowercase());

        // Build SQL query - only add LIMIT if specified
        let limit_clause = match limit {
            Some(l) => format!("LIMIT {}", l),
            None => String::new(), // No limit
        };

        let sql = if prefix.is_some() {
            format!(
                r#"
                SELECT key, size, last_modified, storage_class, e_tag, is_folder
                FROM objects
                WHERE profile_id = ?1
                  AND bucket_name = ?2
                  AND LOWER(key) LIKE ?3
                  AND key LIKE ?4
                ORDER BY key
                {}
                "#,
                limit_clause
            )
        } else {
            format!(
                r#"
                SELECT key, size, last_modified, storage_class, e_tag, is_folder
                FROM objects
                WHERE profile_id = ?1
                  AND bucket_name = ?2
                  AND LOWER(key) LIKE ?3
                ORDER BY key
                {}
                "#,
                limit_clause
            )
        };

        let mut stmt = conn.prepare(&sql)?;

        let results: Vec<S3Object> = if let Some(pfx) = prefix {
            let prefix_pattern = format!("{}%", pfx);
            stmt.query_map(
                params![self.profile_id, bucket_name, search_pattern, prefix_pattern],
                |row| {
                    Ok(S3Object {
                        key: row.get(0)?,
                        size: row.get(1)?,
                        last_modified: row.get(2)?,
                        storage_class: row.get(3)?,
                        e_tag: row.get(4)?,
                        is_folder: row.get(5)?,
                    })
                },
            )?
            .filter_map(Result::ok)
            .collect()
        } else {
            stmt.query_map(
                params![self.profile_id, bucket_name, search_pattern],
                |row| {
                    Ok(S3Object {
                        key: row.get(0)?,
                        size: row.get(1)?,
                        last_modified: row.get(2)?,
                        storage_class: row.get(3)?,
                        e_tag: row.get(4)?,
                        is_folder: row.get(5)?,
                    })
                },
            )?
            .filter_map(Result::ok)
            .collect()
        };

        Ok(results)
    }

    /// Obtenir tous les buckets indexés avec leurs métadonnées
    ///
    /// OPTIMIZED: Uses a single scan of objects table with GROUP BY instead of
    /// 3 correlated subqueries per bucket. This reduces query time from O(N*M)
    /// to O(M) where N = number of buckets and M = total objects.
    ///
    /// Performance improvement: ~30x faster for 10 buckets with 1M objects each
    pub fn get_all_bucket_indexes(&self) -> Result<Vec<BucketIndexMetadata>, AppError> {
        let conn = self.get_connection()?;

        // Single scan of objects table with GROUP BY, then LEFT JOIN with bucket_info
        // This replaces 3 correlated subqueries per bucket with 1 aggregation pass
        //
        // Estimated index size calculation:
        // - ~200 bytes overhead per row (SQLite row structure + B-tree overhead)
        // - Plus actual data lengths (key, e_tag, storage_class, parent_prefix, basename)
        let mut stmt = conn.prepare(
            r#"
            SELECT
                bi.bucket_name,
                COALESCE(stats.total_objects, 0) as total_objects,
                COALESCE(stats.total_size, 0) as total_size,
                bi.initial_index_completed as is_complete,
                bi.last_checked_at as last_indexed_at,
                COALESCE(stats.estimated_index_size, 0) as estimated_index_size
            FROM bucket_info bi
            LEFT JOIN (
                SELECT
                    bucket_name,
                    SUM(CASE WHEN is_folder = 0 THEN 1 ELSE 0 END) as total_objects,
                    SUM(CASE WHEN is_folder = 0 THEN size ELSE 0 END) as total_size,
                    COUNT(*) * 200 +
                    SUM(LENGTH(key)) +
                    SUM(LENGTH(COALESCE(e_tag, ''))) +
                    SUM(LENGTH(COALESCE(storage_class, ''))) +
                    SUM(LENGTH(COALESCE(parent_prefix, ''))) +
                    SUM(LENGTH(COALESCE(basename, ''))) as estimated_index_size
                FROM objects
                WHERE profile_id = ?1
                GROUP BY bucket_name
            ) stats ON stats.bucket_name = bi.bucket_name
            WHERE bi.profile_id = ?1
            ORDER BY bi.bucket_name
            "#,
        )?;

        let results: Vec<BucketIndexMetadata> = stmt
            .query_map(params![self.profile_id], |row| {
                Ok(BucketIndexMetadata {
                    bucket_name: row.get(0)?,
                    total_objects: row.get(1)?,
                    total_size: row.get(2)?,
                    is_complete: row.get(3)?,
                    last_indexed_at: row.get(4)?,
                    estimated_index_size: row.get(5)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(results)
    }

    // ========================================================================
    // Bucket Info
    // ========================================================================

    /// Mettre a jour les informations d'un bucket
    pub fn upsert_bucket_info(&self, info: &BucketInfo) -> Result<(), AppError> {
        let conn = self.get_connection()?;

        conn.execute(
            r#"
            INSERT INTO bucket_info (
                profile_id, bucket_name,
                versioning_enabled, encryption_enabled, default_encryption,
                acl, acl_cached_at,
                region,
                initial_index_requests, initial_index_completed,
                last_checked_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(profile_id, bucket_name)
            DO UPDATE SET
                versioning_enabled = COALESCE(excluded.versioning_enabled, versioning_enabled),
                encryption_enabled = COALESCE(excluded.encryption_enabled, encryption_enabled),
                default_encryption = COALESCE(excluded.default_encryption, default_encryption),
                acl = COALESCE(excluded.acl, acl),
                acl_cached_at = COALESCE(excluded.acl_cached_at, acl_cached_at),
                region = COALESCE(excluded.region, region),
                initial_index_requests = COALESCE(excluded.initial_index_requests, initial_index_requests),
                initial_index_completed = COALESCE(excluded.initial_index_completed, initial_index_completed),
                last_checked_at = excluded.last_checked_at
            "#,
            params![
                info.profile_id,
                info.bucket_name,
                info.versioning_enabled,
                info.encryption_enabled,
                info.default_encryption,
                info.acl,
                info.acl_cached_at,
                info.region,
                info.initial_index_requests,
                info.initial_index_completed,
                info.last_checked_at,
            ],
        )?;

        Ok(())
    }

    /// Recuperer les informations d'un bucket
    pub fn get_bucket_info(&self, bucket_name: &str) -> Result<Option<BucketInfo>, AppError> {
        let conn = self.get_connection()?;

        let result = conn
            .query_row(
                r#"
            SELECT
                id, profile_id, bucket_name,
                versioning_enabled, encryption_enabled, default_encryption,
                acl, acl_cached_at,
                region,
                initial_index_requests, initial_index_completed,
                last_checked_at
            FROM bucket_info
            WHERE profile_id = ?1 AND bucket_name = ?2
            "#,
                params![self.profile_id, bucket_name],
                |row| {
                    Ok(BucketInfo {
                        id: row.get(0)?,
                        profile_id: row.get(1)?,
                        bucket_name: row.get(2)?,
                        versioning_enabled: row.get(3)?,
                        encryption_enabled: row.get(4)?,
                        default_encryption: row.get(5)?,
                        acl: row.get(6)?,
                        acl_cached_at: row.get(7)?,
                        region: row.get(8)?,
                        initial_index_requests: row.get(9)?,
                        initial_index_completed: row.get(10)?,
                        last_checked_at: row.get(11)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    // ========================================================================
    // Maintenance
    // ========================================================================

    /// Purger les objets obsoletes (plus vieux que stale_hours)
    pub fn purge_stale_objects(
        &self,
        bucket_name: &str,
        stale_hours: u32,
    ) -> Result<i64, AppError> {
        let conn = self.get_connection()?;

        let cutoff = chrono::Utc::now().timestamp_millis() - (stale_hours as i64 * 60 * 60 * 1000);

        let deleted = conn.execute(
            r#"
            DELETE FROM objects
            WHERE profile_id = ?1
              AND bucket_name = ?2
              AND indexed_at < ?3
            "#,
            params![self.profile_id, bucket_name, cutoff],
        )?;

        Ok(deleted as i64)
    }

    /// Vider tout l'index d'un bucket
    pub fn clear_bucket_index(&self, bucket_name: &str) -> Result<(), AppError> {
        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;

        tx.execute(
            "DELETE FROM objects WHERE profile_id = ?1 AND bucket_name = ?2",
            params![self.profile_id, bucket_name],
        )?;

        tx.execute(
            "DELETE FROM prefix_status WHERE profile_id = ?1 AND bucket_name = ?2",
            params![self.profile_id, bucket_name],
        )?;

        tx.execute(
            "DELETE FROM bucket_info WHERE profile_id = ?1 AND bucket_name = ?2",
            params![self.profile_id, bucket_name],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Optimiser la base de donnees (VACUUM)
    pub fn optimize(&self) -> Result<(), AppError> {
        let conn = self.get_connection()?;
        conn.execute_batch("VACUUM; ANALYZE;")?;
        Ok(())
    }

    /// Obtenir le profile_id
    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }
}

// ============================================================================
// Global Database Pool Manager with LRU + TTL Cache
// ============================================================================

lazy_static::lazy_static! {
    /// Cache LRU+TTL des gestionnaires de base de donnees
    ///
    /// Configuration:
    /// - Max 5 profils en cache (LRU eviction au-dela)
    /// - Eviction apres 10 min d'inactivite
    /// - TTL max de 1 heure
    static ref DB_MANAGERS: ManagedCache<String, Arc<DatabaseManager>> = {
        ManagedCache::new(
            "DatabaseManagers",
            CacheConfig {
                max_entries: 5,           // Max 5 profils en cache
                idle_timeout_secs: 600,   // 10 minutes d'inactivite
                ttl_secs: Some(3600),     // 1 heure max
            },
        )
    };
}

/// Obtenir ou creer un gestionnaire de base de donnees pour un profil
///
/// Utilise un cache LRU+TTL pour limiter la memoire.
/// Si le cache est plein, le profil le moins recemment utilise est evince.
pub fn get_db_manager(profile_id: &str) -> Result<Arc<DatabaseManager>, AppError> {
    DB_MANAGERS.get_or_insert_with(profile_id.to_string(), || {
        Ok(Arc::new(DatabaseManager::new(profile_id)?))
    })
}

/// Prechauffer le cache pour un profil (warmup)
///
/// Utile pour precreeer le manager avant que l'utilisateur en ait besoin,
/// par exemple lors du survol d'un profil dans l'UI.
pub fn warmup_db_manager(profile_id: &str) -> Result<(), AppError> {
    let _ = get_db_manager(profile_id)?;
    Ok(())
}

/// Fermer et retirer un gestionnaire de base de donnees du cache
///
/// A appeler lors de la suppression d'un profil pour liberer les ressources.
pub fn close_db_manager(profile_id: &str) {
    DB_MANAGERS.remove(&profile_id.to_string());
}

/// Vider tout le cache des gestionnaires de base de donnees
///
/// Utile pour la maintenance ou les tests.
pub fn clear_all_db_managers() {
    DB_MANAGERS.clear();
}

/// Obtenir le statut du cache des gestionnaires de base de donnees
///
/// Retourne les metriques (hits, misses, evictions) et la configuration.
pub fn get_db_cache_status() -> CacheStatus {
    DB_MANAGERS.status()
}

/// Verifier si un profil est en cache
pub fn is_db_manager_cached(profile_id: &str) -> bool {
    DB_MANAGERS.contains(&profile_id.to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_db() -> Result<DatabaseManager, AppError> {
        // Utiliser un ID unique pour chaque test
        let test_id = format!("test-{}", uuid::Uuid::new_v4());
        DatabaseManager::new(&test_id)
    }

    #[test]
    fn test_db_creation() {
        let db = create_test_db();
        assert!(db.is_ok());
    }

    #[test]
    fn test_upsert_and_get_object() {
        let db = create_test_db().unwrap();

        let obj = IndexedObject {
            id: None,
            profile_id: db.profile_id().to_string(),
            bucket_name: "test-bucket".to_string(),
            key: "folder/file.txt".to_string(),
            version_id: None,
            size: 1024,
            last_modified: Some("2024-01-01T00:00:00Z".to_string()),
            e_tag: Some("abc123".to_string()),
            storage_class: "STANDARD".to_string(),
            owner_id: None,
            owner_display_name: None,
            checksum_algorithm: None,
            restore_status: None,
            restore_expiry_date: None,
            content_type: None,
            server_side_encryption: None,
            sse_kms_key_id: None,
            parent_prefix: "folder/".to_string(),
            basename: "file.txt".to_string(),
            extension: Some("txt".to_string()),
            depth: 1,
            is_folder: false,
            indexed_at: chrono::Utc::now().timestamp_millis(),
            metadata_loaded: false,
        };

        db.upsert_object(&obj).unwrap();

        let retrieved = db.get_object("test-bucket", "folder/file.txt").unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.size, 1024);
        assert_eq!(retrieved.basename, "file.txt");
    }

    #[test]
    fn test_calculate_prefix_stats() {
        let db = create_test_db().unwrap();
        let profile_id = db.profile_id().to_string();

        // Inserer quelques objets
        for i in 0..5 {
            let obj = IndexedObject {
                id: None,
                profile_id: profile_id.clone(),
                bucket_name: "test-bucket".to_string(),
                key: format!("data/file{}.txt", i),
                version_id: None,
                size: 100,
                last_modified: None,
                e_tag: None,
                storage_class: "STANDARD".to_string(),
                owner_id: None,
                owner_display_name: None,
                checksum_algorithm: None,
                restore_status: None,
                restore_expiry_date: None,
                content_type: None,
                server_side_encryption: None,
                sse_kms_key_id: None,
                parent_prefix: "data/".to_string(),
                basename: format!("file{}.txt", i),
                extension: Some("txt".to_string()),
                depth: 1,
                is_folder: false,
                indexed_at: chrono::Utc::now().timestamp_millis(),
                metadata_loaded: false,
            };
            db.upsert_object(&obj).unwrap();
        }

        let (count, size) = db.calculate_prefix_stats("test-bucket", "data/").unwrap();
        assert_eq!(count, 5);
        assert_eq!(size, 500);
    }

    #[test]
    fn test_batch_upsert() {
        let db = create_test_db().unwrap();
        let profile_id = db.profile_id().to_string();

        let objects: Vec<IndexedObject> = (0..100)
            .map(|i| IndexedObject {
                id: None,
                profile_id: profile_id.clone(),
                bucket_name: "test-bucket".to_string(),
                key: format!("batch/file{}.txt", i),
                version_id: None,
                size: 50,
                last_modified: None,
                e_tag: None,
                storage_class: "STANDARD".to_string(),
                owner_id: None,
                owner_display_name: None,
                checksum_algorithm: None,
                restore_status: None,
                restore_expiry_date: None,
                content_type: None,
                server_side_encryption: None,
                sse_kms_key_id: None,
                parent_prefix: "batch/".to_string(),
                basename: format!("file{}.txt", i),
                extension: Some("txt".to_string()),
                depth: 1,
                is_folder: false,
                indexed_at: chrono::Utc::now().timestamp_millis(),
                metadata_loaded: false,
            })
            .collect();

        let count = db.upsert_objects_batch(&objects).unwrap();
        assert_eq!(count, 100);

        let (obj_count, total_size) = db.calculate_prefix_stats("test-bucket", "batch/").unwrap();
        assert_eq!(obj_count, 100);
        assert_eq!(total_size, 5000);
    }

    #[test]
    fn test_prefix_status() {
        let db = create_test_db().unwrap();
        let profile_id = db.profile_id().to_string();

        let status = PrefixStatus {
            id: None,
            profile_id: profile_id.clone(),
            bucket_name: "test-bucket".to_string(),
            prefix: "data/".to_string(),
            is_complete: true,
            objects_count: 100,
            total_size: 50000,
            continuation_token: None,
            last_indexed_key: Some("data/file99.txt".to_string()),
            last_sync_started_at: Some(chrono::Utc::now().timestamp_millis()),
            last_sync_completed_at: Some(chrono::Utc::now().timestamp_millis()),
        };

        db.upsert_prefix_status(&status).unwrap();

        let retrieved = db.get_prefix_status("test-bucket", "data/").unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert!(retrieved.is_complete);
        assert_eq!(retrieved.objects_count, 100);
    }
}
