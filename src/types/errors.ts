/**
 * Structured error types for better error handling
 */

export enum ErrorCode {
  // Network errors
  NETWORK_ERROR = 'NETWORK_ERROR',
  TIMEOUT = 'TIMEOUT',

  // S3 errors
  BUCKET_NOT_FOUND = 'BUCKET_NOT_FOUND',
  OBJECT_NOT_FOUND = 'OBJECT_NOT_FOUND',
  ACCESS_DENIED = 'ACCESS_DENIED',
  INVALID_CREDENTIALS = 'INVALID_CREDENTIALS',

  // Validation errors
  INVALID_BUCKET_NAME = 'INVALID_BUCKET_NAME',
  INVALID_OBJECT_KEY = 'INVALID_OBJECT_KEY',
  INVALID_ENDPOINT = 'INVALID_ENDPOINT',

  // Application errors
  PROFILE_NOT_FOUND = 'PROFILE_NOT_FOUND',
  NO_PROFILE_SELECTED = 'NO_PROFILE_SELECTED',
  NO_BUCKET_SELECTED = 'NO_BUCKET_SELECTED',

  // Generic
  UNKNOWN_ERROR = 'UNKNOWN_ERROR',
}

export class AppError extends Error {
  constructor(
    public code: ErrorCode,
    message: string,
    public details?: any
  ) {
    super(message)
    this.name = 'AppError'

    // Maintains proper stack trace
    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, AppError)
    }
  }

  /**
   * Create AppError from unknown error
   */
  static fromUnknown(error: unknown, fallbackMessage = 'An error occurred'): AppError {
    if (error instanceof AppError) {
      return error
    }

    if (error instanceof Error) {
      // Try to infer error code from message
      const code = inferErrorCode(error.message)
      return new AppError(code, error.message, error)
    }

    if (typeof error === 'string') {
      const code = inferErrorCode(error)
      return new AppError(code, error)
    }

    return new AppError(ErrorCode.UNKNOWN_ERROR, fallbackMessage, error)
  }

  /**
   * Get user-friendly message
   */
  getUserMessage(t?: (key: string) => string): string {
    if (!t) return this.message

    // Map error codes to translation keys
    const errorKeyMap: Record<ErrorCode, string> = {
      [ErrorCode.NETWORK_ERROR]: 'errorNetwork',
      [ErrorCode.TIMEOUT]: 'errorTimeout',
      [ErrorCode.BUCKET_NOT_FOUND]: 'errorBucketNotFound',
      [ErrorCode.OBJECT_NOT_FOUND]: 'errorObjectNotFound',
      [ErrorCode.ACCESS_DENIED]: 'errorAccessDenied',
      [ErrorCode.INVALID_CREDENTIALS]: 'errorInvalidCredentials',
      [ErrorCode.INVALID_BUCKET_NAME]: 'errorInvalidBucketName',
      [ErrorCode.INVALID_OBJECT_KEY]: 'errorInvalidObjectKey',
      [ErrorCode.INVALID_ENDPOINT]: 'errorInvalidEndpoint',
      [ErrorCode.PROFILE_NOT_FOUND]: 'errorProfileNotFound',
      [ErrorCode.NO_PROFILE_SELECTED]: 'errorNoProfileSelected',
      [ErrorCode.NO_BUCKET_SELECTED]: 'errorNoBucketSelected',
      [ErrorCode.UNKNOWN_ERROR]: 'errorUnknown',
    }

    const key = errorKeyMap[this.code]
    return key ? t(key) : this.message
  }
}

/**
 * Infer error code from error message
 */
function inferErrorCode(message: string): ErrorCode {
  const lowerMessage = message.toLowerCase()

  if (lowerMessage.includes('network') || lowerMessage.includes('connection')) {
    return ErrorCode.NETWORK_ERROR
  }
  if (lowerMessage.includes('timeout')) {
    return ErrorCode.TIMEOUT
  }
  if (lowerMessage.includes('bucket') && lowerMessage.includes('not found')) {
    return ErrorCode.BUCKET_NOT_FOUND
  }
  if (lowerMessage.includes('access denied') || lowerMessage.includes('forbidden')) {
    return ErrorCode.ACCESS_DENIED
  }
  if (lowerMessage.includes('credentials') || lowerMessage.includes('authentication')) {
    return ErrorCode.INVALID_CREDENTIALS
  }

  return ErrorCode.UNKNOWN_ERROR
}

/**
 * Helper to wrap async operations with error handling
 */
export async function withErrorHandling<T>(
  operation: () => Promise<T>,
  errorMessage?: string
): Promise<T> {
  try {
    return await operation()
  } catch (error) {
    throw AppError.fromUnknown(error, errorMessage)
  }
}
