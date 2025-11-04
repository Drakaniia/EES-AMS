/**
 * Global TypeScript Types
 */

export type UserRole = 'student' | 'faculty' | 'staff' | 'admin';

export interface User {
  id: string;
  userId: string;
  email?: string;
  name?: string;
  role: UserRole;
  createdAt: string;
  updatedAt: string;
}

export interface AuthResponse {
  user: User;
  token: string;
  refreshToken: string;
}

export interface ApiResponse<T = unknown> {
  success: boolean;
  data?: T;
  message?: string;
  error?: string;
}
