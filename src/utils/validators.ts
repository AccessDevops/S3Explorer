/**
 * Input validation utilities
 */

export interface ValidationResult {
  valid: boolean
  error?: string
}

/**
 * Validate S3 bucket name according to AWS rules
 * - Must be between 3 and 63 characters
 * - Can only contain lowercase letters, numbers, dots (.), and hyphens (-)
 * - Must begin and end with a letter or number
 * - Must not be formatted as an IP address
 */
export function validateBucketName(name: string): ValidationResult {
  if (!name || name.trim().length === 0) {
    return { valid: false, error: 'Bucket name cannot be empty' }
  }

  const trimmedName = name.trim()

  if (trimmedName.length < 3) {
    return { valid: false, error: 'Bucket name must be at least 3 characters long' }
  }

  if (trimmedName.length > 63) {
    return { valid: false, error: 'Bucket name must be no more than 63 characters long' }
  }

  // Check if formatted as IP address
  const ipRegex = /^(\d{1,3}\.){3}\d{1,3}$/
  if (ipRegex.test(trimmedName)) {
    return { valid: false, error: 'Bucket name cannot be formatted as an IP address' }
  }

  // Check valid characters and format
  const bucketNameRegex = /^[a-z0-9][a-z0-9.-]*[a-z0-9]$/
  if (!bucketNameRegex.test(trimmedName)) {
    return {
      valid: false,
      error:
        'Bucket name can only contain lowercase letters, numbers, dots, and hyphens, and must start and end with a letter or number',
    }
  }

  // Check for consecutive periods
  if (trimmedName.includes('..')) {
    return { valid: false, error: 'Bucket name cannot contain consecutive periods' }
  }

  // Check for period-dash combinations
  if (trimmedName.includes('.-') || trimmedName.includes('-.')) {
    return {
      valid: false,
      error: 'Bucket name cannot contain periods adjacent to hyphens',
    }
  }

  return { valid: true }
}

/**
 * Validate S3 object key
 * - Cannot be empty
 * - Maximum 1024 characters
 * - Should not contain certain characters that can cause issues
 */
export function validateObjectKey(key: string): ValidationResult {
  if (!key || key.trim().length === 0) {
    return { valid: false, error: 'Object key cannot be empty' }
  }

  const trimmedKey = key.trim()

  if (trimmedKey.length > 1024) {
    return { valid: false, error: 'Object key must be no more than 1024 characters long' }
  }

  // Check for potentially problematic characters
  // eslint-disable-next-line no-control-regex
  const invalidChars = /[\x00-\x1F\x7F]/
  if (invalidChars.test(trimmedKey)) {
    return { valid: false, error: 'Object key contains invalid control characters' }
  }

  return { valid: true }
}

/**
 * Validate endpoint URL
 * - Must be a valid URL format
 * - Must use http or https protocol
 */
export function validateEndpoint(endpoint: string): ValidationResult {
  if (!endpoint || endpoint.trim().length === 0) {
    return { valid: true } // Empty endpoint is valid (will use AWS default)
  }

  const trimmedEndpoint = endpoint.trim()

  try {
    const url = new URL(trimmedEndpoint)

    if (url.protocol !== 'http:' && url.protocol !== 'https:') {
      return { valid: false, error: 'Endpoint must use http:// or https:// protocol' }
    }

    if (!url.hostname) {
      return { valid: false, error: 'Endpoint must have a valid hostname' }
    }

    return { valid: true }
  } catch (e) {
    return { valid: false, error: 'Invalid endpoint URL format' }
  }
}

/**
 * Validate region name
 * - Cannot be empty
 * - Must match AWS region format
 */
export function validateRegion(region: string): ValidationResult {
  if (!region || region.trim().length === 0) {
    return { valid: false, error: 'Region cannot be empty' }
  }

  const trimmedRegion = region.trim()

  // AWS region format: us-east-1, eu-west-2, etc.
  const regionRegex = /^[a-z]{2}-[a-z]+-\d+$/
  if (!regionRegex.test(trimmedRegion)) {
    return {
      valid: false,
      error: 'Region must match AWS format (e.g., us-east-1, eu-west-2)',
    }
  }

  return { valid: true }
}

/**
 * Sanitize object key by removing/replacing invalid characters
 */
export function sanitizeObjectKey(key: string): string {
  if (!key) return ''

  return (
    key
      // Remove control characters
      // eslint-disable-next-line no-control-regex
      .replace(/[\x00-\x1F\x7F]/g, '')
      // Trim
      .trim()
      // Limit length
      .substring(0, 1024)
  )
}

/**
 * Sanitize bucket name to make it valid
 */
export function sanitizeBucketName(name: string): string {
  if (!name) return ''

  return (
    name
      // Convert to lowercase
      .toLowerCase()
      // Remove invalid characters
      .replace(/[^a-z0-9.-]/g, '')
      // Remove consecutive periods
      .replace(/\.{2,}/g, '.')
      // Remove period-dash combinations
      .replace(/\.-|-\./g, '-')
      // Ensure starts with alphanumeric
      .replace(/^[^a-z0-9]+/, '')
      // Ensure ends with alphanumeric
      .replace(/[^a-z0-9]+$/, '')
      // Limit length
      .substring(0, 63)
  )
}
