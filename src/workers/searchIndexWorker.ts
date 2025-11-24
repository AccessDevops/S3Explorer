/**
 * Web Worker for building search indexes
 * Runs on a separate thread to avoid blocking the UI
 */

export interface SearchIndexObject {
  key: string
  size: number
  lastModified: number
  searchKey: string
}

export interface SearchIndex {
  bucketName: string
  profileId: string
  lastBuilt: number
  totalObjects: number
  sizeInBytes: number
  objects: SearchIndexObject[]
}

interface WorkerInitMessage {
  type: 'init'
  bucketName: string
  profileId: string
}

interface WorkerProcessBatchMessage {
  type: 'processBatch'
  objects: Array<{
    key: string
    size: number
    last_modified?: string
  }>
}

interface WorkerFinalizeMessage {
  type: 'finalize'
}

type WorkerInputMessage = WorkerInitMessage | WorkerProcessBatchMessage | WorkerFinalizeMessage

interface WorkerProgressMessage {
  type: 'progress'
  current: number
  total: number
}

interface WorkerCompleteMessage {
  type: 'complete'
  index: SearchIndex
}

interface WorkerErrorMessage {
  type: 'error'
  error: string
}

type _WorkerOutputMessage = WorkerProgressMessage | WorkerCompleteMessage | WorkerErrorMessage

// Worker state
let index: SearchIndex | null = null
let processedCount = 0

/**
 * Process objects in chunks to avoid blocking even the worker thread
 * (useful for extremely large batches)
 */
async function processObjectsInChunks(
  objects: Array<{ key: string; size: number; last_modified?: string }>,
  chunkSize: number = 500 // Larger chunks in worker thread
): Promise<void> {
  if (!index) {
    throw new Error('Index not initialized. Call init first.')
  }

  for (let i = 0; i < objects.length; i += chunkSize) {
    const chunk = objects.slice(i, Math.min(i + chunkSize, objects.length))

    // Process this chunk
    for (const obj of chunk) {
      index.objects.push({
        key: obj.key,
        size: obj.size,
        lastModified: obj.last_modified ? new Date(obj.last_modified).getTime() : Date.now(),
        searchKey: obj.key.toLowerCase(), // Pre-lowercase for fast search
      })
    }

    processedCount += chunk.length
    index.totalObjects = processedCount

    // Send progress update after each chunk
    const progressMessage: WorkerProgressMessage = {
      type: 'progress',
      current: processedCount,
      total: processedCount, // We don't know total upfront
    }
    self.postMessage(progressMessage)

    // Yield to event loop between chunks (even in worker)
    if (i + chunkSize < objects.length) {
      await new Promise((resolve) => setTimeout(resolve, 0))
    }
  }
}

/**
 * Initialize a new index
 */
function initIndex(bucketName: string, profileId: string): void {
  index = {
    bucketName,
    profileId,
    lastBuilt: Date.now(),
    totalObjects: 0,
    sizeInBytes: 0,
    objects: [],
  }
  processedCount = 0
}

/**
 * Finalize index and calculate size
 */
function finalizeIndex(): SearchIndex {
  if (!index) {
    throw new Error('Index not initialized. Call init first.')
  }

  // Calculate index size in bytes
  const indexJson = JSON.stringify(index)
  const encoder = new TextEncoder()
  const indexBytes = encoder.encode(indexJson)
  index.sizeInBytes = indexBytes.length

  return index
}

/**
 * Message handler
 */
self.onmessage = async (e: MessageEvent<WorkerInputMessage>) => {
  try {
    const message = e.data

    switch (message.type) {
      case 'init': {
        initIndex(message.bucketName, message.profileId)
        break
      }

      case 'processBatch': {
        if (!index) {
          throw new Error('Index not initialized. Call init first.')
        }
        await processObjectsInChunks(message.objects)
        break
      }

      case 'finalize': {
        const finalIndex = finalizeIndex()
        const completeMessage: WorkerCompleteMessage = {
          type: 'complete',
          index: finalIndex,
        }
        self.postMessage(completeMessage)
        // Reset state
        index = null
        processedCount = 0
        break
      }

      default: {
        const exhaustiveCheck: never = message
        throw new Error(`Unknown message type: ${(exhaustiveCheck as any).type}`)
      }
    }
  } catch (error) {
    const errorMessage: WorkerErrorMessage = {
      type: 'error',
      error: error instanceof Error ? error.message : String(error),
    }
    self.postMessage(errorMessage)
  }
}

// Export empty object to make this a module
export {}
