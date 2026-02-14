// Infrastructure implementation for UserRepository
#![allow(dead_code)]

use async_trait::async_trait;
use std::path::PathBuf;
use std::fs;
use serde_json;
use chrono::Utc;
use crate::domain::repositories::UserRepository;
use crate::domain::entities::user::{User, UserProfile};

#[derive(Clone)]
pub struct UserRepositoryImpl {
    data_dir: PathBuf,
}

impl UserRepositoryImpl {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn get_users_file_path(&self) -> PathBuf {
        self.data_dir.join("users.json")
    }

    async fn load_users(&self) -> Result<Vec<User>, String> {
        let file_path = self.get_users_file_path();
        
        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read users file: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse users file: {}", e))
    }

    async fn save_users(&self, users: &[User]) -> Result<(), String> {
        let file_path = self.get_users_file_path();
        
        // Ensure directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create users directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(users)
            .map_err(|e| format!("Failed to serialize users: {}", e))?;

        fs::write(file_path, content)
            .map_err(|e| format!("Failed to write users file: {}", e))
    }

    fn get_next_id(&self, users: &[User]) -> i64 {
        users.iter().map(|u| u.id).max().unwrap_or(0) + 1
    }
}

#[async_trait]
impl UserRepository for UserRepositoryImpl {
    async fn create(&self, user: &User) -> Result<i64, String> {
        let mut users = self.load_users().await?;
        let id = self.get_next_id(&users);
        
        let mut new_user = user.clone();
        new_user.id = id;
        new_user.created_at = Utc::now();
        new_user.last_login = Utc::now();
        
        users.push(new_user);
        self.save_users(&users).await?;
        Ok(id)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, String> {
        let users = self.load_users().await?;
        Ok(users.into_iter().find(|u| u.email == email))
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<User>, String> {
        let users = self.load_users().await?;
        Ok(users.into_iter().find(|u| u.id == id))
    }

    async fn update_last_login(&self, id: i64) -> Result<(), String> {
        let mut users = self.load_users().await?;
        
        if let Some(user) = users.iter_mut().find(|u| u.id == id) {
            user.last_login = Utc::now();
            self.save_users(&users).await?;
            Ok(())
        } else {
            Err("User not found".to_string())
        }
    }

    async fn update_profile(&self, id: i64, profile: &UserProfile) -> Result<(), String> {
        let mut users = self.load_users().await?;
        
        if let Some(user) = users.iter_mut().find(|u| u.id == id) {
            user.display_name = profile.display_name.clone();
            user.school_name = profile.school_name.clone();
            user.position = profile.position.clone();
            user.department = profile.department.clone();
            user.employee_id = profile.employee_id.clone();
            
            self.save_users(&users).await?;
            Ok(())
        } else {
            Err("User not found".to_string())
        }
    }

    async fn delete(&self, id: i64) -> Result<(), String> {
        let mut users = self.load_users().await?;
        users.retain(|u| u.id != id);
        self.save_users(&users).await
    }

    async fn get_user_profile(&self, id: i64) -> Result<Option<UserProfile>, String> {
        let users = self.load_users().await?;
        
        if let Some(user) = users.into_iter().find(|u| u.id == id) {
            Ok(Some(UserProfile {
                id: user.id,
                email: user.email,
                display_name: user.display_name,
                school_name: user.school_name,
                position: user.position,
                department: user.department,
                employee_id: user.employee_id,
                organization_type: user.organization_type,
                organization_name: user.organization_name,
                created_at: user.created_at,
                last_login: user.last_login,
            }))
        } else {
            Ok(None)
        }
    }
}