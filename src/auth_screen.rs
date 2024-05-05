use std::error::Error;
use std::fmt::Debug;

use iced::theme::Button;
use iced::widget::{button, container, text, text_input, Column};
use iced::{Command, Element, Length, Theme};
use quaddlecl::client::http::{self, Http};
use url::Url;

use crate::USER_AGENT;

#[derive(Debug)]
enum ActionState {
    Idle,
    InProgress,
    Error(Box<dyn Error + Send + Sync>),
    Success,
}

#[derive(Debug)]
enum AuthScreenState {
    Login(ActionState),
    Signup(ActionState),
}

pub struct AuthScreen {
    state: AuthScreenState,
    server: String,
    username: String,
    password: String,
}

#[derive(Debug, Clone)]
pub enum UiMessage {
    ServerUpdated(String),
    UsernameUpdated(String),
    PasswordUpdated(String),
    SignupInitiated,
    LoginInitiated,
    SignupSucceeded,
    SwitchToLogin,
    SwitchToSignup,
}

#[derive(Debug)]
pub enum IoMessage {
    SignupSucceeded,
    SignupFailed(Box<dyn Error + Send + Sync>),
    LoginSucceeded(Http, Url),
    LoginFailed(Box<dyn Error + Send + Sync>),
}

#[derive(Debug)]
pub enum Message {
    Ui(UiMessage),
    Io(IoMessage),
}

pub fn validate_credentials(server: &str, username: &str, password: &str) -> bool {
    (1..1024).contains(&username.len())
        && (1..1024).contains(&password.len())
        && Url::parse(server).is_ok()
}

impl Default for AuthScreen {
    fn default() -> Self {
        Self {
            state: AuthScreenState::Login(ActionState::Idle),
            server: String::new(),
            username: String::new(),
            password: String::new(),
        }
    }
}

impl AuthScreen {
    pub fn update(&mut self, msg: Message) -> Command<Message> {
        use Message::{Io, Ui};
        match msg {
            Ui(UiMessage::ServerUpdated(srv)) => self.server = srv,
            Ui(UiMessage::UsernameUpdated(uname)) => self.username = uname,
            Ui(UiMessage::PasswordUpdated(pwd)) => self.password = pwd,
            Ui(UiMessage::SignupInitiated) => {
                self.state = AuthScreenState::Signup(ActionState::InProgress);
                let server: String = self.server.to_string();
                let username: String = self.username.to_string();
                let password: String = self.password.to_string();
                return Command::perform(
                    async move {
                        Http::new(Url::parse(&server).unwrap(), USER_AGENT.to_string())?
                            .signup(&username, &password)
                            .await
                    },
                    |res: Result<_, http::Error>| match res {
                        Ok(_) => Io(IoMessage::SignupSucceeded),
                        Err(e) => Io(IoMessage::SignupFailed(Box::new(e))),
                    },
                );
            }
            Io(IoMessage::SignupSucceeded) => {
                self.state = AuthScreenState::Signup(ActionState::Success)
            }
            Io(IoMessage::SignupFailed(err)) => {
                self.state = AuthScreenState::Signup(ActionState::Error(err))
            }
            Ui(UiMessage::LoginInitiated) => {
                self.state = AuthScreenState::Login(ActionState::InProgress);
                let server: Url = Url::parse(&self.server).unwrap();
                let username: String = self.username.to_string();
                let password: String = self.password.to_string();
                return Command::perform(
                    async move {
                        let mut http = Http::new(server.clone(), USER_AGENT.to_string())?;
                        http.login(&username, &password).await?;
                        Ok((http, server))
                    },
                    |res: Result<_, http::Error>| match res {
                        Ok((http, server)) => Io(IoMessage::LoginSucceeded(http, server)),
                        Err(e) => Io(IoMessage::LoginFailed(Box::new(e))),
                    },
                );
            }
            Io(IoMessage::LoginFailed(err)) => {
                self.state = AuthScreenState::Login(ActionState::Error(err))
            }
            Ui(UiMessage::SwitchToLogin) => self.state = AuthScreenState::Login(ActionState::Idle),
            Ui(UiMessage::SwitchToSignup) => {
                self.state = AuthScreenState::Signup(ActionState::Idle)
            }
            _ => {}
        }

        Command::none()
    }

    pub fn view<'a>(&self, theme: &Theme) -> Element<'a, Message> {
        let AuthScreen {
            server,
            username,
            password,
            state,
            ..
        } = self;
        let submit_msg = match state {
            AuthScreenState::Login(_) => UiMessage::LoginInitiated,
            AuthScreenState::Signup(_) => UiMessage::SignupInitiated,
        };
        let el: Element<'a, UiMessage> = container(
            Column::new()
                .push_maybe({
                    match state {
                        AuthScreenState::Login(ActionState::Error(err))
                        | AuthScreenState::Signup(ActionState::Error(err)) => {
                            Some(text(err).style(theme.palette().danger))
                        }
                        AuthScreenState::Signup(ActionState::Success) => Some(
                            text("Account successfully created").style(theme.palette().success),
                        ),
                        _ => None,
                    }
                })
                .push(
                    text_input("Server", server)
                        .on_input(UiMessage::ServerUpdated)
                        .on_submit(submit_msg.clone()),
                )
                .push(
                    text_input("Username", username)
                        .on_input(UiMessage::UsernameUpdated)
                        .on_submit(submit_msg.clone()),
                )
                .push(
                    text_input("Password", password)
                        .secure(true)
                        .on_input(UiMessage::PasswordUpdated)
                        .on_submit(submit_msg.clone()),
                )
                .push(match state {
                    AuthScreenState::Login(s) => {
                        button(container("Log in").center_x().width(Length::Fill))
                            .width(Length::Fill)
                            .on_press_maybe({
                                Some(UiMessage::LoginInitiated)
                                    .filter(|_| !matches!(s, ActionState::InProgress))
                                    .filter(|_| validate_credentials(server, username, password))
                            })
                    }
                    AuthScreenState::Signup(s) => {
                        button(container("Sign up").center_x().width(Length::Fill))
                            .width(Length::Fill)
                            .on_press_maybe({
                                Some(UiMessage::SignupInitiated)
                                    .filter(|_| !matches!(s, ActionState::InProgress))
                                    .filter(|_| validate_credentials(server, username, password))
                            })
                    }
                })
                .push(match state {
                    AuthScreenState::Login(s) => {
                        button(container("Sign up").center_x().width(Length::Fill))
                            .on_press_maybe({
                                Some(UiMessage::SwitchToSignup)
                                    .filter(|_| !matches!(s, ActionState::InProgress))
                            })
                            .style(Button::Secondary)
                    }
                    AuthScreenState::Signup(s) => {
                        button(container("Back").center_x().width(Length::Fill))
                            .on_press_maybe({
                                Some(UiMessage::SwitchToLogin)
                                    .filter(|_| !matches!(s, ActionState::InProgress))
                            })
                            .style(Button::Secondary)
                    }
                })
                .spacing(10)
                .width(200),
        )
        .center_x()
        .center_y()
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
        el.map(Message::Ui)
    }
}
