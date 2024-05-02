use iced::widget::{button, column, container, text, text_input};
use iced::{Element, Length};

#[derive(Debug, Clone)]
pub enum Message {
    ServerUpdated(String),
    UsernameUpdated(String),
    PasswordUpdated(String),
    SignupInitiated,
    LoginInitiated
}

pub fn signup_screen<'a>(server: &'a str, username: &'a str, password: &'a str)
                     -> Element<'a, Message> {
    container(column![
        text_input("Server", server)
            .on_input(Message::ServerUpdated)
            .on_submit(Message::SignupInitiated),

        text_input("Username", username)
            .on_input(Message::UsernameUpdated)
            .on_submit(Message::SignupInitiated),

        text_input("Password", password)
            .on_input(Message::PasswordUpdated)
            .on_submit(Message::SignupInitiated),

        button("Sign up"),
    ])
        .center_x()
        .center_y()
        .into()
}
pub fn login_screen<'a>(server: &'a str, username: &'a str, password: &'a str)
                        -> Element<'a, Message> {
    container(
        column![
            text_input("Server", server)
                .on_input(Message::ServerUpdated)
                .on_submit(Message::SignupInitiated),

            text_input("Username", username)
                .on_input(Message::UsernameUpdated)
                .on_submit(Message::SignupInitiated),

            text_input("Password", password)
                .on_input(Message::PasswordUpdated)
                .on_submit(Message::SignupInitiated),

            button(container("Log in").center_x().width(Length::Fill))
                .width(Length::Fill),
        ]
            .spacing(20)
            .width(200)
    )
        .center_x()
        .center_y()
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
