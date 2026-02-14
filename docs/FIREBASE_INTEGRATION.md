# Firebase Integration Guide for EES-AMS

This guide explains how to set up and integrate Firebase Firestore with the EES-AMS system for hybrid storage capabilities.

## Overview

EES-AMS now supports hybrid storage across multiple platforms:
- **Local Storage** - JSON files for offline functionality
- **Google Drive** - File storage and backup
- **Firebase Firestore** - Real-time data synchronization and collaboration

## Firebase Setup

### 1. Create Firebase Project

1. Go to [Firebase Console](https://console.firebase.google.com/)
2. Click "Add project"
3. Enter project details (e.g., "ees-ams-production")
4. Enable Google Analytics (optional)
5. Click "Create project"

### 2. Enable Firestore Database

1. In your project dashboard, go to "Build" → "Firestore Database"
2. Click "Create database"
3. Choose "Start in test mode" for initial setup
4. Select a location (choose nearest to your users)
5. Click "Enable"

### 3. Get Service Account Key

1. Go to Project Settings (⚙️ icon) → "Service accounts"
2. Click "Generate new private key"
3. Select JSON format
4. Click "Generate"
5. Save the downloaded JSON file as `firebase-service-account.json` in your project root
6. **IMPORTANT**: Never commit this file to version control

### 4. Configure Firebase Rules

In Firestore Database → Rules, replace default rules with:

```javascript
rules_version = '2';
service cloud.firestore {
  match /databases/{database}/documents {
    // Only authenticated users can read/write their own data
    match /users/{userId}/{document=**} {
      allow read, write: if request.auth.uid == userId;
    }
    
    // Public readable data with restricted writes
    match /classes/{classId} {
      allow read: if true;
      allow write: if request.auth != null;
    }
    
    // Student data requires authentication
    match /students/{studentId} {
      allow read, write: if request.auth != null;
    }
    
    // Attendance data requires authentication
    match /attendance/{attendanceId} {
      allow read, write: if request.auth != null;
    }
    
    // Sync metadata for conflict resolution
    match /sync_metadata/{docId} {
      allow read, write: if request.auth != null;
    }
  }
}
```

## Configuration

### Environment Variables

Copy `.env.example` to `.env` and update with your Firebase details:

```bash
# Firebase Configuration
FIREBASE_PROJECT_ID=your-firebase-project-id
FIREBASE_SERVICE_ACCOUNT_KEY_PATH=./firebase-service-account.json
FIREBASE_API_KEY=your-firebase-web-api-key
FIREBASE_AUTH_DOMAIN=your-project.firebaseapp.com
FIREBASE_DATABASE_URL=https://your-project.firebaseio.com
```

### Required Values

- `FIREBASE_PROJECT_ID`: Your Firebase project ID from Firebase Console
- `FIREBASE_SERVICE_ACCOUNT_KEY_PATH`: Path to your service account JSON file
- `FIREBASE_API_KEY`: Web API key from Firebase Console → Project Settings → General
- `FIREBASE_AUTH_DOMAIN`: Your project's Firebase Auth domain
- `FIREBASE_DATABASE_URL`: Your Firebase database URL

## Usage

### Basic Operations

```rust
// Initialize Firebase service
let firebase = FirebaseService::new("your-project-id", "path/to/service-account.json").await?;

// Sync student to Firebase
let synced_student = firebase.sync_student(&student).await?;

// Get student from Firebase
let student = firebase.get_student(student_id).await?;

// Get students by class
let students = firebase.get_students_by_class(class_id).await?;

// Batch sync multiple students
let results = firebase.batch_sync_students(&students).await?;
```

### Hybrid Synchronization

```rust
// Configure hybrid sync
let sync_config = SyncConfig {
    firebase_enabled: true,
    google_drive_enabled: true,
    sync_direction: SyncDirection::Bidirectional,
    sync_interval_seconds: 1800,
    conflict_resolution: ConflictResolutionStrategy::MostRecent,
};

let mut hybrid_sync = HybridSyncService::new(student_repo);
hybrid_sync.configure(sync_config).await?;

// Sync all students
let sync_result = hybrid_sync.sync_all_students().await?;

// Start background sync
hybrid_sync.start_background_sync().await?;
```

## Data Models

### Student Document

```json
{
  "id": 12345,
  "student_id": "STD000123",
  "lrn": "2021001234",
  "last_name": "Smith",
  "first_name": "John",
  "middle_name": "Doe",
  "gender": "Male",
  "birthday": "2015-05-15",
  "age": 8,
  "mother_name": "Jane Smith",
  "father_name": "Robert Smith",
  "guardian_name": "Jane Smith",
  "address": "123 Main St, City",
  "class_id": 1,
  "created_at": "2025-02-14T10:00:00Z",
  "updated_at": "2025-02-14T10:00:00Z"
}
```

### Class Document

```json
{
  "id": 1,
  "name": "Grade 3 - MATAPAT",
  "section": "1",
  "school_year": "2024-2025",
  "created_at": "2025-02-14T10:00:00Z",
  "updated_at": "2025-02-14T10:00:00Z"
}
```

### Attendance Document

```json
{
  "id": 23456,
  "student_id": 12345,
  "date": "2025-02-14",
  "status": "Present",
  "remarks": null,
  "created_at": "2025-02-14T08:00:00Z",
  "updated_at": "2025-02-14T08:00:00Z"
}
```

## Conflict Resolution

The hybrid sync system handles conflicts using multiple strategies:

### 1. Most Recent (Default)
Uses the document with the most recent `updated_at` timestamp.

### 2. Local Wins
Always prefers the local version over remote changes.

### 3. Remote Wins
Always prefers the remote version over local changes.

### 4. Manual
Preserves both versions and flags for manual review.

## Security Considerations

1. **Never expose service account keys** in client-side code
2. **Use environment variables** for sensitive configuration
3. **Implement proper authentication** before allowing data access
4. **Set appropriate Firebase Security Rules** for your use case
5. **Regularly rotate API keys** and service account credentials
6. **Monitor Firestore usage** and costs in Firebase Console

## Performance Optimization

1. **Batch operations** when syncing multiple documents
2. **Use queries** instead of full collection scans
3. **Implement pagination** for large datasets
4. **Cache frequently accessed data** locally
5. **Monitor read/write operations** to avoid costly queries

## Troubleshooting

### Common Issues

1. **Authentication errors**
   - Check service account key path and permissions
   - Verify project ID matches Firebase Console

2. **Permission denied**
   - Review Firestore Security Rules
   - Ensure proper authentication flow

3. **Sync conflicts**
   - Check `updated_at` timestamps
   - Verify conflict resolution strategy

4. **Performance issues**
   - Add Firestore indexes for complex queries
   - Reduce document size by normalizing data

### Debug Logging

Enable debug logging:

```rust
env_logger::init();
```

## Best Practices

1. **Implement offline support** with local caching
2. **Use optimistic UI updates** for better user experience
3. **Regular backups** of important data
4. **Monitor sync status** and handle failures gracefully
5. **Test conflict resolution** scenarios thoroughly
6. **Implement retry logic** for failed sync operations

## Deployment

### Production Checklist

- [ ] Update Firebase Security Rules for production
- [ ] Enable billing for your Firebase project
- [ ] Set up monitoring and alerts
- [ ] Configure data retention policies
- [ ] Test disaster recovery procedures
- [ ] Document backup and restore processes