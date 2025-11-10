use axum::extract::FromRequestParts;
use axum::http::{header, request::Parts, StatusCode};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // QQ号
    pub nickname: String,
    pub exp: usize, // 过期时间
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub qq: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub qq: String,
    pub nickname: String,
    pub birthday: Option<String>,
    pub role_name: Option<String>,
    pub permissions: Vec<String>,
    pub is_default_password: bool, // 是否使用默认密码
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub qq: String,
    pub nickname: String,
    pub password: String,
    pub birthday: Option<String>,
}

pub const JWT_SECRET: &[u8] = b"team-operation-system-secret-key-change-in-production";

#[derive(Debug, Clone)]
pub struct AuthenticatedUser(pub Claims);

impl AuthenticatedUser {
    pub fn qq(&self) -> &str {
        &self.0.sub
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.0.permissions.iter().any(|p| p == permission)
    }

    pub fn require_permission(&self, permission: &str) -> Result<(), StatusCode> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let header_value = parts
            .headers
            .get(header::AUTHORIZATION)
            .ok_or(StatusCode::UNAUTHORIZED)?
            .to_str()
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        let token = header_value
            .strip_prefix("Bearer ")
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(JWT_SECRET),
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

        Ok(AuthenticatedUser(token_data.claims))
    }
}
