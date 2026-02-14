import { auth } from "./firebase";
import { 
  signInWithEmailAndPassword,
  createUserWithEmailAndPassword,
  signOut,
  onAuthStateChanged,
  User as FirebaseUser,
  updateProfile
} from "firebase/auth";
import { doc, setDoc, getDoc, updateDoc, serverTimestamp } from "firebase/firestore";
import { db } from "./firebase";
import { UserProfile, createProfileFromUser, detectOrganization } from "./organization";

export const authService = {
  async signIn(email: string, password: string): Promise<UserProfile> {
    try {
      const userCredential = await signInWithEmailAndPassword(auth, email, password);
      const user = userCredential.user;
      
      // Update last login
      await this.updateUserProfile(user.uid, { lastLogin: new Date() });
      
      return await this.getUserProfile(user);
    } catch (error) {
      throw new Error(`Sign in failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  },

  async signUp(email: string, password: string, displayName: string): Promise<UserProfile> {
    try {
      const userCredential = await createUserWithEmailAndPassword(auth, email, password);
      const user = userCredential.user;
      
      // Update display name
      await updateProfile(user, { displayName });
      
      // Create user profile
      const profile = createProfileFromUser(user);
      profile.displayName = displayName;
      
      await this.createUserProfile(user.uid, profile);
      
      return profile;
    } catch (error) {
      throw new Error(`Sign up failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  },

  async signOut(): Promise<void> {
    try {
      await signOut(auth);
    } catch (error) {
      throw new Error(`Sign out failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  },

  async createUserProfile(uid: string, profile: UserProfile): Promise<void> {
    try {
      const profileRef = doc(db, "users", uid);
      const profileData = {
        ...profile,
        createdAt: serverTimestamp(),
        lastLogin: serverTimestamp()
      };
      await setDoc(profileRef, profileData);
    } catch (error) {
      throw new Error(`Failed to create user profile: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  },

  async getUserProfile(user: FirebaseUser): Promise<UserProfile> {
    try {
      const profileRef = doc(db, "users", user.uid);
      const profileDoc = await getDoc(profileRef);
      
      if (profileDoc.exists()) {
        return {
          ...profileDoc.data() as UserProfile,
          uid: user.uid,
          email: user.email || "",
          displayName: user.displayName || profileDoc.data().displayName || "",
          photoURL: user.photoURL || profileDoc.data().photoURL || ""
        };
      } else {
        // Create profile if it doesn't exist
        const profile = createProfileFromUser(user);
        await this.createUserProfile(user.uid, profile);
        return profile;
      }
    } catch (error) {
      throw new Error(`Failed to get user profile: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  },

  async updateUserProfile(uid: string, updates: Partial<UserProfile>): Promise<void> {
    try {
      const profileRef = doc(db, "users", uid);
      await updateDoc(profileRef, {
        ...updates,
        lastLogin: serverTimestamp()
      });
    } catch (error) {
      throw new Error(`Failed to update user profile: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  },

  onAuthStateChanged(callback: (user: FirebaseUser | null) => void) {
    return onAuthStateChanged(auth, callback);
  },

  getCurrentUser(): FirebaseUser | null {
    return auth.currentUser;
  }
};