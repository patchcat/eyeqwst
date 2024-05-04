use std::time::Duration;

use auth_screen::AuthScreen;
use config::Config;
use gateway::{Connection, GatewayMessage};
use iced::{executor, keyboard::{key, on_key_press, Key}, theme, widget::{self, container, row}, Application, Color, Command, Element, Event, Length, Renderer, Subscription, Theme};
use quaddlecl::{client::{gateway::{ClientGatewayMessage, Gateway, GatewayEvent}, http::Http, Client}, model::user::User};
use url::Url;
use auth_screen::Message as AuthMessage;
use auth_screen::IoMessage as AuthIoMessage;

use crate::channel_select::ChannelList;

pub mod auth_screen;
pub mod gateway;
pub mod config;
pub mod channel_select;
pub mod toggle_button;

const USER_AGENT: &'static str = concat!("eyeqwst/v", env!("CARGO_PKG_VERSION"));

async fn sleep(d: Duration) {
    tokio::time::sleep(d).await;
}

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
}

pub enum EyeqwstState {
    Authenticating(AuthScreen),
    LoggedIn {
        server: Url,
        http: Http,
        selected_channel: usize,
        gateway_state: GatewayState,
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
                *s = EyeqwstState::LoggedIn { http, server, selected_channel: 0, gateway_state: GatewayState::Disconnected };
            },
            (EyeqwstState::LoggedIn { gateway_state, .. },
             Message::GatewayEvent(GatewayMessage::Connected { user, conn, .. })) => {
                *gateway_state = GatewayState::Connected { user, conn };
            },
            (_, Message::GatewayEvent(GatewayMessage::ConnectionError(e))) =>
                log::warn!("gateway connection error: {e}"),
            (EyeqwstState::Authenticating(scr), Message::AuthScreen(msg)) =>
                return scr.update(msg).map(Message::AuthScreen),
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
            EyeqwstState::LoggedIn { selected_channel, gateway_state, server, .. }  => {
                log::debug!("gateway state: {gateway_state:?}");
                row![
                    container({
                        ChannelList::new(
                            gateway_state
                                .user()
                                .and_then(|user| self.config.get_account_config(server, user.id))
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
                        })
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
