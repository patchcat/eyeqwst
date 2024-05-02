use std::task::Poll;

use futures::stream::FusedStream;
use futures::{Sink, SinkExt, Stream, StreamExt, TryStreamExt};
use reqwest::header::USER_AGENT;
use reqwest::Client;
use reqwest_websocket::{RequestBuilderExt, WebSocket};
use reqwest_websocket::Message as WsMessage;
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
    #[error("serialization error")]
    Serialization(#[from] serde_json::Error),
    #[error("unexpected binary message")]
    UnexpectedBinaryMessage,
    #[error("gateway error: {0}")]
    GatewayError(String),
    #[error("unexpected event: {0:?}")]
    UnexpectedEvent(GatewayEvent),
    #[error("socket closed")]
    UnexpectedSocketClose,
}

/// Gateway messages that the client makes.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "op", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ClientGatewayMessage {
    Identify { token: String },
    Subscribe { channel_id: ChannelId },
}

/// Gateway messages that the server makes.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "event", rename_all = "snake_case")]
#[non_exhaustive]
pub enum GatewayEvent {
    Ready { session_id: String, user: User },
    Error { reason: String },
    MessageCreate { message: Message },
}

pub struct Gateway {
    ws: WebSocket,
    closed: bool,
}

impl Gateway {
    /// Connects to the gateway of the Quaddle instance at `quaddle_url`.
    pub async fn connect(mut quaddle_url: Url, user_agent: String) -> Result<Gateway, Error> {
        let Ok(mut segments) = quaddle_url.path_segments_mut() else {
            return Err(Error::InvalidUrl(quaddle_url))
        };

        segments.push("app");

        drop(segments);

        let ws = Client::default()
            .get(quaddle_url)
            .header(USER_AGENT, user_agent)
            .upgrade()
            .send()
            .await?
            .into_websocket()
            .await?;

        Ok(Self { ws, closed: true })
    }

    /// Sends an identify message and returns the session ID.
    pub async fn identify(&mut self, token: String) -> Result<(String, User), Error> {
        self.send(ClientGatewayMessage::Identify { token })
            .await?;

        match self.try_next().await? {
            Some(GatewayEvent::Ready { session_id, user }) => Ok((session_id, user)),
            Some(GatewayEvent::Error { reason }) => Err(Error::GatewayError(reason)),
            Some(ev) => Err(Error::UnexpectedEvent(ev)),
            None => Err(Error::UnexpectedSocketClose),
        }
    }

    /// Subscribes to the channel with ID `channel_id`
    pub async fn subscribe(&mut self, channel_id: ChannelId) -> Result<(), Error> {
        self.send(ClientGatewayMessage::Subscribe { channel_id })
            .await
    }
}

/// A lower-level way of sending gateway messages.
/// Prefer using the dedicated associated functions.
impl Sink<ClientGatewayMessage> for Gateway {
    type Error = Error;

    fn poll_ready(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.ws.poll_ready_unpin(cx).map_err(Into::into)
    }

    fn start_send(mut self: std::pin::Pin<&mut Self>, msg: ClientGatewayMessage) -> Result<(), Self::Error> {
        self.ws.start_send_unpin(WsMessage::Text(serde_json::to_string(&msg)?))
            .map_err(Into::into)
    }

    fn poll_flush(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.ws.poll_flush_unpin(cx).map_err(Into::into)
    }

    fn poll_close(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.ws.poll_close_unpin(cx).map_err(Into::into)
    }
}

impl Stream for Gateway {
    type Item = Result<GatewayEvent, Error>;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        if self.closed {
            return Poll::Ready(None);
        }

        self.ws
            .poll_next_unpin(cx)
            .map_err(Error::from)
            .map(|r| match r {
                Some(Ok(WsMessage::Binary(_))) => Some(Err(Error::UnexpectedBinaryMessage)),
                Some(Ok(WsMessage::Text(txt))) => Some(serde_json::from_str(&txt).map_err(Into::into)),
                Some(Err(e)) => Some(Err(e)),
                None => None,
            })
    }
}

impl FusedStream for Gateway {
    fn is_terminated(&self) -> bool {
        self.closed
    }
}

impl Drop for Gateway {
    fn drop(&mut self) {
        drop(self.close());
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;
    use crate::client::http::tests::{make_http, make_signed_in, make_username};

    pub async fn make_gateway() -> Gateway {
        let url = Url::parse("http://localhost:8080")
            .expect("could not parse URL");

        Gateway::connect(url, "quaddlecl tester".to_string())
            .await
            .expect("failed to connect to local Quaddle server")
    }

    #[tokio::test]
    async fn test_connect() {
        let url = Url::parse("http://localhost:8080")
            .expect("failed to parse URL");

        Gateway::connect(url, "quaddlecl tester".to_string())
            .await
            .expect("failed to connect to local Quaddle server");
    }

    #[tokio::test]
    async fn test_identify() {
        let uname = make_username();
        let mut http = make_http();
        let mut gateway = make_gateway().await;

        http.signup(&uname, "the_meower")
            .await
            .expect("failed to sign up");

        http.login(&uname, "the_meower")
            .await
            .expect("failed to log in");

        let (_, user) = gateway
            .identify(http.token().expect("not logged in").to_string())
            .await
            .expect("failed to identify");

        assert_eq!(user.name, uname);
    }

    #[tokio::test]
    #[serial(message_create)]
    async fn test_subscribe() {
        let http = make_signed_in().await;
        let mut gateway = make_gateway().await;

        gateway.identify(http.token().expect("not logged in").to_string())
               .await
               .expect("failed to identify");

        gateway.subscribe(ChannelId(1))
               .await
               .expect("failed to send the subscribe message");

        http.create_message(ChannelId(1), "sussy balls")
            .await
            .expect("failed to send a message");

        let GatewayEvent::MessageCreate { message } = gateway
            .try_next()
            .await
            .expect("error receiving event")
            .expect("gateway socket closed")
        else {
            panic!("received an unexpected event")
        };

        assert_eq!(message.content, "sussy balls");
    }
}
