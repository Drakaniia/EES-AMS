import { FC, useState } from 'react'
import { useAuth } from '../contexts/AuthContext'
import { UserProfile } from '../lib/auth-tauri'

const ProfileSettings: FC = () => {
    const { userProfile, updateUserProfile, signOut } = useAuth()
    const [isLoading, setIsLoading] = useState(false)
    const [activeTab, setActiveTab] = useState<'profile' | 'account' | 'notifications'>('profile')
    
    const [formData, setFormData] = useState<Partial<UserProfile>>({
        display_name: userProfile?.display_name || '',
        school_name: userProfile?.school_name || ''
    })

    const handleInputChange = (field: keyof UserProfile, value: string) => {
        setFormData(prev => ({ ...prev, [field]: value }))
    }

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault()
        if (!userProfile) return
        
        setIsLoading(true)
        
        try {
            const updatedProfile: UserProfile = {
                ...userProfile,
                display_name: formData.display_name || userProfile.display_name,
                school_name: formData.school_name || userProfile.school_name,
            }
            
            await updateUserProfile(updatedProfile)
            // Show success message
            alert('Profile updated successfully!')
        } catch (error) {
            console.error('Failed to update profile:', error)
            alert('Failed to update profile. Please try again.')
        } finally {
            setIsLoading(false)
        }
    }

    const handleSignOut = async () => {
        if (confirm('Are you sure you want to sign out?')) {
            try {
                await signOut()
            } catch (error) {
                console.error('Failed to sign out:', error)
                alert('Failed to sign out. Please try again.')
            }
        }
    }

    if (!userProfile) {
        return (
            <div className="flex items-center justify-center h-full">
                <div className="w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
            </div>
        )
    }

    return (
        <div className="space-y-6">
            {/* Header */}
            <div>
                <h1 className="text-2xl font-bold text-white mb-1">Profile Settings</h1>
                <p className="text-gray-400">Manage your account settings and preferences</p>
            </div>

            <div className="glass rounded-2xl overflow-hidden">
                {/* Tabs */}
                <div className="flex border-b border-white/10">
                    <button
                        onClick={() => setActiveTab('profile')}
                        className={`flex-1 px-6 py-4 text-sm font-medium transition-colors ${
                            activeTab === 'profile'
                                ? 'text-white border-b-2 border-blue-500 bg-blue-500/10'
                                : 'text-gray-400 hover:text-white hover:bg-white/5'
                        }`}
                    >
                        Personal Information
                    </button>
                    <button
                        onClick={() => setActiveTab('account')}
                        className={`flex-1 px-6 py-4 text-sm font-medium transition-colors ${
                            activeTab === 'account'
                                ? 'text-white border-b-2 border-blue-500 bg-blue-500/10'
                                : 'text-gray-400 hover:text-white hover:bg-white/5'
                        }`}
                    >
                        Account Details
                    </button>
                    <button
                        onClick={() => setActiveTab('notifications')}
                        className={`flex-1 px-6 py-4 text-sm font-medium transition-colors ${
                            activeTab === 'notifications'
                                ? 'text-white border-b-2 border-blue-500 bg-blue-500/10'
                                : 'text-gray-400 hover:text-white hover:bg-white/5'
                        }`}
                    >
                        Notifications
                    </button>
                </div>

                <div className="p-6">
                    {activeTab === 'profile' && (
                        <form onSubmit={handleSubmit} className="space-y-6">
                            {/* Profile Picture Section */}
                            <div className="flex items-center gap-6 pb-6 border-b border-white/10">
                                <div className="w-24 h-24 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center">
                                    <span className="text-3xl text-white font-semibold">
                                        {userProfile.display_name.charAt(0).toUpperCase()}
                                    </span>
                                </div>
                                <div>
                                    <h3 className="text-lg font-medium text-white mb-1">Profile Picture</h3>
                                    <p className="text-sm text-gray-400 mb-3">Upload a profile picture</p>
                                    <button type="button" className="btn btn-secondary text-sm">
                                        Change Photo
                                    </button>
                                </div>
                            </div>

                            {/* Personal Information */}
                            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                                <div>
                                    <label className="block text-sm font-medium text-gray-300 mb-2">
                                        Display Name
                                    </label>
                                    <input
                                        type="text"
                                        value={formData.display_name}
                                        onChange={(e) => handleInputChange('display_name', e.target.value)}
                                        className="input w-full"
                                    />
                                </div>
                                <div>
                                    <label className="block text-sm font-medium text-gray-300 mb-2">
                                        Email Address
                                    </label>
                                    <input
                                        type="email"
                                        value={userProfile.email}
                                        disabled
                                        className="input w-full disabled:opacity-50"
                                    />
                                </div>
                            </div>

                            <div className="space-y-4">
                                <h3 className="text-lg font-medium text-white">Organization</h3>
                                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                                    <div>
                                        <label className="block text-sm font-medium text-gray-300 mb-2">
                                            Organization Name
                                        </label>
                                        <div className="input w-full bg-blue-500/10 border-blue-500/30 text-blue-400 pointer-events-none">
                                            {userProfile.organization_name}
                                        </div>
                                    </div>
                                    <div>
                                        <label className="block text-sm font-medium text-gray-300 mb-2">
                                            Type
                                        </label>
                                        <div className="input w-full capitalize bg-blue-500/10 border-blue-500/30 text-blue-400 pointer-events-none">
                                            {userProfile.organization_type}
                                        </div>
                                    </div>
                                </div>
                            </div>

                            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                                <div>
                                    <label className="block text-sm font-medium text-gray-300 mb-2">
                                        Position
                                    </label>
                                    <input
                                        type="text"
                                        value={formData.position}
                                        onChange={(e) => handleInputChange('position', e.target.value)}
                                        placeholder="e.g., Head Teacher, Faculty"
                                        className="input w-full"
                                    />
                                </div>
                                <div>
                                    <label className="block text-sm font-medium text-gray-300 mb-2">
                                        Department
                                    </label>
                                    <input
                                        type="text"
                                        value={formData.department}
                                        onChange={(e) => handleInputChange('department', e.target.value)}
                                        placeholder="e.g., Science, Mathematics"
                                        className="input w-full"
                                    />
</div>

                            <div>
                                <label className="block text-sm font-medium text-gray-300 mb-2">
                                    School Name
                                </label>
                                <input
                                    type="text"
                                    value={formData.school_name}
                                    onChange={(e) => handleInputChange('school_name', e.target.value)}
                                    placeholder="School or institution name"
                                    className="input w-full"
                                />
                            </div>
                                <div>
                                    <label className="block text-sm font-medium text-gray-300 mb-2">
                                        Employee ID
                                    </label>
                                    <input
                                        type="text"
                                        value={formData.employee_id}
                                        onChange={(e) => handleInputChange('employee_id', e.target.value)}
                                        placeholder="Employee or staff ID"
                                        className="input w-full"
                                    />
                                </div>
                            </div>

                            <div className="flex justify-end gap-4">
                                <button type="button" className="btn btn-secondary">
                                    Cancel
                                </button>
                                <button type="submit" disabled={isLoading} className="btn btn-primary">
                                    {isLoading ? 'Saving...' : 'Save Changes'}
                                </button>
                            </div>
                        </form>
                    )}

                    {activeTab === 'account' && (
                        <div className="space-y-6">
                            <div className="glass rounded-xl p-6">
                                <h3 className="text-lg font-medium text-white mb-4">Account Information</h3>
                                <div className="space-y-4">
                                    <div className="flex justify-between items-center">
                                        <div>
                                            <p className="text-sm text-gray-300">Account ID</p>
                                            <p className="text-white font-mono text-sm">{userProfile.id}</p>
                                        </div>
                                        <button className="btn btn-ghost text-xs">Copy</button>
                                    </div>
                                    <div className="flex justify-between items-center py-3 border-t border-white/10">
                                        <div>
                                            <p className="text-sm text-gray-300">Email Address</p>
                                            <p className="text-white">{userProfile.email}</p>
                                        </div>
                                        <button className="btn btn-secondary text-xs">Change Email</button>
                                    </div>
                                    <div className="flex justify-between items-center py-3 border-t border-white/10">
                                        <div>
                                            <p className="text-sm text-gray-300">Password</p>
                                            <p className="text-white">••••••••</p>
                                        </div>
                                        <button className="btn btn-secondary text-xs">Change Password</button>
                                    </div>
                                    <div className="flex justify-between items-center py-3 border-t border-white/10">
                                        <div>
                                            <p className="text-sm text-gray-300">Account Created</p>
                                            <p className="text-white">
                                                {new Date(userProfile.created_at).toLocaleDateString()}
                                            </p>
                                        </div>
                                    </div>
                                    <div className="flex justify-between items-center py-3 border-t border-white/10">
                                        <div>
                                            <p className="text-sm text-gray-300">Last Login</p>
                                            <p className="text-white">
                                                {new Date(userProfile.last_login).toLocaleDateString()}
                                            </p>
                                        </div>
                                    </div>
                                </div>
                            </div>

                            <div className="glass rounded-xl p-6">
                                <h3 className="text-lg font-medium text-white mb-4">Danger Zone</h3>
                                <p className="text-sm text-gray-400 mb-4">
                                    These actions are irreversible. Please proceed with caution.
                                </p>
                                <div className="space-y-3">
                                    <button className="btn btn-secondary w-full">
                                        Export All Data
                                    </button>
                                    <button 
                                        onClick={handleSignOut}
                                        className="btn btn-secondary w-full"
                                    >
                                        Sign Out
                                    </button>
                                </div>
                            </div>
                        </div>
                    )}

                    {activeTab === 'notifications' && (
                        <div className="space-y-6">
                            <div className="glass rounded-xl p-6">
                                <h3 className="text-lg font-medium text-white mb-4">Notification Preferences</h3>
                                <div className="space-y-4">
                                    <div className="flex items-center justify-between">
                                        <div>
                                            <p className="text-white font-medium">Email Notifications</p>
                                            <p className="text-sm text-gray-400">Receive notifications via email</p>
                                        </div>
                                        <label className="relative inline-flex items-center cursor-pointer">
                                            <input type="checkbox" className="sr-only peer" defaultChecked />
                                            <div className="w-11 h-6 bg-gray-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500"></div>
                                        </label>
                                    </div>
                                    <div className="flex items-center justify-between">
                                        <div>
                                            <p className="text-white font-medium">Attendance Reminders</p>
                                            <p className="text-sm text-gray-400">Daily reminders to take attendance</p>
                                        </div>
                                        <label className="relative inline-flex items-center cursor-pointer">
                                            <input type="checkbox" className="sr-only peer" defaultChecked />
                                            <div className="w-11 h-6 bg-gray-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500"></div>
                                        </label>
                                    </div>
                                    <div className="flex items-center justify-between">
                                        <div>
                                            <p className="text-white font-medium">Sync Notifications</p>
                                            <p className="text-sm text-gray-400">Notifications for Google Sync status</p>
                                        </div>
                                        <label className="relative inline-flex items-center cursor-pointer">
                                            <input type="checkbox" className="sr-only peer" />
                                            <div className="w-11 h-6 bg-gray-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500"></div>
                                        </label>
                                    </div>
                                    <div className="flex items-center justify-between">
                                        <div>
                                            <p className="text-white font-medium">Weekly Reports</p>
                                            <p className="text-sm text-gray-400">Receive weekly attendance summaries</p>
                                        </div>
                                        <label className="relative inline-flex items-center cursor-pointer">
                                            <input type="checkbox" className="sr-only peer" />
                                            <div className="w-11 h-6 bg-gray-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500"></div>
                                        </label>
                                    </div>
                                </div>
                            </div>

                            <div className="glass rounded-xl p-6">
                                <h3 className="text-lg font-medium text-white mb-4">App Preferences</h3>
                                <div className="space-y-4">
                                    <div>
                                        <label className="block text-sm font-medium text-gray-300 mb-2">
                                            Default View
                                        </label>
                                        <select className="input w-full">
                                            <option>Dashboard</option>
                                            <option>Attendance</option>
                                            <option>Classes</option>
                                            <option>Students</option>
                                        </select>
                                    </div>
                                    <div>
                                        <label className="block text-sm font-medium text-gray-300 mb-2">
                                            Language
                                        </label>
                                        <select className="input w-full">
                                            <option>English</option>
                                            <option>Filipino</option>
                                        </select>
                                    </div>
                                    <div>
                                        <label className="block text-sm font-medium text-gray-300 mb-2">
                                            Date Format
                                        </label>
                                        <select className="input w-full">
                                            <option>MM/DD/YYYY</option>
                                            <option>DD/MM/YYYY</option>
                                            <option>YYYY-MM-DD</option>
                                        </select>
                                    </div>
                                </div>
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div>
    )
}

export default ProfileSettings