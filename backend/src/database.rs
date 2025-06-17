use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub google_id: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Chat {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub system_prompt: Option<String>,
    pub provider: String,
    pub model: String,
    pub pinned: bool,
    pub is_branch: bool,
    pub parent_chat_id: Option<String>,
    pub branch_point_message_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub chat_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct UserApiKey {
    pub user_id: String,
    pub provider: String,
    #[serde(skip_serializing)]
    pub encrypted_key: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct UserModel {
    pub user_id: String,
    pub provider: String,
    pub model_id: String,
    pub model_name: String,
    pub is_enabled: bool,
    pub display_order: i32,
    pub created_at: String,
}
