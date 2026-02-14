import React from 'react';
import { useUpdateService } from '../hooks/useUpdateService';

interface UpdateNotificationProps {
  className?: string;
}

export const UpdateNotification: React.FC<UpdateNotificationProps> = ({ className = '' }) => {
  const {
    updateStatus,
    isUpdating,
    updateProgress,
    error,
    downloadAndInstall,
    restart,
    checkForUpdates,
  } = useUpdateService();

  if (!updateStatus && !isUpdating && !error) {
    return null;
  }

  const showUpdateAvailable = updateStatus?.available && !isUpdating;
  const showUpdateProgress = isUpdating || updateProgress;

  if (error) {
    return (
      <div className={`bg-red-50 border-l-4 border-red-400 p-4 mb-4 ${className}`}>
        <div className="flex">
          <div className="flex-shrink-0">
            <svg className="h-5 w-5 text-red-400" viewBox="0 0 20 20" fill="currentColor">
              <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clipRule="evenodd" />
            </svg>
          </div>
          <div className="ml-3">
            <p className="text-sm text-red-700">
              Update Error: {error}
            </p>
            <div className="mt-2">
              <button
                onClick={checkForUpdates}
                className="text-sm text-red-600 underline hover:text-red-800"
              >
                Try Again
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (showUpdateProgress) {
    return (
      <div className={`bg-blue-50 border-l-4 border-blue-400 p-4 mb-4 ${className}`}>
        <div className="flex">
          <div className="flex-shrink-0">
            <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-blue-600"></div>
          </div>
          <div className="ml-3 flex-1">
            <p className="text-sm text-blue-700 font-medium">
              {updateProgress}
            </p>
            {updateProgress.includes('Update installed') && (
              <div className="mt-2">
                <p className="text-sm text-blue-600">
                  Restarting application in 2 seconds...
                </p>
                <button
                  onClick={restart}
                  className="mt-1 text-sm text-blue-600 underline hover:text-blue-800"
                >
                  Restart Now
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
    );
  }

  if (showUpdateAvailable) {
    return (
      <div className={`bg-green-50 border-l-4 border-green-400 p-4 mb-4 ${className}`}>
        <div className="flex">
          <div className="flex-shrink-0">
            <svg className="h-5 w-5 text-green-400" viewBox="0 0 20 20" fill="currentColor">
              <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clipRule="evenodd" />
            </svg>
          </div>
          <div className="ml-3">
            <h3 className="text-sm font-medium text-green-800">
              Update Available
            </h3>
            <div className="mt-2 text-sm text-green-700">
              <p className="font-medium">
                Version {updateStatus.latest_version} is available
              </p>
              <p className="text-xs text-green-600">
                Current version: {updateStatus.current_version}
              </p>
              {updateStatus.body && (
                <div className="mt-1 text-xs text-green-600">
                  {updateStatus.body.split('\n').slice(0, 3).map((line: string, i: number) => (
                    <p key={i}>{line}</p>
                  ))}
                </div>
              )}
            </div>
            <div className="mt-3 flex space-x-2">
              <button
                onClick={downloadAndInstall}
                className="bg-green-600 text-white px-3 py-1 text-sm rounded hover:bg-green-700 transition-colors"
              >
                Update Now
              </button>
              <button
                onClick={() => {/* Later option - just dismiss */}}
                className="text-green-600 underline text-sm hover:text-green-800"
              >
                Later
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return null;
};

export default UpdateNotification;