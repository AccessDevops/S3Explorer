# Plan: Bucket Settings Modal (Read-Only)

## Objectif
Ajouter un bouton settings sur chaque card de bucket (sur la meme ligne que l'icon delete) qui ouvre un modal avec 6 onglets en lecture seule:
1. Bucket Policy
2. ACL
3. CORS
4. Lifecycle Rules
5. Versioning Status
6. Encryption Config

---

## Phase 1: Backend Rust - Nouvelles API S3

### 1.1 Ajouter les types dans `models.rs`

```rust
// ============================================================================
// Bucket Configuration Types (Read-Only)
// ============================================================================

/// Bucket Policy (JSON document)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketPolicyResponse {
    pub policy: Option<String>, // Raw JSON policy or null if no policy
    pub error: Option<String>,  // Error message if access denied
}

/// CORS Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsRule {
    pub allowed_headers: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_origins: Vec<String>,
    pub expose_headers: Vec<String>,
    pub max_age_seconds: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketCorsResponse {
    pub rules: Vec<CorsRule>,
    pub error: Option<String>,
}

/// Lifecycle Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleRule {
    pub id: Option<String>,
    pub status: String, // "Enabled" or "Disabled"
    pub filter_prefix: Option<String>,
    pub expiration_days: Option<i32>,
    pub expiration_date: Option<String>,
    pub transitions: Vec<LifecycleTransition>,
    pub noncurrent_version_expiration_days: Option<i32>,
    pub abort_incomplete_multipart_days: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleTransition {
    pub days: Option<i32>,
    pub date: Option<String>,
    pub storage_class: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketLifecycleResponse {
    pub rules: Vec<LifecycleRule>,
    pub error: Option<String>,
}

/// Versioning Status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketVersioningResponse {
    pub status: Option<String>, // "Enabled", "Suspended", or null (never enabled)
    pub mfa_delete: Option<String>, // "Enabled" or "Disabled"
    pub error: Option<String>,
}

/// Encryption Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketEncryptionRule {
    pub sse_algorithm: String, // "AES256" or "aws:kms"
    pub kms_master_key_id: Option<String>,
    pub bucket_key_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketEncryptionResponse {
    pub rules: Vec<BucketEncryptionRule>,
    pub error: Option<String>,
}

/// Complete bucket configuration (all settings)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketConfigurationResponse {
    pub policy: BucketPolicyResponse,
    pub acl: String, // Existing: "Public" or "Private"
    pub cors: BucketCorsResponse,
    pub lifecycle: BucketLifecycleResponse,
    pub versioning: BucketVersioningResponse,
    pub encryption: BucketEncryptionResponse,
}
```

### 1.2 Nouvelles méthodes dans `s3_adapter.rs`

```rust
/// Get bucket policy (JSON document)
pub async fn get_bucket_policy(&self, bucket_name: &str) -> BucketPolicyResponse {
    match self.client.get_bucket_policy().bucket(bucket_name).send().await {
        Ok(output) => BucketPolicyResponse {
            policy: output.policy,
            error: None,
        },
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("NoSuchBucketPolicy") {
                BucketPolicyResponse { policy: None, error: None }
            } else {
                BucketPolicyResponse { policy: None, error: Some(err_str) }
            }
        }
    }
}

/// Get bucket CORS configuration
pub async fn get_bucket_cors(&self, bucket_name: &str) -> BucketCorsResponse { ... }

/// Get bucket lifecycle configuration
pub async fn get_bucket_lifecycle(&self, bucket_name: &str) -> BucketLifecycleResponse { ... }

/// Get bucket versioning status
pub async fn get_bucket_versioning(&self, bucket_name: &str) -> BucketVersioningResponse { ... }

/// Get bucket encryption configuration
pub async fn get_bucket_encryption(&self, bucket_name: &str) -> BucketEncryptionResponse { ... }

/// Get all bucket configuration at once (parallel calls)
pub async fn get_bucket_configuration(&self, bucket_name: &str) -> BucketConfigurationResponse { ... }
```

### 1.3 Nouvelles commandes Tauri dans `commands.rs`

```rust
#[tauri::command]
pub async fn get_bucket_configuration(
    profile_id: String,
    bucket_name: String,
    cache: State<'_, CacheManager>,
    profiles_state: State<'_, ProfilesState>,
) -> Result<BucketConfigurationResponse, String> { ... }
```

### 1.4 Enregistrer la commande dans `main.rs`

Ajouter `get_bucket_configuration` au `.invoke_handler()`.

---

## Phase 2: Frontend - Service Tauri

### 2.1 Types TypeScript dans `types/index.ts`

```typescript
// Bucket Configuration Types
export interface BucketPolicyResponse {
  policy: string | null
  error: string | null
}

export interface CorsRule {
  allowed_headers: string[]
  allowed_methods: string[]
  allowed_origins: string[]
  expose_headers: string[]
  max_age_seconds: number | null
}

export interface BucketCorsResponse {
  rules: CorsRule[]
  error: string | null
}

export interface LifecycleTransition {
  days: number | null
  date: string | null
  storage_class: string
}

export interface LifecycleRule {
  id: string | null
  status: string
  filter_prefix: string | null
  expiration_days: number | null
  expiration_date: string | null
  transitions: LifecycleTransition[]
  noncurrent_version_expiration_days: number | null
  abort_incomplete_multipart_days: number | null
}

export interface BucketLifecycleResponse {
  rules: LifecycleRule[]
  error: string | null
}

export interface BucketVersioningResponse {
  status: string | null  // "Enabled" | "Suspended" | null
  mfa_delete: string | null
  error: string | null
}

export interface BucketEncryptionRule {
  sse_algorithm: string
  kms_master_key_id: string | null
  bucket_key_enabled: boolean | null
}

export interface BucketEncryptionResponse {
  rules: BucketEncryptionRule[]
  error: string | null
}

export interface BucketConfigurationResponse {
  policy: BucketPolicyResponse
  acl: string
  cors: BucketCorsResponse
  lifecycle: BucketLifecycleResponse
  versioning: BucketVersioningResponse
  encryption: BucketEncryptionResponse
}
```

### 2.2 Wrapper Tauri dans `services/tauri.ts`

```typescript
export async function getBucketConfiguration(
  profileId: string,
  bucketName: string
): Promise<BucketConfigurationResponse> {
  return await invoke('get_bucket_configuration', { profileId, bucketName })
}
```

---

## Phase 3: Frontend - Composant BucketSettingsModal

### 3.1 Nouveau composant `BucketSettingsModal.vue`

Structure:
- Dialog modal avec 6 onglets (pattern identique à SettingsButton.vue)
- Chaque onglet affiche les données en lecture seule
- Indicateur de chargement pendant le fetch
- Gestion des erreurs (Access Denied, etc.)

```vue
<template>
  <Dialog v-model:open="isOpen">
    <DialogContent class="max-w-3xl max-h-[85vh] overflow-hidden flex flex-col">
      <DialogHeader>
        <DialogTitle>{{ bucketName }} - {{ t('bucketSettings') }}</DialogTitle>
      </DialogHeader>

      <!-- Tabs -->
      <div class="flex gap-1 border-b pb-2 overflow-x-auto">
        <button v-for="tab in tabs" :key="tab.id" @click="activeTab = tab.id" ...>
          {{ tab.label }}
        </button>
      </div>

      <!-- Loading state -->
      <div v-if="loading" class="flex justify-center py-12">
        <Spinner />
      </div>

      <!-- Tab Content -->
      <div v-else class="flex-1 overflow-y-auto py-4">
        <!-- POLICY TAB -->
        <div v-if="activeTab === 'policy'">
          <PolicyViewer :policy="config?.policy" />
        </div>

        <!-- ACL TAB -->
        <div v-if="activeTab === 'acl'">
          <AclViewer :acl="config?.acl" />
        </div>

        <!-- CORS TAB -->
        <div v-if="activeTab === 'cors'">
          <CorsViewer :cors="config?.cors" />
        </div>

        <!-- LIFECYCLE TAB -->
        <div v-if="activeTab === 'lifecycle'">
          <LifecycleViewer :lifecycle="config?.lifecycle" />
        </div>

        <!-- VERSIONING TAB -->
        <div v-if="activeTab === 'versioning'">
          <VersioningViewer :versioning="config?.versioning" />
        </div>

        <!-- ENCRYPTION TAB -->
        <div v-if="activeTab === 'encryption'">
          <EncryptionViewer :encryption="config?.encryption" />
        </div>
      </div>

      <DialogFooter>
        <Button variant="outline" @click="isOpen = false">{{ t('close') }}</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
```

### 3.2 Sous-composants pour chaque onglet (optionnel - peut etre inline)

Pour garder le code simple, les viewers peuvent etre directement dans le template:

1. **Policy Tab**: Affiche le JSON formaté avec syntax highlighting (ou message "No policy defined")
2. **ACL Tab**: Badge Public/Private avec explication
3. **CORS Tab**: Table avec les règles CORS
4. **Lifecycle Tab**: Liste des règles avec détails (expiration, transitions)
5. **Versioning Tab**: Status badge + MFA Delete status
6. **Encryption Tab**: Type d'encryption (AES256/KMS) avec détails

---

## Phase 4: Intégration dans BucketList.vue

### 4.1 Ajouter l'icone settings

Dans la colonne des icones (à coté du delete), ajouter:

```vue
<!-- Icons column (aligned vertically) -->
<div class="flex flex-col justify-between items-center w-6 flex-shrink-0">
  <!-- ACL Lock Icon (top) -->
  <div v-if="bucketAcls[bucket.name]" ...>...</div>

  <!-- Action icons row (bottom) -->
  <div class="flex items-center gap-1">
    <!-- Settings button -->
    <button
      @click.stop="openBucketSettings(bucket.name)"
      class="p-1 rounded hover:bg-white/20 text-muted-foreground hover:text-white transition-colors opacity-0 group-hover:opacity-100"
      v-tooltip="t('bucketSettings')"
    >
      <SettingsIcon :size="14" />
    </button>

    <!-- Delete bucket button -->
    <button v-if="bucketDeletePermissions[bucket.name]" ...>...</button>
  </div>
</div>
```

### 4.2 State et handlers

```typescript
const showBucketSettingsModal = ref(false)
const selectedBucketForSettings = ref('')

function openBucketSettings(bucketName: string) {
  selectedBucketForSettings.value = bucketName
  showBucketSettingsModal.value = true
}
```

---

## Phase 5: Traductions

### 5.1 Ajouter dans `translations.ts`

```typescript
// EN
bucketSettings: 'Bucket Settings',
bucketPolicy: 'Bucket Policy',
bucketPolicyNone: 'No bucket policy defined',
bucketPolicyAccessDenied: 'Access denied - insufficient permissions to view policy',
bucketAcl: 'ACL',
bucketCors: 'CORS',
bucketCorsNone: 'No CORS configuration',
bucketLifecycle: 'Lifecycle Rules',
bucketLifecycleNone: 'No lifecycle rules configured',
bucketVersioning: 'Versioning',
versioningEnabled: 'Enabled',
versioningSuspended: 'Suspended',
versioningNeverEnabled: 'Never enabled',
mfaDelete: 'MFA Delete',
bucketEncryption: 'Encryption',
bucketEncryptionNone: 'No server-side encryption configured',
encryptionAes256: 'SSE-S3 (AES-256)',
encryptionKms: 'SSE-KMS',
bucketKeyEnabled: 'Bucket Key',
accessDenied: 'Access Denied',
loadingConfiguration: 'Loading configuration...',

// FR
bucketSettings: 'Paramètres du bucket',
bucketPolicy: 'Politique du bucket',
bucketPolicyNone: 'Aucune politique définie',
bucketPolicyAccessDenied: 'Accès refusé - permissions insuffisantes',
bucketAcl: 'ACL',
bucketCors: 'CORS',
bucketCorsNone: 'Aucune configuration CORS',
bucketLifecycle: 'Règles de cycle de vie',
bucketLifecycleNone: 'Aucune règle configurée',
bucketVersioning: 'Versioning',
versioningEnabled: 'Activé',
versioningSuspended: 'Suspendu',
versioningNeverEnabled: 'Jamais activé',
mfaDelete: 'Suppression MFA',
bucketEncryption: 'Chiffrement',
bucketEncryptionNone: 'Pas de chiffrement côté serveur',
encryptionAes256: 'SSE-S3 (AES-256)',
encryptionKms: 'SSE-KMS',
bucketKeyEnabled: 'Bucket Key',
accessDenied: 'Accès refusé',
loadingConfiguration: 'Chargement de la configuration...',
```

---

## Fichiers à modifier/créer

| Fichier | Action |
|---------|--------|
| `src-tauri/src/models.rs` | Ajouter types BucketConfiguration |
| `src-tauri/src/s3_adapter.rs` | Ajouter 6 méthodes get_bucket_* |
| `src-tauri/src/commands.rs` | Ajouter commande get_bucket_configuration |
| `src-tauri/src/main.rs` | Enregistrer la commande |
| `src/types/index.ts` | Ajouter types TypeScript |
| `src/services/tauri.ts` | Ajouter getBucketConfiguration |
| `src/components/BucketSettingsModal.vue` | **NOUVEAU** - Modal avec 6 onglets |
| `src/components/BucketList.vue` | Ajouter icone settings + intégration modal |
| `src/i18n/translations.ts` | Ajouter traductions EN/FR |

---

## Estimation complexité

- **Backend Rust**: ~200 lignes (types + adapter + commands)
- **Frontend Types**: ~80 lignes
- **BucketSettingsModal.vue**: ~400 lignes (modal + 6 tab contents)
- **BucketList.vue**: ~30 lignes modifications
- **Traductions**: ~40 lignes

**Total estimé**: ~750 lignes de code

---

## Notes d'implémentation

1. **Appels parallèles**: `get_bucket_configuration` fait 6 appels S3 en parallèle (tokio::join!)
2. **Gestion des erreurs**: Chaque API peut échouer indépendamment (Access Denied, Not Configured)
3. **MinIO**: Certaines fonctionnalités peuvent ne pas être supportées (lifecycle, encryption)
4. **Cache optionnel**: Les résultats peuvent être cachés quelques minutes pour éviter les appels répétés
5. **JSON Policy**: Utiliser un formateur JSON pour afficher la policy de manière lisible
