use auth_screen::{login_screen, signup_screen};
use gateway::GatewayMessage;
use iced::{executor, Application, Command, Element, Event, Renderer, Subscription, Theme};
use quaddlecl::client::{gateway::{ClientGatewayMessage, Gateway}, http::Http, Client};
use url::Url;

pub mod auth_screen;
pub mod gateway;

const USER_AGENT: &'static str = concat!("eyeqwst/v", env!("CARGO_PKG_VERSION"));

pub enum Eyeqwst {
    Signup { server: String, username: String, password: String },
    Login { server: String, username: String, password: String },
    LoggedIn {
        server: Url,
        http: Http,
    }
}

#[derive(Debug)]
pub enum Message {
    AuthScreen(auth_screen::Message),
    GatewayEvent(GatewayMessage),
}

impl Application for Eyeqwst {
    type Executor = executor::Default;

    type Message = Message;

    type Theme = Theme;

    type Flags = ();

    fn new((): Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Eyeqwst::Login {
                server: String::new(),
                username: String::new(),
                password: String::new()
            },
            Command::none()
        )
    }

    fn title(&self) -> String {
        String::from("eyeqwst")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        use auth_screen::Message::{ServerUpdated, UsernameUpdated, PasswordUpdated};
        match (self, message) {
            (Self::Signup { server, .. } | Self::Login { server, .. },
             Message::AuthScreen(ServerUpdated(newserver))) =>
                *server = newserver,
            (Self::Signup { username, .. } | Self::Login { username, .. },
             Message::AuthScreen(UsernameUpdated(newuname))) =>
                *username = newuname,
            (Self::Signup { password, .. } | Self::Login { password, .. },
             Message::AuthScreen(PasswordUpdated(newpwd))) =>
                *password = newpwd,
            _ => {},
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, Renderer> {
        match self {
            Self::Signup { server, username, password } =>
                signup_screen(&server, &username, &password)
                    .map(Message::AuthScreen),
            Self::Login { server, username, password } =>
                login_screen(&server, &username, &password)
                    .map(Message::AuthScreen),
            _ => todo!()
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        if let Self::LoggedIn { server, http, .. } = self {
            gateway::connect(server.clone(), http.token().unwrap().to_string())
                .map(Message::GatewayEvent)
        } else {
            Subscription::none()
        }
    }
}
