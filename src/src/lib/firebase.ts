import { initializeApp } from "firebase/app";
import { getAuth } from "firebase/auth";
import { getFirestore, doc, setDoc, getDoc } from "firebase/firestore";
import { getStorage } from "firebase/storage";
import { getAnalytics } from "firebase/analytics";

// Firebase configuration
const firebaseConfig = {
  apiKey: import.meta.env.VITE_FIREBASE_API_KEY,
  authDomain: import.meta.env.VITE_FIREBASE_AUTH_DOMAIN,
  projectId: import.meta.env.VITE_FIREBASE_PROJECT_ID,
  storageBucket: import.meta.env.VITE_FIREBASE_STORAGE_BUCKET,
  messagingSenderId: import.meta.env.VITE_FIREBASE_MESSAGING_SENDER_ID,
  appId: import.meta.env.VITE_FIREBASE_APP_ID,
  measurementId: import.meta.env.VITE_FIREBASE_MEASUREMENT_ID
};

// Initialize Firebase
const app = initializeApp(firebaseConfig);

// Initialize Firebase services
export const auth = getAuth(app);
export const db = getFirestore(app);
export const storage = getStorage(app);
export const analytics = getAnalytics(app);

// Export configuration for backend integration
export { firebaseConfig };

// Helper functions for backend integration
export const getFirebaseConfig = () => firebaseConfig;

export const initializeClientFirebase = () => {
  if (!app) {
    throw new Error('Firebase app is not initialized');
  }
  return app;
};

// Hybrid storage utility - uses Firebase as secondary storage
export const hybridStorage = {
  // Save to Firebase (backup)
  async saveToFirebase(path: string, data: unknown) {
    try {
      const docRef = doc(db, path);
      await setDoc(docRef, data);
      return { success: true };
    } catch (error) {
      console.error('Failed to save to Firebase:', error);
      return { success: false, error };
    }
  },

  // Load from Firebase (backup)
  async loadFromFirebase(path: string) {
    try {
      const docRef = doc(db, path);
      const docSnap = await getDoc(docRef);
      if (docSnap.exists()) {
        return { success: true, data: docSnap.data() };
      } else {
        return { success: false, error: 'Document not found' };
      }
    } catch (error) {
      console.error('Failed to load from Firebase:', error);
      return { success: false, error };
    }
  },

  // Sync to Google Sheets via backend
  async syncToGoogleSheets(data: unknown) {
    try {
      // This will be handled by the Tauri backend
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke('google_sync_data', { data });
      return { success: true, result };
    } catch (error) {
      console.error('Failed to sync to Google Sheets:', error);
      return { success: false, error };
    }
  }
};

export default app;