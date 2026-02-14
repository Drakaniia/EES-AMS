import React, { createContext, useContext, useState, useEffect, ReactNode } from "react";
import { authService, UserProfile } from "../lib/auth-tauri";
import { useToast, ToastProvider } from "../components/Toast";

interface AuthContextType {
  userProfile: UserProfile | null;
  loading: boolean;
  signIn: (email: string, password: string) => Promise<boolean>;
  signUp: (data: { email: string; password: string; displayName: string; schoolName: string }) => Promise<boolean>;
  signOut: () => Promise<void>;
  updateUserProfile: (updates: UserProfile) => Promise<boolean>;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

interface AuthProviderProps {
  children: ReactNode;
}

const AuthProviderInner: React.FC<AuthProviderProps> = ({ children }) => {
  const [userProfile, setUserProfile] = useState<UserProfile | null>(null);
  const [loading, setLoading] = useState(true);
  const { showToast } = useToast();

  useEffect(() => {
    const checkAuth = async () => {
      // Check if we're running in Tauri environment
      const isTauriApp = typeof window !== 'undefined' && '__TAURI__' in window;
      
      if (!isTauriApp) {
        console.warn('Not running in Tauri environment - authentication unavailable');
        setLoading(false);
        return;
      }

      try {
        const token = authService.getToken();
        const storedProfile = authService.getStoredProfile();
        
        if (token && storedProfile) {
          // Validate token with backend - don't throw error if validation fails
          const user = await authService.validateToken(token);
          setUserProfile(user);
        } else {
          // Check if there's a current session on backend
          const user = await authService.getCurrentUser();
          if (user) {
            setUserProfile(user);
          }
        }
} catch (error) {
        // Silently handle auth check errors in production
        // Don't show errors for users without accounts yet
        const errorMessage = error instanceof Error ? error.message : 'An error occurred';
        
        // Only log to console, don't show user-facing errors
        if (!errorMessage.includes('invoke')) {
          console.error('Auth check failed:', error);
        }
        console.log('Error variable used for logging');
      } finally {
        setLoading(false);
      }
    };

    checkAuth();
  }, []);

  const signIn = async (email: string, password: string): Promise<boolean> => {
    // Check if running in browser (no Tauri)
    const isTauriApp = typeof window !== 'undefined' && '__TAURI__' in window;
    
    if (!isTauriApp) {
      showToast({
        type: 'error',
        title: 'Backend Required',
        message: 'Please run the desktop app to authenticate with Gmail.'
      });
      return false;
    }
    
    try {
      const response = await authService.login({ email, password });
      
      if (!response.success) {
        showToast({
          type: 'error',
          title: 'Login Failed',
          message: response.message || 'Invalid email or password'
        });
        return false;
      }
      
      if (response.user) {
        setUserProfile(response.user);
        showToast({
          type: 'success',
          title: 'Welcome Back!',
          message: `Logged in as ${response.user.display_name || response.user.email}`
        });
      }
      return true;
    } catch {
      showToast({
        type: 'error',
        title: 'Login Error',
        message: 'An unexpected error occurred. Please try again.'
      });
      return false;
    }
  };

  const signUp = async (data: { 
    email: string; 
    password: string; 
    displayName: string; 
    schoolName: string;
  }): Promise<boolean> => {
    // Check if running in browser (no Tauri)
    const isTauriApp = typeof window !== 'undefined' && '__TAURI__' in window;
    
    if (!isTauriApp) {
      showToast({
        type: 'error',
        title: 'Backend Required',
        message: 'Please run the desktop app to create an account.'
      });
      return false;
    }
    
    try {
      const response = await authService.register({
        email: data.email,
        password: data.password,
        display_name: data.displayName,
        school_name: data.schoolName
      });
      
      if (!response.success) {
        showToast({
          type: 'error',
          title: 'Registration Failed',
          message: response.message || 'Unable to create account'
        });
        return false;
      }
      
      if (response.user) {
        setUserProfile(response.user);
        showToast({
          type: 'success',
          title: 'Account Created!',
          message: `Welcome to AttendEase, ${response.user.display_name}!`
        });
      }
      return true;
    } catch {
      showToast({
        type: 'error',
        title: 'Registration Error',
        message: 'An unexpected error occurred. Please try again.'
      });
      return false;
    }
  };

  const signOut = async () => {
    try {
      await authService.logout();
      setUserProfile(null);
      showToast({
        type: 'info',
        title: 'Signed Out',
        message: 'You have been successfully signed out.'
      });
    } catch {
      showToast({
        type: 'warning',
        title: 'Sign Out Warning',
        message: 'You may still have an active session on this device.'
      });
    }
  };

  const updateUserProfile = async (updates: UserProfile): Promise<boolean> => {
    try {
      const success = await authService.updateUserProfile(updates);
      
      if (!success) {
        showToast({
          type: 'error',
          title: 'Update Failed',
          message: 'Unable to update profile. Please try again.'
        });
        return false;
      }
      
      setUserProfile(updates);
      showToast({
        type: 'success',
          title: 'Profile Updated',
          message: 'Your profile has been successfully updated.'
});
      return true;
    } catch {
      showToast({
        type: 'error',
        title: 'Registration Error',
        message: 'An unexpected error occurred. Please try again.'
      });
      return false;
    }
  };

  const value: AuthContextType = {
    userProfile,
    loading,
    signIn,
    signUp,
    signOut,
    updateUserProfile
  };

  return (
    <AuthContext.Provider value={value}>
      {children}
    </AuthContext.Provider>
  );
};

export const AuthProvider: React.FC<AuthProviderProps> = ({ children }) => (
  <ToastProvider>
    <AuthProviderInner>{children}</AuthProviderInner>
  </ToastProvider>
);

// eslint-disable-next-line react-refresh/only-export-components
export const useAuth = (): AuthContextType => {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
};