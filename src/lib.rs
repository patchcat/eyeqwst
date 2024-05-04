use std::{collections::HashMap, sync::Arc, time::Duration};

use auth_screen::AuthScreen;
use config::{Channel, Config};
use gateway::{Connection, GatewayMessage};
use iced::{executor, keyboard::{key, on_key_press, Key}, theme, widget::{self, container, row, scrollable, Column, Rule}, Application, Color, Command, Element, Event, Font, Length, Pixels, Renderer, Subscription, Theme};
use messageview::retrieve_history;
use quaddlecl::{client::{gateway::{ClientGatewayMessage, Gateway, GatewayEvent}, http::{self, Http}, Client}, model::{channel::ChannelId, user::User}};
use quaddlecl::model::message::Message as QMessage;
use url::Url;
use std::iter;
use std::error::Error;
use auth_screen::Message as AuthMessage;
use auth_screen::IoMessage as AuthIoMessage;

use crate::{channel_select::ChannelList, messageview::{qmessage_list, QMessageWidget}, utils::Gaps};

pub mod auth_screen;
pub mod gateway;
pub mod config;
pub mod channel_select;
pub mod toggle_button;
pub mod messageview;
pub mod utils;

const USER_AGENT: &'static str = concat!("eyeqwst/v", env!("CARGO_PKG_VERSION"));
pub const DEFAULT_FONT: Font = Font::with_name("Roboto");


#[derive(Debug)]
pub enum GatewayState {
    Disconnected,
    Connected { user: User, conn: Connection }
}

impl GatewayState {
    pub fn user(&self) -> Option<&User> {
        match self {
            GatewayState::Connected { user, .. } => Some(user),
            GatewayState::Disconnected => None,
        }
    }

    pub fn channel_at<'a>(&self, config: &'a Config, server: &Url, idx: usize) -> Option<&'a Channel> {
        config
            .get_account_config(server, self.user()?.id)?
            .channels
            .get(idx)

    }
}

pub enum EyeqwstState {
    Authenticating(AuthScreen),
    LoggedIn {
        server: Url,
        http: Arc<Http>,
        selected_channel: usize,
        gateway_state: GatewayState,
        /// messages in the current channel
        messages: Vec<QMessage>,
    }
}

pub struct Eyeqwst {
    state: EyeqwstState,
    config: Config,
}

#[derive(Debug)]
pub enum Message {
    AuthScreen(AuthMessage),
    TabPressed,
    HistoryRetrieved(Vec<QMessage>),
    HistoryRetrievalError(http::Error),
    GatewayEvent(GatewayMessage),
    ChannelSelected(usize),
}

impl Application for Eyeqwst {
    type Executor = executor::Default;

    type Message = Message;

    type Theme = Theme;

    type Flags = ();

    fn new((): Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                state: EyeqwstState::Authenticating(AuthScreen::default()),
                config: Config::load(),
            },
            Command::none()
        )
    }

    fn title(&self) -> String {
        String::from("eyeqwst")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match (&mut self.state, message) {
            (s@EyeqwstState::Authenticating(_),
             Message::AuthScreen(AuthMessage::Io(AuthIoMessage::LoginSucceeded(http, server)))) => {
                *s = EyeqwstState::LoggedIn {
                    http: Arc::new(http),
                    server,
                    selected_channel: 0,
                    gateway_state: GatewayState::Disconnected,
                    messages: Vec::default(),
                };
            },
            (EyeqwstState::Authenticating(scr), Message::AuthScreen(msg)) =>
                return scr.update(msg).map(Message::AuthScreen),
            (EyeqwstState::LoggedIn { http, gateway_state, server, selected_channel, .. },
             Message::GatewayEvent(GatewayMessage::Connected { user, mut conn, .. })) => {
                let channels = self.config.get_account_config(&server, user.id)
                    .map(|c| c.channels.iter())
                    .into_iter()
                    .flatten();
                for channel in channels {
                    conn.send(ClientGatewayMessage::Subscribe {
                        channel_id: channel.id
                    });
                }
                *gateway_state = GatewayState::Connected { user, conn };
                if let Some(channel) = gateway_state.channel_at(&self.config, &server, *selected_channel) {
                    return retrieve_history(
                        Arc::clone(http),
                        channel.id, None, Message::HistoryRetrieved, Message::HistoryRetrievalError
                    )
                }
            },
            (_, Message::GatewayEvent(GatewayMessage::ConnectionError(e))) =>
                log::warn!("gateway connection error: {e}"),
            (EyeqwstState::LoggedIn { messages, selected_channel, gateway_state, server, .. },
             Message::GatewayEvent(GatewayMessage::Event(GatewayEvent::MessageCreate { message }))) => {
                let is_relevant = gateway_state.channel_at(&self.config, &server, *selected_channel)
                    .is_some_and(|c| c.id == message.channel);
                if is_relevant {
                    messages.push(message);
                }
            },
            (EyeqwstState::LoggedIn { http, server, selected_channel, messages, gateway_state, .. },
             Message::ChannelSelected(new_selected)) => {
                let Some(channel) = gateway_state.channel_at(&self.config, &server, *selected_channel)
                else {
                    return Command::none();
                };
                *selected_channel = new_selected;
                *messages = Vec::new();
                return retrieve_history(
                    Arc::clone(http),
                    channel.id,
                    None,
                    Message::HistoryRetrieved,
                    Message::HistoryRetrievalError
                );
            },
            (EyeqwstState::LoggedIn { messages, .. },
             Message::HistoryRetrieved(mut new_msgs)) => {
                new_msgs.reverse();
                *messages = new_msgs
            },
            (_, Message::TabPressed) =>
                return widget::focus_next(),
            _ => {},
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, Renderer> {
        match &self.state {
            EyeqwstState::Authenticating(scr) => {
                scr.view(&self.theme())
                    .map(Message::AuthScreen)
            },
            EyeqwstState::LoggedIn { selected_channel, gateway_state, server, messages, .. }  => {
                log::debug!("gateway state: {gateway_state:?}");
                let account_config = gateway_state
                    .user()
                    .and_then(|user| self.config.get_account_config(server, user.id));
                row![
                    container({
                        ChannelList::new(
                            account_config
                                .map(|account| account.channels.iter())
                                .into_iter()
                                .flatten(),
                            *selected_channel,
                        )
                            .on_selection(Message::ChannelSelected)
                            .width(Length::Fixed(200.0))
                            .height(Length::Fill)
                    })
                        .padding(10)
                        .style({
                            theme::Container::Custom(Box::new({
                                |t: &Theme| {
                                    use iced::widget::container::StyleSheet;
                                    let color = match t.extended_palette().is_dark {
                                        true => Color::from_rgba8(255, 255, 255, 0.05),
                                        false => Color::from_rgba8(0, 0, 0, 0.05),
                                    };
                                    widget::container::Appearance {
                                        background: Some(iced::Background::Color(color)),
                                        ..t.appearance(&theme::Container::Transparent)
                                    }
                                }
                            }))
                        }),
                    qmessage_list(messages)
                ]
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            }
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        Subscription::batch([
            if let EyeqwstState::LoggedIn { server, http, .. } = &self.state {
                gateway::connect(server.clone(), http.token().unwrap().to_string())
                    .map(Message::GatewayEvent)
            } else {
                Subscription::none()
            },
            on_key_press(|key, _| match key {
                Key::Named(key::Named::Tab) => Some(Message::TabPressed),
                _ => None,
            })
        ])
    }

    fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }
}
