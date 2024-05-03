use auth_screen::AuthScreen;
use gateway::GatewayMessage;
use iced::{executor, keyboard::{key, on_key_press, Key}, widget, Application, Command, Element, Event, Renderer, Subscription, Theme};
use quaddlecl::client::{gateway::{ClientGatewayMessage, Gateway}, http::Http, Client};
use url::Url;

pub mod auth_screen;
pub mod gateway;

const USER_AGENT: &'static str = concat!("eyeqwst/v", env!("CARGO_PKG_VERSION"));

pub enum Eyeqwst {
    Authenticating(AuthScreen),
    LoggedIn {
        server: Url,
        http: Http,
    }
}

#[derive(Debug)]
pub enum Message {
    AuthScreen(auth_screen::Message),
    TabPressed,
    GatewayEvent(GatewayMessage),
}

impl Application for Eyeqwst {
    type Executor = executor::Default;

    type Message = Message;

    type Theme = Theme;

    type Flags = ();

    fn new((): Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Eyeqwst::Authenticating(AuthScreen::default()),
            Command::none()
        )
    }

    fn title(&self) -> String {
        String::from("eyeqwst")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match (self, message) {
            (Self::Authenticating(scr), Message::AuthScreen(msg)) =>
                return scr.update(msg).map(Message::AuthScreen),
            (_, Message::TabPressed) =>
                return widget::focus_next(),
            _ => {},
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, Renderer> {
        match self {
            Self::Authenticating(scr) => {
                scr.view(&self.theme())
                    .map(Message::AuthScreen)
            },
            _ => todo!()
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        Subscription::batch([
            if let Self::LoggedIn { server, http, .. } = self {
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
}
