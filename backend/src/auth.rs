use crate::error::AppError;
use crate::AppState;
use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

pub fn create_jwt(user_id: String, secret: &str) -> Result<String, AppError> {
    let iat = Utc::now();
    let exp = iat + Duration::days(7);

    let claims = Claims {
        sub: user_id,
        exp: exp.timestamp() as usize,
        iat: iat.timestamp() as usize,
    };

    let header = Header::default();
    let key = EncodingKey::from_secret(secret.as_ref());
    encode(&header, &claims, &key).map_err(AppError::from)
}

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
                .await
                .map_err(|_| AppError::Unauthorized)?;

        let token_data = decode::<Claims>(
            bearer.token(),
            &DecodingKey::from_secret(app_state.config.jwt_secret.as_ref()),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized)?;

        Ok(token_data.claims)
    }
}
