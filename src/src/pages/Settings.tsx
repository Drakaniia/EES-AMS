import { FC, useEffect, useState } from 'react'
import type { SyncStatus } from '../lib/tauri'
import { useUpdateService } from '../hooks/useUpdateService'

const Settings: FC = () => {
    const [isAuthenticated, setIsAuthenticated] = useState(false)
    const [syncStatus, setSyncStatus] = useState<SyncStatus | null>(null)
    const [authCode, setAuthCode] = useState('')
    const [isAuthenticating, setIsAuthenticating] = useState(false)
    const [isSyncing, setIsSyncing] = useState(false)
    const [credentials, setCredentials] = useState({
        clientId: '',
        clientSecret: '',
        redirectUri: 'http://localhost'
})

            {/* Auto-Update Settings Section */}
            <div className="glass rounded-2xl p-6">
                <h2 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
                    <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none">
                        <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z" fill="#10B981" />
                    </svg>
                    Application Updates
                </h2>

                <div className="space-y-4">
                    {/* Auto-update toggle */}
                    <div className="flex items-center justify-between">
                        <div>
                            <p className="text-white font-medium">Check for Updates Automatically</p>
                            <p className="text-gray-400 text-sm">Enable automatic update checks on app startup</p>
                        </div>
                        <label className="relative inline-flex items-center cursor-pointer">
                            <input
                                type="checkbox"
                                className="sr-only peer"
                                checked={autoUpdateEnabled}
                                onChange={(e) => setAutoUpdateEnabled(e.target.checked)}
                            />
                            <div className="w-11 h-6 bg-gray-600 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
                        </label>
                    </div>

                    {/* Current version info */}
                    <div className="glass rounded-xl p-4 space-y-2">
                        <div className="flex items-center justify-between">
                            <span className="text-gray-400">Current Version</span>
                            <span className="text-white font-medium">
                                {updateStatus?.current_version || '1.0.0'}
                            </span>
                        </div>
                        
                        {updateStatus?.available && (
                            <div className="flex items-center justify-between">
                                <span className="text-gray-400">Update Available</span>
                                <span className="text-green-400 font-medium">
                                    {updateStatus.latest_version}
                                </span>
                            </div>
                        )}
                    </div>

                    {/* Update controls */}
                    <div className="flex gap-3">
                        <button
                            onClick={checkForUpdates}
                            disabled={isUpdating}
                            className="btn btn-secondary"
                        >
                            Check for Updates
                        </button>

                        {updateStatus?.available && !isUpdating && (
                            <button
                                onClick={downloadAndInstall}
                                className="btn btn-primary"
                            >
                                Install Update
                            </button>
                        )}

                        {updateProgress && (
                            <div className="flex-1 glass rounded-xl p-4">
                                <p className="text-sm text-blue-400">{updateProgress}</p>
                            </div>
                        )}
                    </div>

                    {error && (
                        <div className="glass rounded-xl p-4 border border-red-500/20 bg-red-500/5">
                            <p className="text-red-400 text-sm">{error}</p>
                        </div>
                    )}
                </div>
            </div>

            {/* About Section */}
            <div className="glass rounded-2xl p-6">
                <h2 className="text-lg font-semibold text-white mb-4">About</h2>
                <div className="space-y-2 text-sm text-gray-400">
                    <p><strong className="text-white">AttendEase</strong> - Attendance Management System</p>
                    <p>Version 1.0.0</p>
                    <p className="pt-4">
                        A desktop application for managing student attendance with offline-first
                        architecture and Google Sheets sync for backup and reporting.
                    </p>
                </div>
            </div>

            {/* Google Sync Section */}
            <div className="glass rounded-2xl p-6">
                <h2 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
                    <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none">
                        <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" fill="#4285F4" />
                        <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853" />
                        <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" fill="#FBBC05" />
                        <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" fill="#EA4335" />
                    </svg>
                    Google Sheets Sync
                </h2>

                {!isAuthenticated ? (
                    <div className="space-y-4">
                        {!showCredentialsForm ? (
                            <>
                                <p className="text-gray-400 text-sm">
                                    Connect your Google account to automatically sync attendance data to Google Sheets.
                                </p>
                                <div className="glass rounded-xl p-4 border border-yellow-500/20 bg-yellow-500/5">
                                    <p className="text-yellow-400 text-sm">
                                        <strong>Setup Required:</strong> You need to create a Google Cloud project and get OAuth credentials first.
                                    </p>
                                </div>
                                <button onClick={() => setShowCredentialsForm(true)} className="btn btn-primary">
                                    Configure Google API Credentials
                                </button>
                            </>
                        ) : (
                            <div className="space-y-4">
                                <p className="text-gray-400 text-sm">
                                    Enter your OAuth 2.0 credentials from Google Cloud Console.
                                </p>
                                <div>
                                    <label className="block text-sm text-gray-400 mb-2">Client ID</label>
                                    <input
                                        type="text"
                                        value={credentials.clientId}
                                        onChange={(e) => setCredentials({ ...credentials, clientId: e.target.value })}
                                        placeholder="your-client-id.apps.googleusercontent.com"
                                        className="input"
                                    />
                                </div>
                                <div>
                                    <label className="block text-sm text-gray-400 mb-2">Client Secret</label>
                                    <input
                                        type="password"
                                        value={credentials.clientSecret}
                                        onChange={(e) => setCredentials({ ...credentials, clientSecret: e.target.value })}
                                        placeholder="Your client secret"
                                        className="input"
                                    />
                                </div>
                                <div className="flex gap-3">
                                    <button onClick={() => setShowCredentialsForm(false)} className="btn btn-secondary">
                                        Cancel
                                    </button>
                                    <button onClick={handleSaveCredentials} className="btn btn-primary">
                                        Save Credentials
                                    </button>
                                </div>
                            </div>
                        )}

                        {isAuthenticating && (
                            <div className="mt-4 space-y-4">
                                <p className="text-gray-400 text-sm">
                                    A browser window should have opened. Sign in with Google and paste the authorization code below:
                                </p>
                                <div className="flex gap-2">
                                    <input
                                        type="text"
                                        value={authCode}
                                        onChange={(e) => setAuthCode(e.target.value)}
                                        placeholder="Paste authorization code here"
                                        className="input flex-1"
                                    />
                                    <button onClick={handleSubmitCode} className="btn btn-primary">
                                        Submit
                                    </button>
                                </div>
                            </div>
                        )}
                    </div>
                ) : (
                    <div className="space-y-4">
                        <div className="flex items-center gap-3">
                            <div className="w-3 h-3 rounded-full bg-green-500"></div>
                            <span className="text-green-400">Connected to Google</span>
                        </div>

                        {syncStatus && (
                            <div className="grid grid-cols-2 gap-4 text-sm">
                                <div className="glass rounded-xl p-4">
                                    <p className="text-gray-400 mb-1">Last Sync</p>
                                    <p className="text-white font-medium">{formatDate(syncStatus.last_sync_time)}</p>
                                </div>
                                <div className="glass rounded-xl p-4">
                                    <p className="text-gray-400 mb-1">Pending Records</p>
                                    <p className="text-white font-medium">{syncStatus.pending_records}</p>
                                </div>
                            </div>
                        )}

                        <div className="flex gap-3">
                            <button
                                onClick={handleManualSync}
                                disabled={isSyncing}
                                className="btn btn-primary"
                            >
                                {isSyncing ? (
                                    <span className="flex items-center gap-2">
                                        <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"></div>
                                        Syncing...
                                    </span>
                                ) : (
                                    'Sync Now'
                                )}
                            </button>
                            <button onClick={handleLogout} className="btn btn-secondary">
                                Disconnect
                            </button>
                        </div>
                    </div>
                )}
            </div>

            {/* About Section */}
            <div className="glass rounded-2xl p-6">
                <h2 className="text-lg font-semibold text-white mb-4">About</h2>
                <div className="space-y-2 text-sm text-gray-400">
                    <p><strong className="text-white">AttendEase</strong> - Attendance Management System</p>
                    <p>Version 1.0.0</p>
                    <p className="pt-4">
                        A desktop application for managing student attendance with offline-first
                        architecture and Google Sheets sync for backup and reporting.
                    </p>
                </div>
            </div>
        </div>
    )
}

export default Settings
