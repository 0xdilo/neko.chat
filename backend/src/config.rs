use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub encryption_key: String,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub google_redirect_uri: String,
    pub disable_admin_account: bool,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        if database_url.starts_with("sqlite:") {
            let db_path = database_url.strip_prefix("sqlite:").unwrap();
            if let Some(parent) = std::path::Path::new(db_path).parent() {
                std::fs::create_dir_all(parent).ok();
            }
        }

        Self {
            database_url,
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            encryption_key: env::var("ENCRYPTION_KEY")
                .expect("ENCRYPTION_KEY must be set (32 chars)"),
            google_client_id: env::var("GOOGLE_CLIENT_ID").ok(),
            google_client_secret: env::var("GOOGLE_CLIENT_SECRET").ok(),
            google_redirect_uri: env::var("GOOGLE_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:5173/auth/callback".to_string()),
            disable_admin_account: env::var("DISABLE_ADMIN_ACCOUNT")
                .map(|v| v.to_lowercase() == "true" || v == "1")
                .unwrap_or(false),
        }
    }
}
