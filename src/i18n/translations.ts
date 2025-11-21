export type Language = 'en' | 'fr'

export const translations = {
  en: {
    // General
    close: 'Close',
    cancel: 'Cancel',
    save: 'Save',
    delete: 'Delete',
    edit: 'Edit',
    add: 'Add',
    refresh: 'Refresh',
    search: 'Search',
    upload: 'Upload',
    download: 'Download',
    copy: 'Copy',
    paste: 'Paste',
    rename: 'Rename',
    view: 'View',
    create: 'Create',
    clear: 'Clear',
    testConnection: 'Test Connection',
    selected: 'selected',

    // Settings
    settings: 'Settings',
    language: 'Language',
    languageDescription: 'Choose your preferred language',

    // Connections
    connections: 'Connections',
    addConnection: 'Add Connection',
    editConnection: 'Edit Connection',
    deleteConnection: 'Delete Connection',
    connectionProfile: 'Connection Profile',
    profileName: 'Profile Name',
    endpoint: 'Endpoint URL',
    endpointPlaceholder: 'http://localhost:9000 (leave empty for AWS)',
    endpointDescription: 'For MinIO, Backblaze, or other S3-compatible services',
    region: 'Region',
    accessKey: 'Access Key',
    secretKey: 'Secret Key',
    sessionToken: 'Session Token',
    sessionTokenPlaceholder: 'For temporary credentials',
    pathStyle: 'Force path-style addressing (required for MinIO)',
    useTls: 'Use TLS/HTTPS',
    configureS3: 'Configure your S3 connection settings. Supports AWS, MinIO, Backblaze, and other S3-compatible services.',

    // Buckets
    buckets: 'Buckets',
    bucket: 'Bucket',
    selectBucket: 'Select a bucket',
    createBucket: 'Create Bucket',
    bucketName: 'Bucket Name',
    bucketNamePlaceholder: 'my-bucket-name',
    noBuckets: 'No buckets available',
    bucketStats: 'Statistics',
    bucketVisibility: 'Visibility',
    bucketSize: 'Size',
    bucketObjects: 'Objects',

    // Objects
    objects: 'Objects',
    folders: 'Folders',
    files: 'Files',
    newFolder: 'New Folder',
    newFile: 'New File',
    folderName: 'Folder name',
    fileName: 'File name',
    fileContent: 'Content',
    fileContentPlaceholder: 'Enter file content...',

    // Stats
    items: 'items',
    item: 'item',
    totalSize: 'Total size',

    // Messages
    welcomeTitle: 'Welcome to S3 Explorer',
    welcomeMessageNoProfile: 'Please create or select a connection profile to get started.',
    welcomeMessageNoBucket: 'Please select a bucket from the sidebar.',

    // Search
    searchPlaceholder: 'Search files...',
    searching: 'Searching...',

    // Context menu
    createFileHere: 'New File',
    createFolderHere: 'New Folder',

    // Errors
    errorOccurred: 'An error occurred',
    deleteConfirm: 'Are you sure you want to delete',

    // Actions
    uploading: 'Uploading...',
    downloading: 'Downloading...',
    deleting: 'Deleting...',
    creating: 'Creating...',
    renaming: 'Renaming...',
    loading: 'Loading...',
    calculating: 'Calculating...',
    saving: 'Saving...',
    remaining: 'remaining',
    estimating: 'Estimating...',

    // Upload
    dropFilesHere: 'Drop files here to upload',
    uploadFile: 'Upload File',
    selectedFile: 'Selected',
    size: 'Size',

    // Folder operations
    createFolder: 'Create Folder',
    folderDeletedSuccess: 'Folder deleted successfully! {0} objects removed.',

    // File operations
    renameFile: 'Rename File',
    newName: 'New name',
    enterNewFileName: 'Enter new file name',
    createNewFile: 'Create New File',
    contentOptional: 'Content (optional)',
    fileDownloadedSuccess: 'File downloaded successfully!',
    changeContentType: 'Change Content Type',
    contentType: 'Content Type',
    contentTypeUpdated: 'Content type updated successfully!',
    viewVersions: 'View Versions',
    versions: 'Versions',
    version: 'Version',
    versionId: 'Version ID',
    latest: 'Latest',
    noVersions: 'No versions available',
    versioningNotEnabled: 'Versioning is not enabled for this bucket',

    // Confirmations
    deleteFileConfirm: 'Are you sure you want to delete "{0}"?',
    deleteFolderConfirm: 'Are you sure you want to delete the folder "{0}" and all its contents?',
    deleteItemsConfirm: 'Are you sure you want to delete {0} item{1}?',
    fileExistsConfirm: 'A file named "{0}" already exists in this location. Do you want to replace it?',

    // Success messages
    uploadSuccess: 'Successfully uploaded {0} file{1}!',
    deleteSuccess: 'Successfully deleted {0} item{1}!',
    filesDownloadedSuccess: 'Successfully downloaded {0} file(s)!',

    // Error messages
    uploadFailed: 'Upload failed',
    downloadFailed: 'Download failed',
    deleteFailed: 'Delete failed',
    createFailed: 'Failed to create folder',
    renameFailed: 'Failed to rename',
    pasteFailed: 'Failed to paste file',
    createFileFailed: 'Failed to create file',
    deleteOperationFailed: 'Delete operation failed',
    uploadPartialSuccess: 'Uploaded {0} file(s), {1} failed',
    deletePartialSuccess: 'Deleted {0} item(s), {1} failed',
    downloadPartialSuccess: 'Downloaded {0} file(s), {1} failed',
    selectProfileFirst: 'Please select a profile and bucket first',
    cannotCopyFolders: 'Cannot copy folders. Please select files only.',
    multiCopyNotSupported: 'Multiple file copy is not yet supported. Please select only one file.',
    noFilesToDownload: 'No files selected. Please select files to download.',
    selectDownloadFolder: 'Select download folder',

    // Search
    searchFilesAndFolders: 'Search files and folders...',
  },
  fr: {
    // Général
    close: 'Fermer',
    cancel: 'Annuler',
    save: 'Enregistrer',
    delete: 'Supprimer',
    edit: 'Modifier',
    add: 'Ajouter',
    refresh: 'Actualiser',
    search: 'Rechercher',
    upload: 'Téléverser',
    download: 'Télécharger',
    copy: 'Copier',
    paste: 'Coller',
    rename: 'Renommer',
    view: 'Voir',
    create: 'Créer',
    clear: 'Effacer',
    testConnection: 'Tester la connexion',
    selected: 'sélectionné(s)',

    // Paramètres
    settings: 'Paramètres',
    language: 'Langue',
    languageDescription: 'Choisissez votre langue préférée',

    // Connexions
    connections: 'Connexions',
    addConnection: 'Ajouter une connexion',
    editConnection: 'Modifier la connexion',
    deleteConnection: 'Supprimer la connexion',
    connectionProfile: 'Profil de connexion',
    profileName: 'Nom du profil',
    endpoint: 'URL de point de terminaison',
    endpointPlaceholder: 'http://localhost:9000 (laisser vide pour AWS)',
    endpointDescription: 'Pour MinIO, Backblaze ou autres services compatibles S3',
    region: 'Région',
    accessKey: 'Clé d\'accès',
    secretKey: 'Clé secrète',
    sessionToken: 'Jeton de session',
    sessionTokenPlaceholder: 'Pour les identifiants temporaires',
    pathStyle: 'Forcer l\'adressage par chemin (requis pour MinIO)',
    useTls: 'Utiliser TLS/HTTPS',
    configureS3: 'Configurez vos paramètres de connexion S3. Prend en charge AWS, MinIO, Backblaze et autres services compatibles S3.',

    // Buckets
    buckets: 'Buckets',
    bucket: 'Bucket',
    selectBucket: 'Sélectionner un bucket',
    createBucket: 'Créer un bucket',
    bucketName: 'Nom du bucket',
    bucketNamePlaceholder: 'nom-de-mon-bucket',
    noBuckets: 'Aucun bucket disponible',
    bucketStats: 'Statistiques',
    bucketVisibility: 'Visibilité',
    bucketSize: 'Taille',
    bucketObjects: 'Objets',

    // Objets
    objects: 'Objets',
    folders: 'Dossiers',
    files: 'Fichiers',
    newFolder: 'Nouveau dossier',
    newFile: 'Nouveau fichier',
    folderName: 'Nom du dossier',
    fileName: 'Nom du fichier',
    fileContent: 'Contenu',
    fileContentPlaceholder: 'Saisir le contenu du fichier...',

    // Statistiques
    items: 'éléments',
    item: 'élément',
    totalSize: 'Taille totale',

    // Messages
    welcomeTitle: 'Bienvenue dans S3 Explorer',
    welcomeMessageNoProfile: 'Veuillez créer ou sélectionner un profil de connexion pour commencer.',
    welcomeMessageNoBucket: 'Veuillez sélectionner un bucket dans la barre latérale.',

    // Recherche
    searchPlaceholder: 'Rechercher des fichiers...',
    searching: 'Recherche en cours...',

    // Menu contextuel
    createFileHere: 'Nouveau fichier',
    createFolderHere: 'Nouveau dossier',

    // Erreurs
    errorOccurred: 'Une erreur est survenue',
    deleteConfirm: 'Êtes-vous sûr de vouloir supprimer',

    // Actions
    uploading: 'Téléversement...',
    downloading: 'Téléchargement...',
    deleting: 'Suppression...',
    creating: 'Création...',
    renaming: 'Renommage...',
    loading: 'Chargement...',
    calculating: 'Calcul...',
    saving: 'Enregistrement...',
    remaining: 'restant',
    estimating: 'Estimation...',

    // Téléversement
    dropFilesHere: 'Déposer les fichiers ici pour téléverser',
    uploadFile: 'Téléverser un fichier',
    selectedFile: 'Sélectionné',
    size: 'Taille',

    // Opérations sur les dossiers
    createFolder: 'Créer un dossier',
    folderDeletedSuccess: 'Dossier supprimé avec succès ! {0} objets supprimés.',

    // Opérations sur les fichiers
    renameFile: 'Renommer le fichier',
    newName: 'Nouveau nom',
    enterNewFileName: 'Saisir le nouveau nom de fichier',
    createNewFile: 'Créer un nouveau fichier',
    contentOptional: 'Contenu (optionnel)',
    fileDownloadedSuccess: 'Fichier téléchargé avec succès !',
    changeContentType: 'Changer le type de contenu',
    contentType: 'Type de contenu',
    contentTypeUpdated: 'Type de contenu mis à jour avec succès !',
    viewVersions: 'Voir les versions',
    versions: 'Versions',
    version: 'Version',
    versionId: 'ID de version',
    latest: 'Dernière',
    noVersions: 'Aucune version disponible',
    versioningNotEnabled: 'Le versioning n\'est pas activé pour ce bucket',

    // Confirmations
    deleteFileConfirm: 'Êtes-vous sûr de vouloir supprimer "{0}" ?',
    deleteFolderConfirm: 'Êtes-vous sûr de vouloir supprimer le dossier "{0}" et tout son contenu ?',
    deleteItemsConfirm: 'Êtes-vous sûr de vouloir supprimer {0} élément{1} ?',
    fileExistsConfirm: 'Un fichier nommé "{0}" existe déjà à cet emplacement. Voulez-vous le remplacer ?',

    // Messages de succès
    uploadSuccess: '{0} fichier{1} téléversé avec succès !',
    deleteSuccess: '{0} élément{1} supprimé avec succès !',
    filesDownloadedSuccess: '{0} fichier(s) téléchargé(s) avec succès !',

    // Messages d\'erreur
    uploadFailed: 'Échec du téléversement',
    downloadFailed: 'Échec du téléchargement',
    deleteFailed: 'Échec de la suppression',
    createFailed: 'Échec de la création du dossier',
    renameFailed: 'Échec du renommage',
    pasteFailed: 'Échec du collage du fichier',
    createFileFailed: 'Échec de la création du fichier',
    deleteOperationFailed: 'Échec de l\'opération de suppression',
    uploadPartialSuccess: '{0} fichier(s) téléversé(s), {1} échec(s)',
    deletePartialSuccess: '{0} élément(s) supprimé(s), {1} échec(s)',
    downloadPartialSuccess: '{0} fichier(s) téléchargé(s), {1} échec(s)',
    selectProfileFirst: 'Veuillez d\'abord sélectionner un profil et un bucket',
    cannotCopyFolders: 'Impossible de copier des dossiers. Veuillez sélectionner uniquement des fichiers.',
    multiCopyNotSupported: 'La copie de plusieurs fichiers n\'est pas encore prise en charge. Veuillez sélectionner un seul fichier.',
    noFilesToDownload: 'Aucun fichier sélectionné. Veuillez sélectionner des fichiers à télécharger.',
    selectDownloadFolder: 'Sélectionner le dossier de téléchargement',

    // Recherche
    searchFilesAndFolders: 'Rechercher des fichiers et dossiers...',
  },
}

export type TranslationKey = keyof typeof translations.en
