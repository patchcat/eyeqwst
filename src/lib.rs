use std::sync::Arc;

use auth_screen::AuthScreen;
use auth_screen::IoMessage as AuthIoMessage;
use auth_screen::Message as AuthMessage;
use channel_select::ChannelEditMessage;
use channel_select::ChannelEditStrip;
use config::Config;
use editor::send_message;
use gateway::{Connection, GatewayMessage};
use iced::widget::text;
use iced::widget::text_editor;
use iced::Background;
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
use quaddlecl::client;
use quaddlecl::model::message::Message as QMessage;
use quaddlecl::{
    client::{
        gateway::{ClientGatewayMessage, GatewayEvent},
        http::{self, Http},
    },
    model::user::User,
};

use url::Url;

use crate::utils::icon;
use crate::utils::ErrorWithCauses;
use crate::{channel_select::ChannelList, editor::MessageEditor, messageview::qmessage_list};

pub mod auth_screen;
pub mod channel_select;
pub mod config;
pub mod editor;
pub mod gateway;
pub mod messageview;
pub mod toggle_button;
pub mod utils;

const USER_AGENT: &str = concat!("eyeqwst/v", env!("CARGO_PKG_VERSION"));
pub const DEFAULT_FONT: Font = Font::with_name("Roboto");
pub const DEFAULT_FONT_MEDIUM: Font = Font {
    weight: iced::font::Weight::Medium,
    ..DEFAULT_FONT
};
const DISCONNECTED: &str = "\u{f0783}";
const CONNECTING: &str = "\u{f08bd}";

#[derive(Debug)]
pub enum GatewayState {
    Disconnected {
        error: Option<client::gateway::Error>,
    },
    Connected {
        user: User,
        conn: Connection,
    },
}

impl GatewayState {
    pub fn user(&self) -> Option<&User> {
        match self {
            GatewayState::Connected { user, .. } => Some(user),
            GatewayState::Disconnected { .. } => None,
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
        channel_edit_strip: ChannelEditStrip,
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
    ChannelEditStrip(ChannelEditMessage),
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
                    gateway_state: GatewayState::Disconnected { error: None },
                    channel_edit_strip: ChannelEditStrip::default(),
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
                    .get_account_config(server, user.id)
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
                        .channel_at(gateway_state, server, *selected_channel)
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
            (
                EyeqwstState::LoggedIn { gateway_state, .. },
                Message::GatewayEvent(GatewayMessage::DialError(error)),
            ) => {
                *gateway_state = GatewayState::Disconnected { error: Some(error) };
            }
            (
                EyeqwstState::LoggedIn { gateway_state, .. },
                Message::GatewayEvent(GatewayMessage::Disconnected),
            ) => {
                *gateway_state = GatewayState::Disconnected { error: None };
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
                let Some(channel) = self.config.channel_at(gateway_state, server, new_selected)
                else {
                    return Command::none();
                };

                log::debug!("selected channel: {new_selected}");
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
                        .channel_at(gateway_state, server, *selected_channel)
                else {
                    return Command::none();
                };
                return Command::batch([
                    send_message(
                        Arc::clone(http),
                        editor,
                        channel.id,
                        |_| Message::SentSuccessfully,
                        Message::SendError,
                    ),
                    snap_to(scrollable::Id::new(QMESSAGELIST_ID), RelativeOffset::START),
                ]);
            }
            (
                EyeqwstState::LoggedIn {
                    http,
                    server,
                    gateway_state,
                    channel_edit_strip,
                    ..
                },
                Message::ChannelEditStrip(msg),
            ) => {
                let Some(channels) = gateway_state.user().map(|user| {
                    &mut self
                        .config
                        .accounts
                        .entry(server.clone())
                        .or_default()
                        .entry(user.id)
                        .or_default()
                        .channels
                }) else {
                    return Command::none();
                };

                return channel_edit_strip
                    .update(msg, channels, Arc::clone(http))
                    .map(Message::ChannelEditStrip);
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
                channel_edit_strip,
                ..
            } => {
                log::debug!("gateway state: {gateway_state:?}");
                let account_config = gateway_state
                    .user()
                    .and_then(|user| self.config.get_account_config(server, user.id));
                let _channel =
                    account_config.and_then(|account| account.channels.get(*selected_channel));
                let el = row![
                    container({
                        column![
                            channel_edit_strip
                                .view(&self.theme())
                                .map(Message::ChannelEditStrip),
                            ChannelList::new(
                                account_config
                                    .map(|account| account.channels.iter())
                                    .into_iter()
                                    .flatten(),
                                *selected_channel,
                            )
                            .on_selection(Message::ChannelSelected)
                        ]
                        .width(Length::Fixed(200.0))
                        .height(Length::Fill)
                        .spacing(20)
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
                        qmessage_list(&self.theme(), messages),
                        Element::from({
                            container({
                                MessageEditor::new(editor)
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
                .height(Length::Fill);

                const CONNECTING_SIZE: u16 = 16;
                const CONNECTING_ICON_SIZE: u16 = 17;
                match gateway_state {
                    GatewayState::Connected { .. } => el.into(),
                    GatewayState::Disconnected { error } => {
                        let row = match error {
                            Some(err) => container(
                                row![
                                    icon(crate::DISCONNECTED).size(CONNECTING_ICON_SIZE),
                                    text(ErrorWithCauses(err))
                                        .font(DEFAULT_FONT_MEDIUM)
                                        .size(CONNECTING_SIZE)
                                ]
                                .spacing(5)
                                .padding(10),
                            )
                            .style(|t: &Theme| {
                                use container::StyleSheet;
                                container::Appearance {
                                    text_color: Some(t.extended_palette().danger.base.text),
                                    background: Some(Background::Color(
                                        t.extended_palette().danger.base.color,
                                    )),
                                    ..t.appearance(&theme::Container::Box)
                                }
                            }),
                            None => container(
                                row![
                                    icon(crate::CONNECTING).size(CONNECTING_ICON_SIZE),
                                    text("Connecting...")
                                        .font(DEFAULT_FONT_MEDIUM)
                                        .size(CONNECTING_SIZE)
                                ]
                                .spacing(5)
                                .padding(10),
                            )
                            .style(|t: &Theme| {
                                use container::StyleSheet;
                                container::Appearance {
                                    text_color: Some(t.extended_palette().background.strong.text),
                                    background: Some(Background::Color(
                                        t.extended_palette().background.strong.color,
                                    )),
                                    ..t.appearance(&theme::Container::Box)
                                }
                            }),
                        };
                        column![row.width(Length::Fill).center_x(), el]
                            .height(Length::Fill)
                            .width(Length::Fill)
                            .into()
                    }
                }
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
        iced::Theme::Light
    }
}
