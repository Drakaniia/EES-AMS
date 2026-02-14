import { invoke } from '@tauri-apps/api/core'

export interface UserProfile {
  id: number
  email: string
  display_name: string
  school_name: string
  position: string
  department: string
  employee_id: string
  organization_type: string
  organization_name: string
  created_at: string
  last_login: string
}

export interface AuthResponse {
  success: boolean
  user?: UserProfile
  token?: string
  message?: string
}

export interface LoginRequest {
  email: string
  password: string
}

export interface RegisterRequest {
  email: string
  password: string
  display_name: string
  school_name: string
}

// Organization detection utility
export const detectOrganization = (email: string): { type: string; name: string } => {
  if (!email || !email.includes('@')) {
    return { type: 'other', name: 'Unknown' }
  }

  const domain = email.split('@')[1]?.toLowerCase()
  if (!domain) {
    return { type: 'other', name: 'Unknown' }
  }

  // Check for organization patterns
  if (domain.endsWith('deped.gov.ph')) {
    return { type: 'government', name: 'Department of Education' }
  }
  
  if (domain.endsWith('ched.gov.ph')) {
    return { type: 'government', name: 'Commission on Higher Education' }
  }
  
  if (domain.endsWith('dost.gov.ph')) {
    return { type: 'government', name: 'Department of Science and Technology' }
  }
  
  if (domain.endsWith('.edu.ph') || domain.endsWith('.edu')) {
    return { 
      type: 'educational', 
      name: domain.split('.')[0]?.toUpperCase() || 'Educational Institution' 
    }
  }
  
  if (domain.endsWith('.gov.ph')) {
    return { 
      type: 'government', 
      name: domain.split('.')[0]?.toUpperCase() || 'Government Agency' 
    }
  }

  return { type: 'other', name: 'Organization' }
}

// Authentication service using Tauri backend
export const authService = {
  async register(request: RegisterRequest): Promise<AuthResponse> {
    try {
      const response = await invoke<AuthResponse>('auth_register', { request })
      if (response.success && response.token) {
        localStorage.setItem('auth_token', response.token)
        localStorage.setItem('user_profile', JSON.stringify(response.user))
      }
      return response
    } catch (error) {
      return {
        success: false,
        message: error instanceof Error ? error.message : 'Registration failed'
      }
    }
  },

  async login(request: LoginRequest): Promise<AuthResponse> {
    try {
      const response = await invoke<AuthResponse>('auth_login', { request })
      if (response.success && response.token) {
        localStorage.setItem('auth_token', response.token)
        localStorage.setItem('user_profile', JSON.stringify(response.user))
      }
      return response
    } catch (error) {
      return {
        success: false,
        message: error instanceof Error ? error.message : 'Login failed'
      }
    }
  },

  async validateToken(token: string): Promise<UserProfile | null> {
    try {
      const user = await invoke<UserProfile | null>('auth_validate_token', { token })
      if (user) {
        localStorage.setItem('user_profile', JSON.stringify(user))
      }
      return user
    } catch (error) {
      return null
    }
  },

  async getCurrentUser(): Promise<UserProfile | null> {
    try {
      const user = await invoke<UserProfile | null>('auth_get_current_user')
      if (user) {
        localStorage.setItem('user_profile', JSON.stringify(user))
      }
      return user
    } catch (error) {
      return null
    }
  },

  async updateUserProfile(profile: UserProfile): Promise<boolean> {
    try {
      await invoke('auth_update_profile', { profile })
      localStorage.setItem('user_profile', JSON.stringify(profile))
      return true
    } catch (error) {
      return false
    }
  },

  async logout(): Promise<void> {
    try {
      await invoke('auth_logout')
    } catch (error) {
      // Continue with local logout even if backend call fails
    } finally {
      localStorage.removeItem('auth_token')
      localStorage.removeItem('user_profile')
    }
  },

  // Helper methods
  getToken(): string | null {
    return localStorage.getItem('auth_token')
  },

  getStoredProfile(): UserProfile | null {
    const profile = localStorage.getItem('user_profile')
    return profile ? JSON.parse(profile) : null
  },

  isAuthenticated(): boolean {
    return !!this.getToken()
  }
}