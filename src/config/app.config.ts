/**
 * Application Configuration
 * Centralized configuration for application
 */

export const appConfig = {
  name: 'Bukidnon State University Portal',
  description: 'Official portal for students, faculty, and staff',
  url: process.env.NEXT_PUBLIC_APP_URL || 'http://localhost:3000',
  api: {
    baseURL: process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000/api',
    timeout: 10000,
  },
  auth: {
    tokenKey: 'auth_token',
    refreshTokenKey: 'refresh_token',
  },
} as const;