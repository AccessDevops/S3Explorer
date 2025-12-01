# Plan d'Action : Système d'Indexation Locale Intelligent

## Résumé Exécutif

Ce document décrit l'implémentation d'un système d'indexation locale intelligent pour optimiser la navigation S3 en réduisant drastiquement les appels API tout en supportant des buckets de toute taille (jusqu'à 2 millions d'objets).

---

## 1. Analyse de l'Existant

### 1.1 Architecture Actuelle

**Backend Rust (`s3_adapter.rs`)**
- `list_objects()` : Supporte déjà `use_delimiter` (true/false) pour le mode delimiter vs flat
- `calculate_folder_size()` : Calcule la taille récursivement (multiple requêtes sans delimiter)
- `calculate_bucket_stats()` : Liste tous les objets sans delimiter

**Frontend TypeScript**
- `tauri.ts` : Wrapper des commandes Tauri (appels directs au backend)
- `app.ts` (Store Pinia) : Gère `objects`, `folders`, `continuationToken`, navigation
- `useSearchIndex.ts` : Index de recherche existant (IndexedDB, Web Worker)
- `ObjectBrowser.vue` : Watch sur `folders` qui déclenche `calculateFolderSize`

### 1.2 Points Clés Identifiés

1. **`listObjects`** est appelé avec `useDelimiter: true` par défaut (navigation par dossiers)
2. **Pagination** : `continuationToken` stocké dans `app.ts`, utilisé par "Load More"
3. **Calcul taille dossiers** : Déclenché par un `watch` sur `appStore.folders` → appel `calculateFolderSize` pour chaque dossier
4. **Index existant** (`useSearchIndex.ts`) : Structure plate, pas de concept de prefix complet
5. **Pas de cache/index actuel** pour la navigation standard

---

## 2. Architecture Proposée

### 2.1 Vue d'Ensemble

```
┌─────────────────────────────────────────────────────────────────┐
│                         ObjectBrowser.vue                        │
│                    (Consommateur de données)                     │
└───────────────────────────┬─────────────────────────────────────┘
                            │ Utilise
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                     S3IndexProxy (NOUVEAU)                       │
│              src/services/s3IndexProxy.ts                        │
│                                                                 │
│  • Intercepte toutes les requêtes S3 (list, navigate, etc.)     │
│  • Gère l'index local (BucketIndex)                             │
│  • Décide : utiliser index OU faire requête S3                  │
│  • Met à jour l'index après chaque opération CRUD               │
└───────────────────────────┬─────────────────────────────────────┘
                            │ Utilise
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                      tauri.ts (inchangé)                         │
│                  (Appels Tauri vers Rust)                        │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Backend Rust (s3_adapter.rs)                   │
│                      (Requêtes S3 réelles)                       │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Nouvelle Structure d'Index

```typescript
// src/types/index.ts (nouvelles interfaces)

/**
 * Objet indexé avec métadonnées LRU
 */
interface IndexedObject {
  key: string
  size: number
  lastModified: number          // timestamp
  eTag?: string
  storageClass?: string
  lastAccessedAt: number        // Pour LRU eviction
  indexedAt: number             // Quand l'objet a été ajouté à l'index
}

/**
 * Statut de complétion d'un prefix
 */
interface PrefixStatus {
  prefix: string                // Ex: "folder1/subfolder/" ou "" (racine)
  isComplete: boolean           // true = tous les objets de ce niveau sont indexés
  lastUpdatedAt: number         // timestamp
  childCount: number            // Nombre d'enfants directs (objets + dossiers)
  totalSize?: number            // Taille totale si complete, undefined sinon
}

/**
 * Statut global du bucket dans l'index
 */
interface BucketIndexStatus {
  bucketName: string
  profileId: string
  isFullyIndexed: boolean       // true = bucket entièrement indexé
  totalObjects: number          // Nombre total d'objets dans l'index
  totalSize: number             // Taille totale indexée
  lastFullScanAt?: number       // Dernière indexation complète
  createdAt: number
  updatedAt: number
}

/**
 * Index complet d'un bucket
 */
interface BucketIndex {
  status: BucketIndexStatus
  prefixStatuses: Map<string, PrefixStatus>  // Clé = prefix
  objects: Map<string, IndexedObject>        // Clé = key complet
}
```

### 2.3 Options du Proxy

```typescript
/**
 * Options pour les requêtes via le proxy
 */
interface ProxyRequestOptions {
  /**
   * Ignorer complètement l'index et faire une requête S3 directe
   * Utile pour forcer une synchronisation
   */
  ignoreIndex?: boolean

  /**
   * Mettre à jour l'index avec le résultat de la requête
   * @default true
   */
  updateIndex?: boolean

  /**
   * Mode de mise à jour de l'index
   * - 'replace': Remplacer les données existantes pour ce prefix
   * - 'merge': Fusionner avec les données existantes (ajouter/màj)
   * @default 'merge'
   */
  indexUpdateMode?: 'replace' | 'merge'
}
```

---

## 3. Flux de Fonctionnement

### 3.1 Connexion à un Nouveau Bucket

```
┌─────────────────────────────────────────────────────────────────┐
│                    FLUX: Connexion Bucket                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Utilisateur sélectionne un bucket                           │
│     │                                                           │
│     ▼                                                           │
│  2. S3IndexProxy.initializeBucket(profileId, bucketName)        │
│     │                                                           │
│     ├─► Charger index existant depuis IndexedDB (si présent)    │
│     │                                                           │
│     ▼                                                           │
│  3. PHASE D'INDEXATION INITIALE (max 20 requêtes sans delimiter)│
│     │                                                           │
│     │   for (let i = 0; i < MAX_INITIAL_REQUESTS; i++) {        │
│     │     response = listObjects(bucket, prefix="",             │
│     │                            delimiter=false,               │
│     │                            maxKeys=batchSize)             │
│     │     addToIndex(response.objects)                          │
│     │     if (!response.continuation_token) break // Fini!      │
│     │   }                                                       │
│     │                                                           │
│     ▼                                                           │
│  4. ÉVALUATION DU RÉSULTAT                                      │
│     │                                                           │
│     ├─► Si COMPLET (pas de continuation_token après < 20 req):  │
│     │   • Marquer bucket comme isFullyIndexed = true            │
│     │   • Marquer racine "" comme isComplete = true             │
│     │   • Navigation 100% locale possible                       │
│     │                                                           │
│     └─► Si INCOMPLET (continuation_token encore présent):       │
│         • Afficher dialog: "Bucket volumineux détecté"          │
│         • Options: [Continuer indexation] [Laisser progressif]  │
│         • Faire 1 requête avec delimiter à la racine            │
│         • Stocker le continuation_token pour reprise            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Navigation dans l'Arborescence

```
┌─────────────────────────────────────────────────────────────────┐
│                    FLUX: Navigation Dossier                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Utilisateur clique sur un dossier (prefix: "folder1/")      │
│     │                                                           │
│     ▼                                                           │
│  2. S3IndexProxy.listObjects(bucket, prefix="folder1/")         │
│     │                                                           │
│     ├─► Vérifier PrefixStatus pour "folder1/"                   │
│     │                                                           │
│     ├─► SI isComplete = true:                                   │
│     │   • Retourner objets depuis l'index (local)               │
│     │   • Calculer tailles depuis l'index                       │
│     │   • Zéro requête S3!                                      │
│     │                                                           │
│     └─► SI isComplete = false OU absent:                        │
│         • Faire requête S3 AVEC delimiter                       │
│         • Ajouter résultats à l'index                           │
│         • Marquer prefix comme isComplete si pas truncated      │
│         │                                                       │
│         ├─► Pour chaque nouveau sous-dossier découvert:         │
│         │   • Si inconnu dans l'index:                          │
│         │     - Taille affichée = "-" (non calculée)            │
│         │     - PrefixStatus.isComplete = false                 │
│         │                                                       │
│         └─► Pour chaque sous-dossier CONNU dans l'index:        │
│             • Si PrefixStatus.isComplete = true:                │
│               - Calculer taille depuis index                    │
│             • Sinon:                                            │
│               - Taille = "-"                                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 3.3 Synchronisation avec S3 (Détection des suppressions)

```
┌─────────────────────────────────────────────────────────────────┐
│              FLUX: Détection des Suppressions                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Lors d'une navigation avec requête S3 (index incomplet):       │
│                                                                 │
│  1. Recevoir réponse S3 pour prefix "folder1/"                  │
│     │                                                           │
│     ▼                                                           │
│  2. Comparer avec objets existants dans l'index pour ce prefix  │
│     │                                                           │
│     │  indexedObjects = getObjectsForPrefix("folder1/")         │
│     │  s3Objects = response.objects                             │
│     │                                                           │
│     │  // Trouver les objets supprimés                          │
│     │  deletedKeys = indexedObjects.keys - s3Objects.keys       │
│     │                                                           │
│     ▼                                                           │
│  3. Pour chaque clé supprimée:                                  │
│     │                                                           │
│     │  • Supprimer de l'index                                   │
│     │  • Recalculer PrefixStatus.totalSize                      │
│     │  • Propager invalidation aux parents si nécessaire        │
│     │                                                           │
│     ▼                                                           │
│  4. Mettre à jour le PrefixStatus                               │
│     │                                                           │
│     │  prefixStatus.lastUpdatedAt = Date.now()                  │
│     │  prefixStatus.childCount = s3Objects.length               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 3.4 Mise à jour après Opérations CRUD

```
┌─────────────────────────────────────────────────────────────────┐
│                 FLUX: Mise à jour CRUD                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ UPLOAD (putObject, createFolder)                        │    │
│  ├─────────────────────────────────────────────────────────┤    │
│  │ 1. Attendre confirmation S3 (success)                   │    │
│  │ 2. Ajouter objet à l'index avec:                        │    │
│  │    - lastAccessedAt = Date.now()                        │    │
│  │    - indexedAt = Date.now()                             │    │
│  │ 3. Mettre à jour PrefixStatus du parent:                │    │
│  │    - childCount++                                       │    │
│  │    - totalSize += newObject.size                        │    │
│  │ 4. Propager mise à jour taille vers ancêtres            │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ DELETE (deleteObject, deleteFolder)                     │    │
│  ├─────────────────────────────────────────────────────────┤    │
│  │ 1. Attendre confirmation S3 (success)                   │    │
│  │ 2. Supprimer objet(s) de l'index                        │    │
│  │ 3. Mettre à jour PrefixStatus du parent:                │    │
│  │    - childCount--                                       │    │
│  │    - totalSize -= deletedObject.size                    │    │
│  │ 4. Propager mise à jour taille vers ancêtres            │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ RENAME (copy + delete)                                  │    │
│  ├─────────────────────────────────────────────────────────┤    │
│  │ 1. Attendre confirmation S3 (copy success)              │    │
│  │ 2. Attendre confirmation S3 (delete success)            │    │
│  │ 3. Mettre à jour la clé dans l'index                    │    │
│  │ 4. Si changement de prefix parent:                      │    │
│  │    - Décrémenter childCount ancien parent               │    │
│  │    - Incrémenter childCount nouveau parent              │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. Mécanisme LRU (Least Recently Used)

### 4.1 Configuration

```typescript
// Paramètres de l'application (settings.ts)
interface IndexSettings {
  maxIndexCapacity: number      // Default: 2_000_000
  evictionPercentage: number    // Default: 0.10 (10%)
  maxInitialRequests: number    // Default: 20
}
```

### 4.2 Flux d'Éviction

```
┌─────────────────────────────────────────────────────────────────┐
│                    FLUX: Éviction LRU                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  AVANT chaque insertion bulk:                                   │
│                                                                 │
│  1. Calculer: newTotal = currentCount + incomingCount           │
│     │                                                           │
│     ▼                                                           │
│  2. SI newTotal > MAX_CAPACITY:                                 │
│     │                                                           │
│     │  overflow = newTotal - MAX_CAPACITY                       │
│     │  toEvict = max(overflow, MAX_CAPACITY * 0.10)             │
│     │                                                           │
│     ▼                                                           │
│  3. Trier objets par lastAccessedAt (ASC = plus vieux d'abord)  │
│     │                                                           │
│     │  sortedObjects = [...objects].sort(                       │
│     │    (a, b) => a.lastAccessedAt - b.lastAccessedAt          │
│     │  )                                                        │
│     │                                                           │
│     ▼                                                           │
│  4. Supprimer les N objets les plus vieux                       │
│     │                                                           │
│     │  evictedObjects = sortedObjects.slice(0, toEvict)         │
│     │  for (obj of evictedObjects) {                            │
│     │    index.objects.delete(obj.key)                          │
│     │  }                                                        │
│     │                                                           │
│     ▼                                                           │
│  5. Recalculer les PrefixStatus impactés                        │
│     │                                                           │
│     │  affectedPrefixes = new Set()                             │
│     │  for (obj of evictedObjects) {                            │
│     │    affectedPrefixes.add(getParentPrefix(obj.key))         │
│     │  }                                                        │
│     │                                                           │
│     │  for (prefix of affectedPrefixes) {                       │
│     │    prefixStatus.isComplete = false // IMPORTANT!          │
│     │    recalculatePrefixStats(prefix)                         │
│     │  }                                                        │
│     │                                                           │
│     ▼                                                           │
│  6. Mettre à jour BucketIndexStatus                             │
│     │                                                           │
│     │  status.isFullyIndexed = false                            │
│     │  status.totalObjects = index.objects.size                 │
│     │                                                           │
└─────────────────────────────────────────────────────────────────┘
```

### 4.3 Mise à jour de `lastAccessedAt`

```typescript
// À chaque LECTURE d'objets (navigation, affichage):
function touchObjects(keys: string[]) {
  const now = Date.now()
  for (const key of keys) {
    const obj = index.objects.get(key)
    if (obj) {
      obj.lastAccessedAt = now
    }
  }
}
```

---

## 5. Structure des Fichiers à Créer/Modifier

### 5.1 Nouveaux Fichiers

```
src/
├── services/
│   └── s3IndexProxy.ts          # Proxy principal (NOUVEAU)
├── composables/
│   └── useBucketIndex.ts        # Composable pour gestion index (NOUVEAU)
├── types/
│   └── bucketIndex.ts           # Types pour l'index (NOUVEAU)
└── workers/
    └── indexWorker.ts           # Web Worker pour opérations lourdes (NOUVEAU)
```

### 5.2 Fichiers à Modifier

```
src/
├── services/
│   └── tauri.ts                 # Ajouter types pour nouvelles options
├── stores/
│   ├── app.ts                   # Intégrer S3IndexProxy
│   └── settings.ts              # Ajouter paramètres index
├── components/
│   └── ObjectBrowser.vue        # Utiliser proxy, afficher "-" pour tailles
├── composables/
│   └── useSearchIndex.ts        # Potentiellement fusionner/réutiliser
└── types/
    └── index.ts                 # Ajouter nouvelles interfaces
```

---

## 6. Implémentation Détaillée

### 6.1 S3IndexProxy (`src/services/s3IndexProxy.ts`)

```typescript
// Structure du fichier
export class S3IndexProxy {
  private indexes: Map<string, BucketIndex> = new Map()
  private db: IDBDatabase | null = null

  // Configuration
  private maxCapacity: number
  private evictionPercentage: number
  private maxInitialRequests: number

  // === Méthodes Publiques ===

  /**
   * Initialiser un bucket (appelé à la sélection)
   * Effectue l'indexation initiale
   */
  async initializeBucket(
    profileId: string,
    bucketName: string,
    options?: { forceReindex?: boolean }
  ): Promise<InitializationResult>

  /**
   * Lister les objets (navigation)
   * Utilise l'index si possible, sinon requête S3
   */
  async listObjects(
    profileId: string,
    bucket: string,
    prefix: string,
    options?: ProxyRequestOptions
  ): Promise<ProxyListResult>

  /**
   * Charger plus d'objets (pagination)
   */
  async loadMore(
    profileId: string,
    bucket: string,
    prefix: string,
    options?: ProxyRequestOptions
  ): Promise<ProxyListResult>

  /**
   * Calculer la taille d'un dossier
   * Utilise l'index si prefix complet, sinon retourne undefined
   */
  calculateFolderSize(
    profileId: string,
    bucket: string,
    prefix: string
  ): number | undefined

  /**
   * Notifier d'une opération CRUD réussie
   */
  notifyObjectCreated(profileId: string, bucket: string, obj: IndexedObject): void
  notifyObjectDeleted(profileId: string, bucket: string, key: string): void
  notifyObjectsDeleted(profileId: string, bucket: string, keys: string[]): void

  /**
   * Continuer l'indexation (si bucket volumineux)
   */
  async continueIndexing(
    profileId: string,
    bucket: string,
    onProgress?: (current: number, total: number) => void
  ): Promise<void>

  /**
   * Obtenir le statut de l'index
   */
  getIndexStatus(profileId: string, bucket: string): BucketIndexStatus | null

  /**
   * Vider l'index d'un bucket
   */
  async clearIndex(profileId: string, bucket: string): Promise<void>

  // === Méthodes Privées ===

  private async performInitialIndexing(...)
  private async syncPrefixWithS3(...)
  private evictIfNeeded(count: number): void
  private updatePrefixStatus(...)
  private propagateSizeChange(...)
  private getIndexKey(profileId: string, bucket: string): string
  private async saveToIndexedDB(...)
  private async loadFromIndexedDB(...)
}

// Singleton export
export const s3IndexProxy = new S3IndexProxy()
```

### 6.2 Types (`src/types/bucketIndex.ts`)

```typescript
export interface IndexedObject {
  key: string
  size: number
  lastModified: number
  eTag?: string
  storageClass?: string
  lastAccessedAt: number
  indexedAt: number
}

export interface PrefixStatus {
  prefix: string
  isComplete: boolean
  lastUpdatedAt: number
  childCount: number
  totalSize?: number
  continuationToken?: string  // Pour reprendre si interrompu
}

export interface BucketIndexStatus {
  bucketName: string
  profileId: string
  isFullyIndexed: boolean
  totalObjects: number
  totalSize: number
  lastFullScanAt?: number
  indexingContinuationToken?: string  // Pour reprendre l'indexation
  createdAt: number
  updatedAt: number
}

export interface BucketIndex {
  status: BucketIndexStatus
  prefixStatuses: Map<string, PrefixStatus>
  objects: Map<string, IndexedObject>
}

export interface ProxyRequestOptions {
  ignoreIndex?: boolean
  updateIndex?: boolean
  indexUpdateMode?: 'replace' | 'merge'
}

export interface ProxyListResult {
  objects: S3Object[]
  folders: string[]
  hasMore: boolean
  fromIndex: boolean  // true si résultat depuis l'index
  indexStatus?: {
    isComplete: boolean
    totalIndexed: number
  }
}

export interface InitializationResult {
  success: boolean
  isFullyIndexed: boolean
  totalIndexed: number
  showLargeBucketDialog: boolean
  continuationToken?: string
}
```

### 6.3 Paramètres Settings (`src/stores/settings.ts`)

```typescript
// Ajouter aux paramètres existants:

// Index settings
const maxIndexCapacity = ref(2_000_000)        // Max 2M objets
const maxInitialRequests = ref(20)              // Requêtes initiales sans delimiter
const indexEvictionPercentage = ref(10)         // 10% éviction LRU

// Setters
const setMaxIndexCapacity = (value: number) => {
  if (value < 100000 || value > 10_000_000) {
    throw new Error('Index capacity must be between 100,000 and 10,000,000')
  }
  maxIndexCapacity.value = value
  localStorage.setItem('app-maxIndexCapacity', String(value))
}

const setMaxInitialRequests = (value: number) => {
  if (value < 1 || value > 100) {
    throw new Error('Max initial requests must be between 1 and 100')
  }
  maxInitialRequests.value = value
  localStorage.setItem('app-maxInitialRequests', String(value))
}
```

---

## 7. Modifications de l'UI

### 7.1 Affichage des Tailles de Dossiers

```vue
<!-- ObjectBrowser.vue - Colonne taille -->
<div class="text-sm text-muted-foreground w-24 text-right">
  <template v-if="isFolderSizeLoading(folder)">
    <span class="animate-pulse">...</span>
  </template>
  <template v-else-if="getFolderSize(folder) !== undefined">
    {{ formatSize(getFolderSize(folder)) }}
  </template>
  <template v-else>
    <span class="text-muted-foreground/50" v-tooltip="t('sizeNotIndexed')">-</span>
  </template>
</div>
```

### 7.2 Dialog "Bucket Volumineux"

```vue
<!-- Nouveau composant ou dans ObjectBrowser.vue -->
<Dialog v-model:open="showLargeBucketDialog">
  <DialogContent>
    <DialogHeader>
      <DialogTitle>{{ t('largeBucketDetected') }}</DialogTitle>
    </DialogHeader>
    <DialogDescription>
      {{ t('largeBucketMessage', { count: indexedCount }) }}
    </DialogDescription>
    <div class="flex gap-4 mt-4">
      <Button variant="outline" @click="continueIndexing">
        {{ t('continueIndexing') }}
      </Button>
      <Button @click="useProgressiveIndexing">
        {{ t('useProgressiveIndexing') }}
      </Button>
    </div>
  </DialogContent>
</Dialog>
```

### 7.3 Indicateur d'État de l'Index

```vue
<!-- Badge dans la toolbar -->
<Badge v-if="indexStatus" :variant="indexStatus.isFullyIndexed ? 'success' : 'secondary'">
  <template v-if="indexStatus.isFullyIndexed">
    ✓ {{ t('fullyIndexed') }} ({{ formatNumber(indexStatus.totalObjects) }})
  </template>
  <template v-else>
    ◐ {{ t('partiallyIndexed') }} ({{ formatNumber(indexStatus.totalObjects) }})
  </template>
</Badge>
```

---

## 8. Tests

### 8.1 Tests Unitaires

```typescript
// tests/s3IndexProxy.test.ts

describe('S3IndexProxy', () => {
  describe('initializeBucket', () => {
    it('should fully index small buckets (< 20 * batchSize objects)')
    it('should partially index large buckets and show dialog')
    it('should reuse existing index if valid')
  })

  describe('listObjects', () => {
    it('should return from index when prefix is complete')
    it('should fetch from S3 when prefix is incomplete')
    it('should detect and remove deleted objects')
    it('should mark new folders with size "-"')
  })

  describe('LRU eviction', () => {
    it('should evict oldest objects when capacity exceeded')
    it('should mark affected prefixes as incomplete after eviction')
    it('should update lastAccessedAt on read')
  })

  describe('CRUD notifications', () => {
    it('should add object to index on create')
    it('should remove object from index on delete')
    it('should propagate size changes to parent prefixes')
  })
})
```

### 8.2 Tests d'Intégration

```typescript
// tests/integration/indexing.test.ts

describe('Index Integration', () => {
  it('should navigate entirely from index for small bucket')
  it('should mix index and S3 requests for large bucket')
  it('should sync correctly after external modifications')
  it('should handle concurrent operations')
})
```

---

## 9. Plan d'Implémentation (Étapes)

### Phase 1 : Infrastructure (Jour 1-2)
1. Créer les types (`src/types/bucketIndex.ts`)
2. Créer `S3IndexProxy` avec structure de base
3. Implémenter stockage IndexedDB
4. Ajouter paramètres dans `settings.ts`

### Phase 2 : Indexation Initiale (Jour 2-3)
1. Implémenter `initializeBucket()`
2. Implémenter logique des 20 requêtes initiales
3. Implémenter détection bucket volumineux
4. Créer dialog "Bucket Volumineux"

### Phase 3 : Navigation avec Index (Jour 3-4)
1. Implémenter `listObjects()` avec fallback S3
2. Implémenter `calculateFolderSize()` depuis index
3. Implémenter détection des suppressions
4. Modifier `ObjectBrowser.vue` pour utiliser le proxy

### Phase 4 : CRUD et Synchronisation (Jour 4-5)
1. Implémenter notifications CRUD
2. Implémenter propagation des changements de taille
3. Intégrer dans les composants existants (upload, delete, rename)

### Phase 5 : LRU et Optimisations (Jour 5-6)
1. Implémenter mécanisme LRU
2. Implémenter éviction
3. Optimiser avec Web Worker si nécessaire
4. Tests de performance

### Phase 6 : Polish et Tests (Jour 6-7)
1. Tests unitaires
2. Tests d'intégration
3. Documentation
4. Review et corrections

---

## 10. Questions Ouvertes / Décisions

| Question | Décision Proposée |
|----------|-------------------|
| Requêtes initiales séquentielles ou parallèles? | **Séquentielles** (utilise continuation_token) |
| Affichage dossiers non indexés | **"-"** (simple et clair) |
| Fusion avec useSearchIndex.ts existant? | **Non** - Garder séparés (responsabilités différentes) |
| Stockage de l'index | **IndexedDB** (comme l'existant, performant) |
| Web Worker pour LRU? | **Oui** si éviction > 50K objets |

---

## 11. Métriques de Succès

- **Réduction des requêtes S3** : -80% pour buckets entièrement indexés
- **Temps de navigation** : < 100ms depuis l'index vs 500-2000ms depuis S3
- **Utilisation mémoire** : Stable à ~200MB pour 2M objets
- **Temps d'indexation initial** : < 30s pour 20 * 1000 = 20K objets
