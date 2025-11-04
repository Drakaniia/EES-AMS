'use client';

import { useCallback } from 'react';

type ToastType = 'success' | 'error' | 'warning' | 'info';

export function useToast() {
  const toast = useCallback((message: string, type: ToastType = 'info') => {
    // Implement toast notification
    console.log(`[${type.toUpperCase()}] ${message}`);
  }, []);

  return { toast };
}
