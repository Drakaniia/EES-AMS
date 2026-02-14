use crate::domain::repositories::UserRepository;
use crate::domain::entities::user::{User, UserProfile, RegisterRequest};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, rand_core::OsRng};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use chrono::{Duration, Utc};
use std::collections::HashMap;

pub struct AuthService {
    user_repo: Box<dyn UserRepository>,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(user_repo: Box<dyn UserRepository>, jwt_secret: String) -> Self {
        Self { user_repo, jwt_secret }
    }

    // Organization detection from email domain
    fn detect_organization(&self, email: &str) -> (String, String) {
        if let Some(domain) = email.split('@').nth(1) {
            match domain.to_lowercase().as_str() {
                domain if domain.ends_with("deped.gov.ph") => (
                    "government".to_string(),
                    "Department of Education".to_string()
                ),
                domain if domain.ends_with("ched.gov.ph") => (
                    "government".to_string(),
                    "Commission on Higher Education".to_string()
                ),
                domain if domain.ends_with("dost.gov.ph") => (
                    "government".to_string(),
                    "Department of Science and Technology".to_string()
                ),
                domain if domain.ends_with("edu.ph") || domain.ends_with(".edu") => (
                    "educational".to_string(),
                    domain.split('.').next().unwrap_or("Educational Institution")
                        .to_uppercase()
                ),
                domain if domain.ends_with("gov.ph") => (
                    "government".to_string(),
                    domain.split('.').next().unwrap_or("Government Agency")
                        .to_uppercase()
                ),
                _ => ("other".to_string(), "Organization".to_string()),
            }
        } else {
            ("other".to_string(), "Unknown".to_string())
        }
    }

    // Password hashing
    fn hash_password(&self, password: &str) -> Result<String, String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Failed to hash password: {}", e))?;
        
        Ok(password_hash.to_string())
    }

    // Password verification
    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, String> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| format!("Failed to parse password hash: {}", e))?;
        
        let argon2 = Argon2::default();
        Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }

    // JWT token generation
    fn generate_token(&self, user_id: i64) -> Result<String, String> {
        let mut claims = HashMap::new();
        claims.insert("user_id", user_id);
        claims.insert("exp", (Utc::now() + Duration::hours(24)).timestamp());
        
        encode(&Header::default(), &claims, &EncodingKey::from_secret(self.jwt_secret.as_ref()))
            .map_err(|e| format!("Failed to generate token: {}", e))
    }

    // User registration
    pub async fn register(&self, request: RegisterRequest) -> Result<(UserProfile, String), String> {
        // Check if user already exists
        if let Some(_) = self.user_repo.find_by_email(&request.email).await? {
            return Err("User with this email already exists".to_string());
        }

        // Hash password
        let password_hash = self.hash_password(&request.password)?;

        // Detect organization
        let (organization_type, organization_name) = self.detect_organization(&request.email);

        // Create user
        let user = User {
            id: 0, // Will be set by database
            email: request.email,
            password_hash,
            display_name: request.display_name,
            school_name: request.school_name,
            position: String::new(), // Empty string
            department: String::new(), // Empty string
            employee_id: String::new(), // Empty string
            organization_type,
            organization_name,
            created_at: chrono::Utc::now(),
            last_login: chrono::Utc::now(),
            is_active: true,
        };

        // Save user
        let user_id = self.user_repo.create(&user).await?;

        // Generate token
        let token = self.generate_token(user_id)?;

        // Get user profile
        let profile = self.user_repo.get_user_profile(user_id).await?
            .ok_or("Failed to retrieve user profile")?;

        Ok((profile, token))
    }

    // User login
    pub async fn login(&self, email: &str, password: &str) -> Result<(UserProfile, String), String> {
        // Find user
        let user = self.user_repo.find_by_email(email).await?
            .ok_or("Invalid email or password")?;

        // Verify password
        if !self.verify_password(password, &user.password_hash)? {
            return Err("Invalid email or password".to_string());
        }

        // Update last login
        self.user_repo.update_last_login(user.id).await?;

        // Generate token
        let token = self.generate_token(user.id)?;

        // Get user profile
        let profile = self.user_repo.get_user_profile(user.id).await?
            .ok_or("Failed to retrieve user profile")?;

        Ok((profile, token))
    }

    // Validate JWT token
    pub fn validate_token(&self, token: &str) -> Result<i64, String> {
        let token_data = decode::< HashMap<String, serde_json::Value>>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &Validation::default()
        ).map_err(|_| "Invalid token".to_string())?;

        let user_id = token_data.claims.get("user_id")
            .and_then(|v| v.as_i64())
            .ok_or("Invalid token claims".to_string())?;

        Ok(user_id)
    }

    // Get user by token
    pub async fn get_user_by_token(&self, token: &str) -> Result<Option<UserProfile>, String> {
        let user_id = self.validate_token(token)?;
        self.user_repo.get_user_profile(user_id).await
    }

    // Update user profile
    pub async fn update_profile(&self, user_id: i64, profile: UserProfile) -> Result<(), String> {
        self.user_repo.update_profile(user_id, &profile).await
    }
}