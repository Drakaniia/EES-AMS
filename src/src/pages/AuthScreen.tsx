import { FC, useState } from 'react'
import { useAuth } from '../contexts/AuthContext'

const AuthScreen: FC = () => {
    const [isLogin, setIsLogin] = useState(true)
    const [email, setEmail] = useState('')
    const [password, setPassword] = useState('')
    const [displayName, setDisplayName] = useState('')
    const [schoolName, setSchoolName] = useState('')
    const [isLoading, setIsLoading] = useState(false)
    const [error, setError] = useState('')
    const [isGoogleLoading, setIsGoogleLoading] = useState(false)

    const { signIn, signUp } = useAuth()

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault()
        setError('')
        setIsLoading(true)

        try {
            if (isLogin) {
                await signIn(email, password)
            } else {
                await signUp({
                    email,
                    password,
                    displayName,
                    schoolName
                })
            }
            // Navigation will be handled by AuthContext
        } catch (err: unknown) {
            setError(err instanceof Error ? err.message : 'An error occurred')
        } finally {
            setIsLoading(false)
        }
    }

    const handleGoogleSignIn = async () => {
        setIsGoogleLoading(true)
        setError('')
        
        try {
            // TODO: Implement Google Sign-In
            // For now, we'll simulate it
            console.log('Google Sign-In clicked')
            // Implementation will go here
        } catch (err: unknown) {
            setError(err instanceof Error ? err.message : 'Google Sign-In failed')
        } finally {
            setIsGoogleLoading(false)
        }
    }

    const toggleMode = () => {
        setIsLogin(!isLogin)
        setError('')
        setEmail('')
        setPassword('')
        setDisplayName('')
        setSchoolName('')
    }

    return (
        <div className="min-h-screen flex items-center justify-center p-4">
            <div className="w-full max-w-md">
                {/* Logo and Title */}
                <div className="text-center mb-8">
                    <div className="w-16 h-16 mx-auto mb-4 rounded-xl bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center shadow-lg">
                        <svg className="w-10 h-10 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                        </svg>
                    </div>
                    <h1 className="text-2xl font-bold text-white mb-1">AttendEase</h1>
                    <p className="text-gray-400">Attendance Management System</p>
                </div>

                {/* Auth Form */}
                <div className="glass rounded-2xl p-6 max-h-[80vh] overflow-y-auto">
                    <h2 className="text-xl font-semibold text-white mb-6">
                        {isLogin ? 'Sign In' : 'Create Account'}
                    </h2>

                    <form onSubmit={handleSubmit} className="space-y-4">
                        {!isLogin && (
                            <>
                                <div>
                                    <label className="block text-sm font-medium text-gray-300 mb-2">
                                        Display Name*
                                    </label>
                                    <input
                                        type="text"
                                        value={displayName}
                                        onChange={(e) => setDisplayName(e.target.value)}
                                        className="input w-full"
                                        required={!isLogin}
                                        placeholder="Enter your full name"
                                    />
                                </div>
                                
                                <div>
                                    <label className="block text-sm font-medium text-gray-300 mb-2">
                                        School Name*
                                    </label>
                                    <input
                                        type="text"
                                        value={schoolName}
                                        onChange={(e) => setSchoolName(e.target.value)}
                                        className="input w-full"
                                        required={!isLogin}
                                        placeholder="Your school or institution"
                                    />
                                </div>
                            </>
                        )}

                        <div>
                            <label className="block text-sm font-medium text-gray-300 mb-2">
                                Email Address*
                            </label>
                            <input
                                type="email"
                                value={email}
                                onChange={(e) => setEmail(e.target.value)}
                                className="input w-full"
                                required
                                placeholder="Enter your email address"
                            />
                            {!isLogin && email.includes('@') && (
                                <p className="text-xs text-blue-400 mt-1">
                                    Note: DEPED emails will be automatically detected as government accounts
                                </p>
                            )}
                        </div>

                        <div>
                            <label className="block text-sm font-medium text-gray-300 mb-2">
                                Password*
                            </label>
                            <input
                                type="password"
                                value={password}
                                onChange={(e) => setPassword(e.target.value)}
                                className="input w-full"
                                required
                                placeholder="Enter your password"
                            />
                        </div>

                        {error && (
                            <div className="glass rounded-lg p-3 bg-red-500/10 border border-red-500/30">
                                <p className="text-red-400 text-sm">{error}</p>
                            </div>
                        )}

                        <button
                            type="submit"
                            disabled={isLoading}
                            className="btn btn-primary w-full"
                        >
                            {isLoading ? (
                                <div className="flex items-center justify-center gap-2">
                                    <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"></div>
                                    {isLogin ? 'Signing In...' : 'Creating Account...'}
                                </div>
                            ) : (
                                isLogin ? 'Sign In' : 'Create Account'
                            )}
                        </button>

                    {isLogin && (
                        <>
                            {/* Divider */}
                            <div className="relative my-6">
                                <div className="absolute inset-0 flex items-center">
                                    <div className="w-full border-t border-gray-600"></div>
                                </div>
                                <div className="relative flex justify-center text-sm">
                                    <span className="px-2 bg-gray-950 text-gray-400">Or continue with</span>
                                </div>
                            </div>

                            {/* Google Sign-In Button */}
                            <button
                                type="button"
                                onClick={handleGoogleSignIn}
                                disabled={isLoading || isGoogleLoading}
                                className="w-full flex items-center justify-center gap-3 px-4 py-3 border border-gray-600 rounded-xl text-sm font-medium text-white hover:bg-white/5 transition-colors disabled:opacity-50"
                            >
                                {isGoogleLoading ? (
                                    <div className="flex items-center gap-2">
                                        <div className="w-4 h-4 border-2 border-gray-400 border-t-transparent rounded-full animate-spin"></div>
                                        <span>Signing in with Google...</span>
                                    </div>
                                ) : (
                                    <>
                                        <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none">
                                            <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" fill="#4285F4" />
                                            <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853" />
                                            <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" fill="#FBBC05" />
                                            <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" fill="#EA4335" />
                                        </svg>
                                        <span>Sign in with Google</span>
                                    </>
                                )}
                            </button>
                        </>
                    )}
                    </form>

                    {/* Toggle between login and signup */}
                    <div className="mt-6 text-center">
                        <p className="text-sm text-gray-400">
                            {isLogin ? "Don't have an account?" : "Already have an account?"}
                            <button
                                onClick={toggleMode}
                                className="ml-1 text-blue-400 hover:text-blue-300 font-medium"
                            >
                                {isLogin ? 'Sign Up' : 'Sign In'}
                            </button>
                        </p>
                    </div>
                </div>

                {/* Footer */}
                <div className="mt-8 text-center">
                    <p className="text-xs text-gray-500">
                        By continuing, you agree to our Terms of Service and Privacy Policy
                    </p>
                </div>
            </div>
        </div>
    )
}

export default AuthScreen