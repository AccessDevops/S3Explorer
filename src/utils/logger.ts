/**
 * Logging utility with environment-aware behavior
 * - Development: All logs to console
 * - Production: Only errors logged (can be extended with external service like Sentry)
 */

const isDev = import.meta.env.DEV

export const logger = {
  /**
   * Debug logging - only in development
   */
  debug(message: string, ...args: any[]) {
    if (isDev) {
      console.log(`[DEBUG] ${message}`, ...args)
    }
  },

  /**
   * Info logging - only in development
   */
  info(message: string, ...args: any[]) {
    if (isDev) {
      console.info(`[INFO] ${message}`, ...args)
    }
  },

  /**
   * Warning logging - always logged
   */
  warn(message: string, ...args: any[]) {
    console.warn(`[WARN] ${message}`, ...args)
    // TODO: Send to error tracking service in production
  },

  /**
   * Error logging - always logged
   */
  error(message: string, error?: any, ...args: any[]) {
    console.error(`[ERROR] ${message}`, error, ...args)
    // TODO: Send to error tracking service (e.g., Sentry) in production
  },
}
