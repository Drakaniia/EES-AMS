use crate::domain::services::AuthService;
use crate::domain::entities::user::{UserProfile, LoginRequest, RegisterRequest, AuthResponse};

pub struct AuthHandler {
    auth_service: AuthService,
    current_user: Option<UserProfile>,
}

impl AuthHandler {
    pub fn new(auth_service: AuthService) -> Self {
        Self { 
            auth_service,
            current_user: None,
        }
    }

    pub async fn register(&self, request: RegisterRequest) -> AuthResponse {
        match self.auth_service.register(request).await {
            Ok((user, token)) => AuthResponse {
                success: true,
                user: Some(user),
                token: Some(token),
                message: Some("Registration successful".to_string()),
            },
            Err(error) => AuthResponse {
                success: false,
                user: None,
                token: None,
                message: Some(error),
            },
        }
    }

    pub async fn login(&self, request: LoginRequest) -> AuthResponse {
        match self.auth_service.login(&request.email, &request.password).await {
            Ok((user, token)) => {
                AuthResponse {
                    success: true,
                    user: Some(user),
                    token: Some(token),
                    message: Some("Login successful".to_string()),
                }
            },
            Err(error) => AuthResponse {
                success: false,
                user: None,
                token: None,
                message: Some(error),
            },
        }
    }

    pub async fn validate_token(&mut self, token: &str) -> Option<UserProfile> {
        match self.auth_service.get_user_by_token(token).await {
            Ok(Some(user)) => {
                self.current_user = Some(user.clone());
                Some(user)
            },
            Ok(None) => None,
            Err(_) => None,
        }
    }

    pub async fn update_profile(&self, profile: UserProfile) -> Result<(), String> {
        if let Some(current_user) = &self.current_user {
            self.auth_service.update_profile(current_user.id, profile).await
        } else {
            Err("No authenticated user".to_string())
        }
    }

    pub fn get_current_user(&self) -> Option<UserProfile> {
        self.current_user.clone()
    }

    pub fn logout(&mut self) {
        self.current_user = None;
    }
}