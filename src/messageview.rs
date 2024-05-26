use std::collections::VecDeque;
use std::error::Error;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use crate::editor::MessageEditor;
use crate::utils::{icon, ErrorWithCauses, Gaps};
use chrono::{Local, TimeDelta};

use iced::font::Weight;
use iced::widget::scrollable::Properties;
use iced::widget::{button, column, container, mouse_area, row, scrollable, text_editor, Row};
use iced::widget::{text, Column, Space};
use iced::{theme, Alignment, Color, Command, Element, Font, Length, Theme};
use iced_aw::floating_element::Anchor;
use iced_aw::FloatingElement;
use quaddlecl::model::message::MessageId as QMessageId;
use quaddlecl::model::user::User;
use quaddlecl::{
    client::http::{self, Http},
    model::{channel::ChannelId, message::Message as QMessage, snowflake::Snowflake},
};

const RESEND: &str = "\u{f0453}";
// const DELETE: &str = "\u{f0a79}"; this will be readded when delete support drops
const EDIT: &str = "\u{f040}";

#[derive(Debug, Clone)]
pub enum HistoryQMsgMessage {
    MouseEnter,
    MouseLeave,
    EditInitiated,
    EditSubmitted,
    EditCancelled,
    EditFailed(Arc<http::Error>),
    EditSucceeded(QMessage),
    SendingFailed(Arc<http::Error>),
    SendingSucceeded(QMessage),
    ResendInitiated,
    Editor(text_editor::Action),
}

#[derive(Debug)]
pub enum HistoryQMsgState {
    Sending,
    SendingFailed(Arc<http::Error>),
    SubmittingEdit(text_editor::Content),
    Display,
    Editing {
        editor: text_editor::Content,
        last_error: Option<Arc<http::Error>>,
    },
}

static HISTORY_QMSG_ID: AtomicU32 = AtomicU32::new(0);

/// Identifies an instance of a HistoryQMessage.
/// This struct is meant to be a clean way to identify [`HistoryQMessage`]s,
/// since unsent messages don't have proper IDs assigned to them yet.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct HistoryQMessageId(u32);

impl HistoryQMessageId {
    pub fn new() -> Self {
        Self(HISTORY_QMSG_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

/// A widget that represents a Quaddle message.
#[derive(Debug)]
pub struct HistoryQMessage {
    id: HistoryQMessageId,
    hovered: bool,
    state: HistoryQMsgState,
    msg: QMessage,
}

impl HistoryQMessage {
    pub fn new(msg: QMessage) -> Self {
        Self {
            id: HistoryQMessageId::new(),
            hovered: false,
            state: HistoryQMsgState::Display,
            msg,
        }
    }

    pub fn sending(author: User, channel: ChannelId, content: String) -> Self {
        Self {
            id: HistoryQMessageId::new(),
            hovered: false,
            state: HistoryQMsgState::Sending,
            msg: {
                let mut m = QMessage::default();
                m.author = author;
                m.channel = channel;
                m.content = content;
                m
            },
        }
    }

    pub fn id(&self) -> HistoryQMessageId {
        self.id
    }

    /// Returns a command that sends this message.
    pub fn send(&self, http: Arc<Http>) -> Command<(HistoryQMessageId, HistoryQMsgMessage)> {
        use HistoryQMsgMessage as Message;

        let id = self.id;
        let cid = self.msg.channel;
        let content = self.msg.content.clone();
        Command::perform(
            async move { http.create_message(cid, &content).await },
            move |res| match res {
                Ok(msg) => (id, Message::SendingSucceeded(msg)),
                Err(e) => (id, Message::SendingFailed(Arc::new(e))),
            },
        )
    }

    pub fn update(
        &mut self,
        msg: HistoryQMsgMessage,
        http: &Arc<Http>,
    ) -> Command<(HistoryQMessageId, HistoryQMsgMessage)> {
        use HistoryQMsgMessage as Message;
        use HistoryQMsgState as State;
        match (&mut self.state, msg) {
            (_, Message::MouseEnter) => {
                self.hovered = true;
                Command::none()
            }
            (_, Message::MouseLeave) => {
                self.hovered = false;
                Command::none()
            }
            (s @ State::Display, Message::EditInitiated) => {
                *s = State::Editing {
                    editor: text_editor::Content::with_text(&self.msg.content),
                    last_error: None,
                };
                Command::none()
            }
            (s @ State::Editing { .. }, Message::EditSubmitted) => {
                let State::Editing { editor, .. } = std::mem::replace(s, State::Sending) else {
                    unreachable!()
                };
                let content = editor.text();
                *s = State::SubmittingEdit(editor);
                let cid = self.msg.channel;
                let mid = self.msg.id;
                let hqmid = self.id;
                let http = Arc::clone(http);
                Command::perform(
                    async move { http.edit_message(cid, mid, &content).await },
                    move |result| match result {
                        Ok(msg) => (hqmid, Message::EditSucceeded(msg)),
                        Err(e) => (hqmid, Message::EditFailed(Arc::new(e))),
                    },
                )
            }
            (s @ State::Editing { .. }, Message::EditCancelled) => {
                *s = State::Display;
                Command::none()
            }
            (s @ State::SubmittingEdit(_), Message::EditFailed(err)) => {
                let State::SubmittingEdit(editor) = std::mem::replace(s, State::Sending) else {
                    unreachable!()
                };
                *s = State::Editing {
                    editor,
                    last_error: Some(err),
                };
                Command::none()
            }
            (s @ State::SubmittingEdit(_), Message::EditSucceeded(msg)) => {
                *s = State::Display;
                self.msg = msg;
                Command::none()
            }
            (s @ State::Sending, Message::SendingFailed(err)) => {
                *s = State::SendingFailed(err);
                Command::none()
            }
            (s @ State::Sending, Message::SendingSucceeded(msg)) => {
                *s = State::Display;
                self.msg = msg;
                Command::none()
            }
            (State::Editing { editor, .. }, Message::Editor(action)) => {
                editor.perform(action);
                Command::none()
            }
            _ => Command::none(),
        }
    }

    fn icon_button(s: &str, message: HistoryQMsgMessage) -> Element<'_, HistoryQMsgMessage> {
        button(icon(s)).on_press(message).into()
    }

    fn action_buttons(&self) -> Vec<Element<'_, HistoryQMsgMessage>> {
        use HistoryQMsgMessage as Message;
        use HistoryQMsgState as State;
        match &self.state {
            State::Sending => vec![],
            State::SendingFailed(_) => vec![Self::icon_button(RESEND, Message::ResendInitiated)],
            State::SubmittingEdit(_) => vec![],
            State::Display => vec![Self::icon_button(EDIT, Message::EditInitiated)],
            State::Editing { .. } => vec![],
        }
    }

    pub fn view(&self, theme: &Theme, extended_info: bool) -> Element<'_, HistoryQMsgMessage> {
        use HistoryQMsgMessage as Message;
        use HistoryQMsgState as State;

        fn content_plain<'a>(content: &'a str, a: f32, theme: &Theme) -> Element<'a, Message> {
            text(content)
                .style(theme::Text::Color(Color {
                    a,
                    ..theme.extended_palette().background.weak.text
                }))
                .shaping(text::Shaping::Advanced)
                .width(Length::Fill)
                .into()
        }

        fn editor_view<'a>(
            content: &'a text_editor::Content,
            enabled: bool,
        ) -> Column<'a, Message> {
            column([
                {
                    let editor = MessageEditor::new(&content);
                    if enabled {
                        editor.on_action(Message::Editor).into()
                    } else {
                        editor.into()
                    }
                },
                row([
                    button("save")
                        .style(theme::Button::Text)
                        .on_press_maybe(Some(Message::EditSubmitted).filter(|_| enabled))
                        .into(),
                    "/".into(),
                    button("cancel")
                        .style(theme::Button::Text)
                        .on_press_maybe(Some(Message::EditCancelled).filter(|_| enabled))
                        .into(),
                ])
                .spacing(4)
                .into(),
            ])
            .spacing(5)
        }

        fn error_msg<'a, E: 'a + Error>(e: E) -> Element<'a, Message> {
            row([
                icon(crate::WARNING).size(14).into(),
                text(format!("Failed to send: {err}", err = ErrorWithCauses(e)))
                    .size(14)
                    .into(),
            ])
            .spacing(3)
            .into()
        }

        let content = match &self.state {
            State::Sending => content_plain(&self.msg.content, 0.8, theme),
            State::SendingFailed(err) => {
                column([content_plain(&self.msg.content, 1.0, theme), error_msg(err)])
                    .spacing(5)
                    .into()
            }
            State::SubmittingEdit(ed) => editor_view(ed, false).into(),
            State::Editing { editor, last_error } => editor_view(editor, true)
                .push_maybe(last_error.as_ref().map(error_msg))
                .into(),
            State::Display => content_plain(&self.msg.content, 1.0, theme),
        };

        let date_str = self
            .msg
            .id
            .timestamp()
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M");

        let underlay = if extended_info {
            column([
                Space::with_height(10).into(),
                row([
                    text(&self.msg.author.name)
                        .shaping(text::Shaping::Advanced)
                        .font(crate::DEFAULT_FONT_MEDIUM)
                        .into(),
                    text(date_str)
                        .size(10)
                        .style(iced::theme::Text::Color({
                            theme.extended_palette().background.weak.text
                        }))
                        .into(),
                ])
                .align_items(iced::Alignment::Center)
                .spacing(5)
                .into(),
                content,
            ])
            .spacing(3)
            .width(Length::Fill)
            .into()
        } else {
            container(content).width(Length::Fill).into()
        };

        let action_butns = self.action_buttons();

        let el: Element<'_, _> = if !action_butns.is_empty() {
            let overlay = Row::from_vec(action_butns).align_items(Alignment::Center);

            FloatingElement::new(underlay, overlay)
                .anchor(Anchor::NorthEast)
                .hide(!self.hovered)
                .into()
        } else {
            underlay
        };

        mouse_area(el)
            .on_enter(Message::MouseEnter)
            .on_exit(Message::MouseLeave)
            .into()
    }
}

pub const QMESSAGELIST_ID: &str = "qmessage_list";

pub fn qmessage_list<'a>(
    theme: &Theme,
    messages: impl IntoIterator<Item = &'a HistoryQMessage>,
) -> Element<'a, (usize, HistoryQMsgMessage)> {
    let el = scrollable({
        Column::with_children({
            Gaps::new(messages)
                .enumerate()
                .filter_map(|(i, (lastmsg, curmsg_opt))| {
                    let curmsg = curmsg_opt?;
                    let extended_info = !lastmsg.is_some_and(|lmsg| {
                        lmsg.msg.author.id == curmsg.msg.author.id
                            && (curmsg.msg.id.timestamp() - lmsg.msg.id.timestamp())
                                < TimeDelta::minutes(5)
                    });
                    Some(curmsg.view(theme, extended_info).map(move |msg| (i, msg)))
                })
        })
    })
    .direction({
        iced::widget::scrollable::Direction::Vertical({
            Properties::new().alignment(scrollable::Alignment::End)
        })
    })
    .id(scrollable::Id::new(QMESSAGELIST_ID));

    container(el).padding(20).height(Length::Fill).into()
}

pub fn retrieve_history<Message>(
    http: Arc<Http>,
    channel_id: ChannelId,
    before: Option<QMessageId>,
    on_success: impl FnOnce(ChannelId, Vec<QMessage>) -> Message + Send + Sync + 'static,
    on_error: impl FnOnce(http::Error) -> Message + Send + Sync + 'static,
) -> Command<Message> {
    Command::perform(
        async move { http.message_history(channel_id, before).await },
        move |res| match res {
            Ok(msgs) => on_success(channel_id, msgs),
            Err(err) => on_error(err),
        },
    )
}
