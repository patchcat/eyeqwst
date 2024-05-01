use futures::SinkExt;
use reqwest::Client;
use reqwest_websocket::{RequestBuilderExt, WebSocket};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use crate::model::{channel::ChannelId, message::Message, user::User};

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("websocket error")]
    Websocket(#[from] reqwest_websocket::Error),
    #[error("invalid Quaddle URL: {0}")]
    InvalidUrl(Url),
}

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

pub struct Gateway {
    ws: WebSocket,
}

impl Gateway {
    /// Connects to the gateway of the Quaddle instance at `quaddle_url`.
    pub async fn connect(mut quaddle_url: Url) -> Result<Gateway, Error> {
        let Ok(mut segments) = quaddle_url.path_segments_mut() else {
            return Err(Error::InvalidUrl(quaddle_url))
        };

        segments.push("app");

        drop(segments);

        let ws = Client::default()
            .get(quaddle_url)
            .upgrade()
            .send()
            .await?
            .into_websocket()
            .await?;

        Ok(Self { ws })
    }
}

impl Drop for Gateway {
    fn drop(&mut self) {
        drop(self.ws.close());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect() {
        let url = Url::parse("http://localhost:8080")
            .expect("failed to parse URL");

        Gateway::connect(url)
            .await
            .expect("failed to connect to local Quaddle server");
    }
}
