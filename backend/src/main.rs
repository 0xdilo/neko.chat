mod auth;
mod config;
mod database;
mod error;
mod handlers;
mod llm;
mod routes;

use axum::extract::FromRef;
use axum::http::{HeaderValue, Method};
use bcrypt;
use config::Config;
use database::Message;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    db_pool: SqlitePool,
    config: Config,
    tx: broadcast::Sender<Message>,
}

impl FromRef<AppState> for SqlitePool {
    fn from_ref(app_state: &AppState) -> SqlitePool {
        app_state.db_pool.clone()
    }
}

impl FromRef<AppState> for Config {
    fn from_ref(app_state: &AppState) -> Config {
        app_state.config.clone()
    }
}

impl FromRef<AppState> for broadcast::Sender<Message> {
    fn from_ref(app_state: &AppState) -> broadcast::Sender<Message> {
        app_state.tx.clone()
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    print_neko();

    let config = Config::from_env();

    let db_pool = SqlitePoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await
        .expect("failed to connect to database");

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
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
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
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )"#,
        r#"CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            chat_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
            FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE
        )"#,
        r#"CREATE TABLE IF NOT EXISTS user_api_keys (
            user_id TEXT NOT NULL,
            provider TEXT NOT NULL,
            encrypted_key TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
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
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )"#,
        r#"CREATE TABLE IF NOT EXISTS user_settings (
            user_id TEXT PRIMARY KEY,
            theme TEXT NOT NULL DEFAULT 'dark',
            language TEXT NOT NULL DEFAULT 'en',
            font_size INTEGER NOT NULL DEFAULT 14,
            notifications_enabled BOOLEAN NOT NULL DEFAULT true,
            auto_save BOOLEAN NOT NULL DEFAULT true,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )"#,
        r#"CREATE TABLE IF NOT EXISTS user_models (
            user_id TEXT NOT NULL,
            provider TEXT NOT NULL,
            model_id TEXT NOT NULL,
            model_name TEXT NOT NULL,
            is_enabled BOOLEAN NOT NULL DEFAULT true,
            display_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
            PRIMARY KEY (user_id, provider, model_id),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
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
        let admin_password_hash =
            bcrypt::hash("admin", bcrypt::DEFAULT_COST).expect("failed to hash admin password");

        let admin_insert_result = sqlx::query(
            r#"INSERT OR IGNORE INTO users (id, email, name, password_hash, role)
            VALUES ($1, $2, $3, $4, $5)"#,
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

    let migration_result =
        sqlx::query("ALTER TABLE chats ADD COLUMN is_branch BOOLEAN DEFAULT false")
            .execute(&db_pool)
            .await;

    match migration_result {
        Ok(_) => tracing::info!("added is_branch column to chats table"),
        Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column") => {
            tracing::info!("is_branch column already exists, skipping migration");
        }
        Err(e) => {
            tracing::error!("failed to add is_branch column: {}", e);
            panic!("migration failed: {}", e);
        }
    }

    let migration_result = sqlx::query("ALTER TABLE chats ADD COLUMN parent_chat_id TEXT")
        .execute(&db_pool)
        .await;

    match migration_result {
        Ok(_) => tracing::info!("added parent_chat_id column to chats table"),
        Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column") => {
            tracing::info!("parent_chat_id column already exists, skipping migration");
        }
        Err(e) => {
            tracing::error!("failed to add parent_chat_id column: {}", e);
            panic!("migration failed: {}", e);
        }
    }

    let migration_result = sqlx::query("ALTER TABLE chats ADD COLUMN branch_point_message_id TEXT")
        .execute(&db_pool)
        .await;

    match migration_result {
        Ok(_) => {
            tracing::info!("added branch_point_message_id column to chats table")
        }
        Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column") => {
            tracing::info!("branch_point_message_id column already exists, skipping migration");
        }
        Err(e) => {
            tracing::error!("failed to add branch_point_message_id column: {}", e);
            panic!("migration failed: {}", e);
        }
    }

    tracing::info!("all migrations completed");

    let (tx, _) = broadcast::channel::<Message>(100);

    let app_state = AppState {
        db_pool,
        config: config.clone(),
        tx,
    };

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
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
    let addr: SocketAddr = addr_str.parse().expect("invalid server address format");
    tracing::info!("server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
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
