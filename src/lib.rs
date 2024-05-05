use std::sync::Arc;

use auth_screen::AuthScreen;
use auth_screen::IoMessage as AuthIoMessage;
use auth_screen::Message as AuthMessage;
use config::Config;
use editor::send_message;
use gateway::{Connection, GatewayMessage};
use iced::widget::text_editor;
use iced::{
    executor,
    keyboard::{key, on_key_press, Key},
    theme,
    widget::{
        self, column, container, row,
        scrollable::{self, snap_to, RelativeOffset},
    },
    Application, Color, Command, Element, Font, Length, Renderer, Subscription, Theme,
};
use messageview::{retrieve_history, QMESSAGELIST_ID};
use quaddlecl::model::message::Message as QMessage;
use quaddlecl::{
    client::{
        gateway::{ClientGatewayMessage, GatewayEvent},
        http::{self, Http},
    },
    model::user::User,
};

use url::Url;

use crate::{channel_select::ChannelList, editor::MessageEditor, messageview::qmessage_list};

pub mod auth_screen;
pub mod channel_select;
pub mod config;
pub mod editor;
pub mod gateway;
pub mod messageview;
pub mod toggle_button;
pub mod utils;

const USER_AGENT: &'static str = concat!("eyeqwst/v", env!("CARGO_PKG_VERSION"));
pub const DEFAULT_FONT: Font = Font::with_name("Roboto");

#[derive(Debug)]
pub enum GatewayState {
    Disconnected,
    Connected { user: User, conn: Connection },
}

impl GatewayState {
    pub fn user(&self) -> Option<&User> {
        match self {
            GatewayState::Connected { user, .. } => Some(user),
            GatewayState::Disconnected => None,
        }
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
        editor: text_editor::Content,
    },
}

pub struct Eyeqwst {
    state: EyeqwstState,
    config: Config,
}

#[derive(Debug, Clone)]
pub enum EditorMessage {
    Action(text_editor::Action),
    SendInitiated,
}

#[derive(Debug)]
pub enum Message {
    AuthScreen(AuthMessage),
    TabPressed,
    HistoryRetrieved(Vec<QMessage>),
    HistoryRetrievalError(http::Error),
    GatewayEvent(GatewayMessage),
    ChannelSelected(usize),
    Editor(EditorMessage),
    SentSuccessfully,
    SendError(http::Error),
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
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("eyeqwst")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match (&mut self.state, message) {
            (
                s @ EyeqwstState::Authenticating(_),
                Message::AuthScreen(AuthMessage::Io(AuthIoMessage::LoginSucceeded(http, server))),
            ) => {
                *s = EyeqwstState::LoggedIn {
                    http: Arc::new(http),
                    server,
                    selected_channel: 0,
                    gateway_state: GatewayState::Disconnected,
                    messages: Vec::default(),
                    editor: text_editor::Content::new(),
                };
            }
            (EyeqwstState::Authenticating(scr), Message::AuthScreen(msg)) => {
                return scr.update(msg).map(Message::AuthScreen)
            }
            (
                EyeqwstState::LoggedIn {
                    http,
                    gateway_state,
                    server,
                    selected_channel,
                    ..
                },
                Message::GatewayEvent(GatewayMessage::Connected { user, mut conn, .. }),
            ) => {
                let channels = self
                    .config
                    .get_account_config(&server, user.id)
                    .map(|c| c.channels.iter())
                    .into_iter()
                    .flatten();
                for channel in channels {
                    conn.send(ClientGatewayMessage::Subscribe {
                        channel_id: channel.id,
                    });
                }
                *gateway_state = GatewayState::Connected { user, conn };
                if let Some(channel) =
                    self.config
                        .channel_at(gateway_state, &server, *selected_channel)
                {
                    return retrieve_history(
                        Arc::clone(http),
                        channel.id,
                        None,
                        Message::HistoryRetrieved,
                        Message::HistoryRetrievalError,
                    );
                }
            }
            (_, Message::GatewayEvent(GatewayMessage::ConnectionError(e))) => {
                log::warn!("gateway connection error: {e}")
            }
            (
                EyeqwstState::LoggedIn {
                    messages,
                    selected_channel,
                    gateway_state,
                    server,
                    ..
                },
                Message::GatewayEvent(GatewayMessage::Event(GatewayEvent::MessageCreate {
                    message,
                })),
            ) => {
                let is_relevant = self
                    .config
                    .channel_at(gateway_state, server, *selected_channel)
                    .is_some_and(|c| c.id == message.channel);
                if is_relevant {
                    messages.push(message);
                }
            }
            (
                EyeqwstState::LoggedIn {
                    http,
                    server,
                    selected_channel,
                    messages,
                    gateway_state,
                    ..
                },
                Message::ChannelSelected(new_selected),
            ) => {
                let Some(channel) =
                    self.config
                        .channel_at(gateway_state, server, *selected_channel)
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
                    Message::HistoryRetrievalError,
                );
            }
            (
                EyeqwstState::LoggedIn {
                    http,
                    selected_channel,
                    gateway_state,
                    server,
                    editor,
                    ..
                },
                Message::Editor(EditorMessage::SendInitiated),
            ) => {
                let Some(channel) =
                    self.config
                        .channel_at(gateway_state, &server, *selected_channel)
                else {
                    return Command::none();
                };
                return Command::batch([
                    send_message(
                        Arc::clone(&http),
                        editor,
                        channel.id,
                        |_| Message::SentSuccessfully,
                        Message::SendError,
                    ),
                    snap_to(scrollable::Id::new(QMESSAGELIST_ID), RelativeOffset::START),
                ]);
            }
            (EyeqwstState::LoggedIn { messages, .. }, Message::HistoryRetrieved(mut new_msgs)) => {
                new_msgs.reverse();
                *messages = new_msgs
            }
            (
                EyeqwstState::LoggedIn { editor, .. },
                Message::Editor(EditorMessage::Action(action)),
            ) => editor.perform(action),
            (_, Message::TabPressed) => return widget::focus_next(),
            _ => {}
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, Renderer> {
        match &self.state {
            EyeqwstState::Authenticating(scr) => scr.view(&self.theme()).map(Message::AuthScreen),
            EyeqwstState::LoggedIn {
                selected_channel,
                gateway_state,
                server,
                messages,
                editor,
                ..
            } => {
                log::debug!("gateway state: {gateway_state:?}");
                let account_config = gateway_state
                    .user()
                    .and_then(|user| self.config.get_account_config(server, user.id));
                let _channel =
                    account_config.and_then(|account| account.channels.get(*selected_channel));
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
                    column![
                        qmessage_list(messages),
                        Element::from({
                            container({
                                MessageEditor::new(&editor)
                                    .on_action(EditorMessage::Action)
                                    .on_enter(EditorMessage::SendInitiated)
                                    .padding(10)
                            })
                            .padding(10)
                        })
                        .map(Message::Editor),
                    ]
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
            }),
        ])
    }

    fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }
}
