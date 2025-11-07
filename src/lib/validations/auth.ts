import { z } from 'zod';

export const loginSchema = z.object({
  role: z.enum(['student', 'faculty', 'parent', 'admin'], {
    message: 'Please select a role',
  }),
  userId: z
    .string()
    .min(1, 'User ID is required')
    .min(5, 'User ID must be at least 5 characters'),
  password: z
    .string()
    .min(1, 'Password is required')
    .min(8, 'Password must be at least 8 characters'),
});

export type LoginFormData = z.infer<typeof loginSchema>;