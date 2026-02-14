// Hybrid Synchronization Service
// Coordinates synchronization between local storage, Google Drive, and Firebase

use crate::domain::entities::{Student, Class, Attendance};
use crate::infrastructure::database::student_repository_impl::StudentRepositoryImpl;
use crate::infrastructure::external::firebase::{FirebaseService, SyncResult, ConflictInfo};
use crate::infrastructure::external::google_sync::GoogleDriveService;
use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub enum SyncSource {
    Local,
    Firebase,
    GoogleDrive,
}

#[derive(Debug, Clone)]
pub enum SyncDirection {
    Upload,
    Download,
    Bidirectional,
}

pub struct HybridSyncService {
    firebase: Arc<RwLock<Option<FirebaseService>>>,
    google_drive: Arc<RwLock<Option<GoogleDriveService>>>,
    student_repo: StudentRepositoryImpl,
    sync_interval_seconds: u64,
}

#[derive(Debug)]
pub struct SyncConfig {
    pub firebase_enabled: bool,
    pub google_drive_enabled: bool,
    pub sync_direction: SyncDirection,
    pub sync_interval_seconds: u64,
    pub conflict_resolution: ConflictResolutionStrategy,
}

#[derive(Debug, Clone)]
pub enum ConflictResolutionStrategy {
    LocalWins,
    RemoteWins,
    MostRecent,
    Manual,
}

impl HybridSyncService {
    pub fn new(student_repo: StudentRepositoryImpl) -> Self {
        HybridSyncService {
            firebase: Arc::new(RwLock::new(None)),
            google_drive: Arc::new(RwLock::new(None)),
            student_repo,
            sync_interval_seconds: 1800, // 30 minutes default
        }
    }

    pub async fn configure(&mut self, config: SyncConfig) -> Result<()> {
        self.sync_interval_seconds = config.sync_interval_seconds;
        
        // Initialize Firebase if enabled
        if config.firebase_enabled {
            self.initialize_firebase().await?;
        }
        
        // Initialize Google Drive if enabled
        if config.google_drive_enabled {
            self.initialize_google_drive().await?;
        }
        
        Ok(())
    }

    async fn initialize_firebase(&self) -> Result<()> {
        let project_id = std::env::var("FIREBASE_PROJECT_ID")
            .map_err(|_| anyhow!("FIREBASE_PROJECT_ID not set"))?;
        let key_path = std::env::var("FIREBASE_SERVICE_ACCOUNT_KEY_PATH")
            .map_err(|_| anyhow!("FIREBASE_SERVICE_ACCOUNT_KEY_PATH not set"))?;
        
        let firebase = FirebaseService::new(&project_id, &key_path).await?;
        *self.firebase.write().await = Some(firebase);
        
        Ok(())
    }

    async fn initialize_google_drive(&self) -> Result<()> {
        // Initialize Google Drive service (assuming existing implementation)
        let google_drive = GoogleDriveService::new().await?;
        *self.google_drive.write().await = Some(google_drive);
        
        Ok(())
    }

    // Student synchronization methods
    pub async fn sync_student(&self, student: &Student, source: SyncSource) -> Result<Student> {
        match source {
            SyncSource::Local => {
                // Upload to all remote sources
                self.upload_student_to_remotes(student).await
            }
            SyncSource::Firebase => {
                // Download and merge with local
                self.merge_student_from_firebase(student).await
            }
            SyncSource::GoogleDrive => {
                // Download and merge with local
                self.merge_student_from_google_drive(student).await
            }
        }
    }

    async fn upload_student_to_remotes(&self, student: &Student) -> Result<Student> {
        let mut updated_student = student.clone();
        
        // Upload to Firebase
        if let Some(firebase) = self.firebase.read().await.as_ref() {
            match firebase.upsert_student(student).await {
                Ok(_) => {
                    updated_student.updated_at = chrono::Utc::now().to_rfc3339();
                }
                Err(e) => eprintln!("Failed to upload student to Firebase: {}", e),
            }
        }
        
        // Upload to Google Drive
        if let Some(google_drive) = self.google_drive.read().await.as_ref() {
            match google_drive.sync_student(student).await {
                Ok(_) => {
                    updated_student.updated_at = chrono::Utc::now().to_rfc3339();
                }
                Err(e) => eprintln!("Failed to upload student to Google Drive: {}", e),
            }
        }
        
        // Update local
        self.student_repo.update(&updated_student).await?;
        
        Ok(updated_student)
    }

    async fn merge_student_from_firebase(&self, student: &Student) -> Result<Student> {
        if let Some(firebase) = self.firebase.read().await.as_ref() {
            match firebase.get_student(student.id).await {
                Ok(Some(remote_student)) => {
                    // Check for conflicts
                    let local_updated = chrono::DateTime::parse_from_rfc3339(&student.updated_at)?;
                    let remote_updated = chrono::DateTime::parse_from_rfc3339(&remote_student.updated_at)?;
                    
                    if local_updated != remote_updated {
                        // Conflict detected - resolve using most recent strategy
                        let resolved = if local_updated > remote_updated {
                            student.clone()
                        } else {
                            remote_student
                        };
                        
                        // Update both sides
                        firebase.upsert_student(&resolved).await?;
                        self.student_repo.update(&resolved).await?;
                        
                        return Ok(resolved);
                    }
                }
                Ok(None) => {
                    // Remote doesn't exist, upload local
                    firebase.upsert_student(student).await?;
                }
                Err(e) => eprintln!("Failed to get student from Firebase: {}", e),
            }
        }
        
        Ok(student.clone())
    }

    async fn merge_student_from_google_drive(&self, student: &Student) -> Result<Student> {
        if let Some(google_drive) = self.google_drive.read().await.as_ref() {
            // Similar logic for Google Drive
            match google_drive.get_student(student.id).await {
                Ok(Some(remote_student)) => {
                    // Conflict resolution logic
                    let local_updated = chrono::DateTime::parse_from_rfc3339(&student.updated_at)?;
                    let remote_updated = chrono::DateTime::parse_from_rfc3339(&remote_student.updated_at)?;
                    
                    if local_updated != remote_updated {
                        let resolved = if local_updated > remote_updated {
                            student.clone()
                        } else {
                            remote_student
                        };
                        
                        google_drive.sync_student(&resolved).await?;
                        self.student_repo.update(&resolved).await?;
                        
                        return Ok(resolved);
                    }
                }
                Ok(None) => {
                    google_drive.sync_student(student).await?;
                }
                Err(e) => eprintln!("Failed to get student from Google Drive: {}", e),
            }
        }
        
        Ok(student.clone())
    }

    // Batch synchronization
    pub async fn sync_all_students(&self) -> Result<SyncResult<Student>> {
        let mut sync_result = SyncResult::new();
        
        // Get all local students
        let local_students = self.student_repo.get_all().await?;
        
        for student in &local_students {
            match self.sync_student(student, SyncSource::Local).await {
                Ok(synced_student) => sync_result.synced_items.push(synced_student),
                Err(e) => sync_result.failed_items.push(format!("Student {}: {}", student.id, e)),
            }
        }
        
        // Also download from remotes and merge
        if let Some(firebase) = self.firebase.read().await.as_ref() {
            match self.download_and_merge_from_firebase().await {
                Ok(downloaded) => {
                    sync_result.synced_items.extend(downloaded);
                }
                Err(e) => eprintln!("Failed to download from Firebase: {}", e),
            }
        }
        
        Ok(sync_result)
    }

    async fn download_and_merge_from_firebase(&self) -> Result<Vec<Student>> {
        if let Some(firebase) = self.firebase.read().await.as_ref() {
            // Get all classes first
            let classes = self.student_repo.get_classes().await?;
            let mut merged_students = Vec::new();
            
            for class in classes {
                if let Ok(firebase_students) = firebase.get_students_by_class(class.id).await {
                    for firebase_student in firebase_students {
                        match self.student_repo.get_by_id(firebase_student.id).await {
                            Ok(Some(local_student)) => {
                                // Merge if needed
                                let merged = self.merge_student_data(&local_student, &firebase_student)?;
                                if merged.updated_at != local_student.updated_at {
                                    self.student_repo.update(&merged).await?;
                                    merged_students.push(merged);
                                }
                            }
                            Ok(None) => {
                                // Remote student doesn't exist locally, create it
                                self.student_repo.create(&firebase_student).await?;
                                merged_students.push(firebase_student);
                            }
                            Err(e) => eprintln!("Error checking local student {}: {}", firebase_student.id, e),
                        }
                    }
                }
            }
            
            Ok(merged_students)
        } else {
            Ok(Vec::new())
        }
    }

    fn merge_student_data(&self, local: &Student, remote: &Student) -> Result<Student> {
        let local_updated = chrono::DateTime::parse_from_rfc3339(&local.updated_at)?;
        let remote_updated = chrono::DateTime::parse_from_rfc3339(&remote.updated_at)?;
        
        if remote_updated > local_updated {
            Ok(remote.clone())
        } else {
            Ok(local.clone())
        }
    }

    // Start background sync
    pub async fn start_background_sync(&self) -> Result<()> {
        let sync_interval = self.sync_interval_seconds;
        let student_repo = self.student_repo.clone();
        let firebase = self.firebase.clone();
        let google_drive = self.google_drive.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(sync_interval));
            
            loop {
                interval.tick().await;
                
                // Perform sync
                if let Err(e) = Self::perform_sync_round(
                    &student_repo, 
                    &firebase, 
                    &google_drive
                ).await {
                    eprintln!("Background sync failed: {}", e);
                }
            }
        });
        
        Ok(())
    }

    async fn perform_sync_round(
        student_repo: &StudentRepositoryImpl,
        firebase: &Arc<RwLock<Option<FirebaseService>>>,
        google_drive: &Arc<RwLock<Option<GoogleDriveService>>>,
    ) -> Result<()> {
        // Implementation of one sync round
        // This would contain the actual sync logic
        
        Ok(())
    }
}