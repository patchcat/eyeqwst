use serde::{Deserialize, Serialize};

use crate::model::{channel::ChannelId, message::Message, user::User};

/// Gateway messages that the client makes.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
#[non_exhaustive]
enum ClientGatewayMessage {
    Identify { token: String },
    Subscribe { channel_id: ChannelId },
}

/// Gateway messages that the server makes.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
#[non_exhaustive]
pub enum GatewayEvent {
    Ready { session_id: String, user: User },
    Error { reason: String },
    MessageCreate { message: Message },
}
