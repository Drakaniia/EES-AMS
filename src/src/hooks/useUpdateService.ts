import { useState, useEffect } from 'react';
import { tauriAPI } from '../lib/tauri';

interface UpdateStatus {
  available: boolean;
  current_version: string;
  latest_version?: string;
  body?: string;
}

interface UpdateService {
  checkForUpdates: () => Promise<void>;
  downloadAndInstall: () => Promise<void>;
  restart: () => Promise<void>;
  updateStatus: UpdateStatus | null;
  isUpdating: boolean;
  updateProgress: string;
  error: string | null;
}

export const useUpdateService = (): UpdateService => {
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus | null>(null);
  const [isUpdating, setIsUpdating] = useState(false);
  const [updateProgress, setUpdateProgress] = useState('');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Check for updates on component mount
    checkForUpdates();

    // Set up update progress listener
    tauriAPI.updater.onUpdateProgress((progress) => {
      setUpdateProgress(progress);
    });

    // Periodic check for updates (every 4 hours)
    const interval = setInterval(checkForUpdates, 4 * 60 * 60 * 1000);

    return () => clearInterval(interval);
  }, []);

  const checkForUpdates = async () => {
    try {
      setError(null);
      const response = await tauriAPI.updater.checkForUpdates();
      if (response.success && response.data) {
        setUpdateStatus(response.data);
      } else {
        setError(response.error || 'Failed to check for updates');
      }
    } catch (err) {
      setError('Network error while checking for updates');
    }
  };

  const downloadAndInstall = async () => {
    try {
      setIsUpdating(true);
      setError(null);
      setUpdateProgress('Starting download...');

      const response = await tauriAPI.updater.downloadAndInstall();
      
      if (response.success) {
        setUpdateProgress(response.data || 'Update installed successfully');
        setTimeout(() => {
          // Auto-restart after 2 seconds
          restart();
        }, 2000);
      } else {
        setError(response.error || 'Failed to download update');
        setIsUpdating(false);
      }
    } catch (err) {
      setError('Network error during update download');
      setIsUpdating(false);
    }
  };

  const restart = async () => {
    try {
      await tauriAPI.updater.restart();
    } catch (err) {
      setError('Failed to restart application');
    }
  };

  return {
    checkForUpdates,
    downloadAndInstall,
    restart,
    updateStatus,
    isUpdating,
    updateProgress,
    error,
  };
};