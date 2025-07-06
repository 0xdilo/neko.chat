use serde::{Serialize, Deserialize};
use crate::database::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WsMessageType {
    ChatMessage,
    MessageUpdate,
    ChatUpdate,
    ChatDelete,
    TypingStart,
    TypingStop,
    UserOnline,
    UserOffline,
    SystemNotification,
    SettingsUpdate,
    UsageUpdate,
    Ping,
    Pong,
    Auth,
    AuthSuccess,
    AuthError,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    #[serde(rename = "type")]
    pub message_type: WsMessageType,
    pub data: serde_json::Value,
}

impl WsMessage {
    pub fn new_chat_message(message: Message) -> Self {
        WsMessage {
            message_type: WsMessageType::ChatMessage,
            data: serde_json::to_value(message).unwrap_or_default(),
        }
    }

    pub fn new_message_update(chat_id: String, old_id: String, new_id: String) -> Self {
        WsMessage {
            message_type: WsMessageType::MessageUpdate,
            data: serde_json::json!({
                "chat_id": chat_id,
                "old_id": old_id,
                "new_id": new_id,
            }),
        }
    }

    pub fn new_chat_update(chat: crate::database::Chat) -> Self {
        WsMessage {
            message_type: WsMessageType::ChatUpdate,
            data: serde_json::to_value(chat).unwrap_or_default(),
        }
    }

    pub fn new_chat_delete(chat_id: String) -> Self {
        WsMessage {
            message_type: WsMessageType::ChatDelete,
            data: serde_json::json!({
                "chat_id": chat_id,
            }),
        }
    }

    pub fn new_typing_start(chat_id: String, user_id: String) -> Self {
        WsMessage {
            message_type: WsMessageType::TypingStart,
            data: serde_json::json!({
                "chat_id": chat_id,
                "user_id": user_id,
            }),
        }
    }

    pub fn new_typing_stop(chat_id: String, user_id: String) -> Self {
        WsMessage {
            message_type: WsMessageType::TypingStop,
            data: serde_json::json!({
                "chat_id": chat_id,
                "user_id": user_id,
            }),
        }
    }

    pub fn new_user_online(user_id: String) -> Self {
        WsMessage {
            message_type: WsMessageType::UserOnline,
            data: serde_json::json!({
                "user_id": user_id,
            }),
        }
    }

    pub fn new_user_offline(user_id: String) -> Self {
        WsMessage {
            message_type: WsMessageType::UserOffline,
            data: serde_json::json!({
                "user_id": user_id,
            }),
        }
    }

    pub fn new_system_notification(message: String) -> Self {
        WsMessage {
            message_type: WsMessageType::SystemNotification,
            data: serde_json::json!({
                "message": message,
            }),
        }
    }

    pub fn new_settings_update(settings: serde_json::Value) -> Self {
        WsMessage {
            message_type: WsMessageType::SettingsUpdate,
            data: settings,
        }
    }

    pub fn new_usage_update(usage: serde_json::Value) -> Self {
        WsMessage {
            message_type: WsMessageType::UsageUpdate,
            data: usage,
        }
    }

    pub fn new_ping() -> Self {
        WsMessage {
            message_type: WsMessageType::Ping,
            data: serde_json::Value::Null,
        }
    }

    pub fn new_pong() -> Self {
        WsMessage {
            message_type: WsMessageType::Pong,
            data: serde_json::Value::Null,
        }
    }

    pub fn new_auth(token: String) -> Self {
        WsMessage {
            message_type: WsMessageType::Auth,
            data: serde_json::json!({
                "token": token,
            }),
        }
    }

    pub fn new_auth_success() -> Self {
        WsMessage {
            message_type: WsMessageType::AuthSuccess,
            data: serde_json::Value::Null,
        }
    }

    pub fn new_auth_error(message: String) -> Self {
        WsMessage {
            message_type: WsMessageType::AuthError,
            data: serde_json::json!({
                "message": message,
            }),
        }
    }

    pub fn new_error(message: String) -> Self {
        WsMessage {
            message_type: WsMessageType::Error,
            data: serde_json::json!({
                "message": message,
            }),
        }
    }
}
