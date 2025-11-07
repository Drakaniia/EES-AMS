'use server';

import { loginSchema, type LoginFormData } from '@/lib/validations/auth';

export async function loginAction(data: LoginFormData) {
  try {
    const validatedData = loginSchema.parse(data);

    if (
      validatedData.userId === 'demo' &&
      validatedData.password === 'password123'
    ) {
      const user = {
        id: '1',
        userId: validatedData.userId,
        role: validatedData.role,
      };

      // Generate a simple token (in production, use JWT)
      const token = btoa(JSON.stringify({ ...user, exp: Date.now() + 86400000 }));

      return {
        success: true,
        message: 'Login successful',
        user,
        token,
      };
    }

    return {
      success: false,
      message: 'Invalid credentials',
    };
  } catch (_error) {
    return {
      success: false,
      message: 'An error occurred during login',
    };
  }
}