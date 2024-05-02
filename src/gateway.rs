use std::{any::TypeId, convert::Infallible, error::Error};

use futures::{channel::mpsc, select, SinkExt, StreamExt};
use iced::{subscription, Subscription};
use quaddlecl::{client::gateway::{self, ClientGatewayMessage, Gateway, GatewayEvent}, model::user::User};
use url::Url;

use crate::USER_AGENT;

#[derive(Debug, Clone)]
pub struct Connection(mpsc::UnboundedSender<ClientGatewayMessage>);

#[derive(Debug)]
pub enum GatewayMessage {
    Connected {
        conn: Connection,
        user: User,
        session_id: String
    },
    ConnectionError(gateway::Error),
    Disconnected,
    Event(GatewayEvent),
}

impl Connection {
    pub fn send(&mut self, msg: ClientGatewayMessage) -> bool {
        self.0.unbounded_send(msg).is_ok()
    }
}

enum GatewayState {
    Disconnected,
    Connected {
        gateway: Gateway,
        receiver: mpsc::UnboundedReceiver<ClientGatewayMessage>,
    }
}

async fn gateway_service(mut output: mpsc::Sender<GatewayMessage>, url: Url, token: String) -> Infallible {
    let mut state = GatewayState::Disconnected;
    loop {
        match state {
            GatewayState::Disconnected => {
                let gateway_res = Gateway
                    ::connect(url.clone(), USER_AGENT.to_string())
                    .await;

                let mut gateway = match gateway_res {
                    Ok(x) => x,
                    Err(e) => {
                        let _ = output.send(GatewayMessage::ConnectionError(e)).await;
                        continue
                    },
                };

                let (session_id, user) = match gateway.identify(token.to_string()).await {
                    Ok(x) => x,
                    Err(e) => {
                        let _ = output.send(GatewayMessage::ConnectionError(e)).await;
                        continue
                    }
                };

                let (sender, receiver) = mpsc::unbounded();

                let _ = output
                    .send(GatewayMessage::Connected {
                        conn: Connection(sender),
                        user,
                        session_id,
                    })
                    .await;

                state = GatewayState::Connected { gateway, receiver };
            },
            GatewayState::Connected { ref mut gateway, ref mut receiver } => {
                select! {
                    gateway_res = gateway.next() => {
                        match gateway_res {
                            Some(Ok(ev)) => {
                                let _ = output
                                    .try_send(GatewayMessage::Event(ev));
                            },
                            Some(Err(e)) => {
                                let _ = output
                                    .try_send(GatewayMessage::ConnectionError(e));
                            },
                            None => {
                                let _ = output.send(GatewayMessage::Disconnected)
                                              .await;
                                state = GatewayState::Disconnected;
                            },
                        }
                    },
                    new_message = receiver.select_next_some() => {
                        let _ = gateway.send(new_message)
                                       .await;
                    }
                }
            }
        }
    }
}

pub fn connect(url: Url, token: String) -> Subscription<GatewayMessage> {
    struct Connect;

    subscription::channel(
        TypeId::of::<Connect>(),
        50,
        |output| gateway_service(output, url, token)
    )
}
