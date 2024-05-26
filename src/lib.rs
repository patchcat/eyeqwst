use auth_screen::AuthScreen;
use auth_screen::IoMessage as AuthIoMessage;
use auth_screen::Message as AuthMessage;
use config::Config;
use iced::keyboard::{key, on_key_press, Key};
use iced::Font;
use iced::{executor, widget, Application, Command, Element, Renderer, Subscription, Theme};
use main_screen::MainScreen;
use main_screen::MainScreenMessage;

pub mod auth_screen;
pub mod channel_select;
pub mod config;
pub mod editor;
pub mod gateway;
pub mod main_screen;
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
const WARNING: &str = "\u{f071}";

pub enum EyeqwstState {
    Authenticating(AuthScreen),
    LoggedIn(main_screen::MainScreen),
}

pub struct Eyeqwst {
    state: EyeqwstState,
    config: Config,
}

#[derive(Debug)]
pub enum Message {
    AuthScreen(AuthMessage),
    MainScreen(MainScreenMessage),
    AutoSave,
    TabPressed,
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
                *s = EyeqwstState::LoggedIn(MainScreen::new(http, server));
            }
            (EyeqwstState::Authenticating(scr), Message::AuthScreen(msg)) => {
                return scr.update(msg).map(Message::AuthScreen)
            }
            (EyeqwstState::LoggedIn(mscr), Message::MainScreen(msg)) => {
                return mscr.update(msg, &mut self.config).map(Message::MainScreen)
            }
            (_, Message::AutoSave) => self.config.save(),
            (_, Message::TabPressed) => return widget::focus_next(),
            _ => {}
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, Renderer> {
        match &self.state {
            EyeqwstState::Authenticating(scr) => scr.view(&self.theme()).map(Message::AuthScreen),
            EyeqwstState::LoggedIn(scr) => scr
                .view(&self.theme(), &self.config)
                .map(Message::MainScreen),
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch([
            match &self.state {
                EyeqwstState::LoggedIn(scr) => scr.subscription().map(Message::MainScreen),
                _ => Subscription::none(),
            },
            on_key_press(|key, _| match key {
                Key::Named(key::Named::Tab) => Some(Message::TabPressed),
                _ => None,
            }),
            #[cfg(target_arch = "wasm32")]
            iced::time::every(Duration::from_secs(10)).map(|_| Message::AutoSave),
        ])
    }

    fn theme(&self) -> iced::Theme {
        iced::Theme::Light
    }
}
