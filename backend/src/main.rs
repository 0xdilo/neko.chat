mod auth;
mod config;
mod database;
mod error;
mod handlers;
mod llm;
mod routes;
mod validation;
mod streaming_manager;
mod ws_messages;

use axum::extract::FromRef;
use axum::http::{HeaderValue, Method};
use bcrypt;
use config::Config;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use streaming_manager::StreamingManager;
use ws_messages::WsMessage;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub config: Config,
    pub tx: broadcast::Sender<WsMessage>,
    pub streaming_manager: Option<StreamingManager>,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(app_state: &AppState) -> PgPool {
        app_state.db_pool.clone()
    }
}

impl FromRef<AppState> for Config {
    fn from_ref(app_state: &AppState) -> Config {
        app_state.config.clone()
    }
}

impl FromRef<AppState> for broadcast::Sender<WsMessage> {
    fn from_ref(app_state: &AppState) -> broadcast::Sender<WsMessage> {
        app_state.tx.clone()
    }
}

impl FromRef<AppState> for Option<StreamingManager> {
    fn from_ref(app_state: &AppState) -> Option<StreamingManager> {
        app_state.streaming_manager.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    print_neko();

    let config = Config::from_env();

    let db_pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to connect to database: {}", e);
            e
        })?;

    tracing::info!("database connection established");

    let schema_statements = vec![
        r#"CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            password_hash TEXT,
            google_id TEXT UNIQUE,
            avatar_url TEXT,
            role TEXT NOT NULL DEFAULT 'user',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS chats (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            title TEXT NOT NULL,
            system_prompt TEXT,
            provider TEXT NOT NULL DEFAULT 'openai',
            model TEXT NOT NULL DEFAULT 'gpt-4o',
            pinned BOOLEAN NOT NULL DEFAULT false,
            is_branch BOOLEAN NOT NULL DEFAULT false,
            parent_chat_id TEXT,
            branch_point_message_id TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )"#,
        r#"CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            chat_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE
        )"#,
        r#"CREATE TABLE IF NOT EXISTS user_api_keys (
            user_id TEXT NOT NULL,
            provider TEXT NOT NULL,
            encrypted_key TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (user_id, provider),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )"#,
        r#"CREATE TABLE IF NOT EXISTS system_prompts (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            name TEXT NOT NULL,
            prompt TEXT NOT NULL,
            description TEXT,
            is_default BOOLEAN NOT NULL DEFAULT false,
            category TEXT NOT NULL DEFAULT 'general',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )"#,
        r#"CREATE TABLE IF NOT EXISTS user_settings (
            user_id TEXT PRIMARY KEY,
            theme TEXT NOT NULL DEFAULT 'dark',
            language TEXT NOT NULL DEFAULT 'en',
            font_size INTEGER NOT NULL DEFAULT 14,
            notifications_enabled BOOLEAN NOT NULL DEFAULT true,
            auto_save BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )"#,
        r#"CREATE TABLE IF NOT EXISTS user_models (
            user_id TEXT NOT NULL,
            provider TEXT NOT NULL,
            model_id TEXT NOT NULL,
            model_name TEXT NOT NULL,
            is_enabled BOOLEAN NOT NULL DEFAULT true,
            display_order INTEGER NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (user_id, provider, model_id),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )"#,
        r#"CREATE TABLE IF NOT EXISTS streaming_states (
            id TEXT PRIMARY KEY,
            message_id TEXT NOT NULL,
            chat_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'streaming',
            content TEXT NOT NULL DEFAULT '',
            provider TEXT NOT NULL,
            model TEXT NOT NULL,
            total_tokens INTEGER,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            completed_at TIMESTAMPTZ,
            error_message TEXT,
            last_chunk_index INTEGER DEFAULT -1,
            chunks JSONB DEFAULT '[]'::jsonb,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE,
            FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
        )"#,
    ];

    for statement in schema_statements {
        if let Err(e) = sqlx::query(statement).execute(&db_pool).await {
            if !e.to_string().contains("already exists") {
                tracing::error!("schema statement failed: {}", e);
                tracing::error!("statement was: {}", statement);
            }
        }
    }

    if config.disable_admin_account {
        tracing::info!(
            "DISABLE_ADMIN_ACCOUNT flag is set. attempting to remove default admin account..."
        );
        let admin_delete_result = sqlx::query("DELETE FROM users WHERE email = 'admin@admin.com'")
            .execute(&db_pool)
            .await;

        match admin_delete_result {
            Ok(result) if result.rows_affected() > 0 => {
                tracing::info!(
                    "default admin account ('admin@admin.com') was successfully removed."
                );
            }
            Ok(_) => {
                tracing::info!("default admin account was not found, no action taken.");
            }
            Err(e) => {
                tracing::error!(
                    "an error occurred while trying to remove the default admin account: {}",
                    e
                );
            }
        }
    } else {
        tracing::info!("ensuring default admin account ('admin@admin.com') is enabled...");
        let admin_password_hash = bcrypt::hash("admin", bcrypt::DEFAULT_COST)
            .map_err(|e| {
                tracing::error!("Failed to hash admin password: {}", e);
                e
            })?;

        let admin_insert_result = sqlx::query(
            r#"INSERT INTO users (id, email, name, password_hash, role)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (email) DO NOTHING"#,
        )
        .bind("admin-default-id")
        .bind("admin@admin.com")
        .bind("Admin")
        .bind(&admin_password_hash)
        .bind("admin")
        .execute(&db_pool)
        .await;

        match admin_insert_result {
            Ok(result) if result.rows_affected() > 0 => {
                tracing::info!(
                    "new admin user created. email: 'admin@admin.com', password: 'admin'"
                );
            }
            Ok(_) => {
                tracing::info!("admin user already exists. to reset password, delete the user from the db and restart the server.");
            }
            Err(e) => {
                tracing::error!("failed to create or verify admin user: {}", e);
            }
        }
    }

    tracing::info!("database schema initialized");

    // Add performance indexes
    let index_statements = vec![
        "CREATE INDEX IF NOT EXISTS idx_system_prompts_user_default ON system_prompts(user_id, is_default)",
        "CREATE INDEX IF NOT EXISTS idx_messages_chat_created ON messages(chat_id, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_chats_user_created ON chats(user_id, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_user_models_user_enabled ON user_models(user_id, is_enabled)",
        "CREATE INDEX IF NOT EXISTS idx_streaming_states_user_status ON streaming_states(user_id, status)",
        "CREATE INDEX IF NOT EXISTS idx_streaming_states_message_id ON streaming_states(message_id)",
        "CREATE INDEX IF NOT EXISTS idx_streaming_states_created_at ON streaming_states(created_at)",
        "CREATE INDEX IF NOT EXISTS idx_streaming_states_active ON streaming_states(user_id, status, created_at DESC) WHERE status = 'streaming'",
    ];

    for statement in index_statements {
        if let Err(e) = sqlx::query(statement).execute(&db_pool).await {
            tracing::warn!("Index creation failed (may already exist): {}", e);
        }
    }

    tracing::info!("database indexes created");

    let migration_result =
        sqlx::query("ALTER TABLE chats ADD COLUMN IF NOT EXISTS is_branch BOOLEAN DEFAULT false")
            .execute(&db_pool)
            .await;

    match migration_result {
        Ok(_) => tracing::info!("added is_branch column to chats table"),
        Err(e) => {
            tracing::error!("failed to add is_branch column: {}", e);
        }
    }

    let migration_result = sqlx::query("ALTER TABLE chats ADD COLUMN IF NOT EXISTS parent_chat_id TEXT")
        .execute(&db_pool)
        .await;

    match migration_result {
        Ok(_) => tracing::info!("added parent_chat_id column to chats table"),
        Err(e) => {
            tracing::error!("failed to add parent_chat_id column: {}", e);
        }
    }

    let migration_result = sqlx::query("ALTER TABLE chats ADD COLUMN IF NOT EXISTS branch_point_message_id TEXT")
        .execute(&db_pool)
        .await;

    match migration_result {
        Ok(_) => {
            tracing::info!("added branch_point_message_id column to chats table")
        }
        Err(e) => {
            tracing::error!("failed to add branch_point_message_id column: {}", e);
        }
    }

    tracing::info!("all migrations completed");

    // Create broadcast channel for WebSocket messages with higher capacity
    let (tx, _rx) = broadcast::channel::<WsMessage>(1000); // Increased from 100

    // Create streaming manager
    let streaming_manager = StreamingManager::new(db_pool.clone(), tx.clone());

    let app_state = AppState {
        db_pool,
        config: config.clone(),
        tx,
        streaming_manager: Some(streaming_manager),
    };

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>()
            .map_err(|e| {
                tracing::error!("Failed to parse CORS origin: {}", e);
                e
            })?)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers(vec![
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ]);
    let app = routes::create_router(app_state).layer(cors);

    let addr_str = std::env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let addr: SocketAddr = addr_str.parse()
        .map_err(|e| {
            tracing::error!("Invalid server address format '{}': {}", addr_str, e);
            e
        })?;
    tracing::info!("server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| {
            tracing::error!("Failed to bind to address {}: {}", addr, e);
            e
        })?;
    axum::serve(listener, app).await
        .map_err(|e| {
            tracing::error!("Server error: {}", e);
            e
        })?;
    Ok(())
}

fn print_neko() {
    let ascii_art = "
        ████                      ████        
      ██    ██                  ██    ██      
      ██    ██                  ██    ██      
    ██        ██████████████████        ██    
    ██        ▓▓▓▓  ▓▓▓▓▓▓  ▓▓▓▓        ██    
    ██        ▓▓▓▓  ▓▓▓▓▓▓  ▓▓▓▓        ██    
  ██                                      ██  
  ██  ██    ████              ████    ██  ██  
  ██    ██  ████      ██      ████  ██    ██  
██    ██            ██████            ██    ██
██                                          ██
██                                          ██
██▓▓▓▓                                  ▓▓▓▓██
██▓▓▓▓                                  ▓▓▓▓██
██                                          ██

███╗   ██╗    ███████╗    ██╗  ██╗     ██████╗ 
████╗  ██║    ██╔════╝    ██║ ██╔╝    ██╔═══██╗
██╔██╗ ██║    █████╗      █████╔╝     ██║   ██║
██║╚██╗██║    ██╔══╝      ██╔═██╗     ██║   ██║
██║ ╚████║    ███████╗    ██║  ██╗    ╚██████╔╝
╚═╝  ╚═══╝    ╚══════╝    ╚═╝  ╚═╝     ╚═════╝ .chat

  ";
    println!("{}", ascii_art)
}
