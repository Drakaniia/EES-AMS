import React, { createContext, useContext, useState, useEffect, ReactNode } from "react";
import { authService, UserProfile } from "../lib/auth-tauri";

interface AuthContextType {
  userProfile: UserProfile | null;
  loading: boolean;
  signIn: (email: string, password: string) => Promise<void>;
  signUp: (data: { email: string; password: string; displayName: string; schoolName: string }) => Promise<void>;
  signOut: () => Promise<void>;
  updateUserProfile: (updates: UserProfile) => Promise<void>;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

interface AuthProviderProps {
  children: ReactNode;
}

export const AuthProvider: React.FC<AuthProviderProps> = ({ children }) => {
  const [userProfile, setUserProfile] = useState<UserProfile | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const checkAuth = async () => {
      const token = authService.getToken();
      const storedProfile = authService.getStoredProfile();
      
      if (token && storedProfile) {
        // Validate token with backend
        const user = await authService.validateToken(token);
        setUserProfile(user);
      } else {
        // Check if there's a current session on backend
        const user = await authService.getCurrentUser();
        setUserProfile(user);
      }
      
      setLoading(false);
    };

    checkAuth();
  }, []);

  const signIn = async (email: string, password: string) => {
    const response = await authService.login({ email, password });
    
    if (!response.success) {
      throw new Error(response.message || 'Login failed');
    }
    
    if (response.user) {
      setUserProfile(response.user);
    }
  };

  const signUp = async (data: { 
    email: string; 
    password: string; 
    displayName: string; 
    schoolName: string;
  }) => {
    const response = await authService.register({
      email: data.email,
      password: data.password,
      display_name: data.displayName,
      school_name: data.schoolName
    });
    
    if (!response.success) {
      throw new Error(response.message || 'Registration failed');
    }
    
    if (response.user) {
      setUserProfile(response.user);
    }
  };

  const signOut = async () => {
    await authService.logout();
    setUserProfile(null);
  };

  const updateUserProfile = async (updates: UserProfile) => {
    const success = await authService.updateUserProfile(updates);
    
    if (!success) {
      throw new Error('Failed to update profile');
    }
    
    setUserProfile(updates);
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

export const useAuth = (): AuthContextType => {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
};