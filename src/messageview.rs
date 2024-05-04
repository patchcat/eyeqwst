use std::sync::Arc;

use iced::{advanced::graphics::futures::{MaybeSend, MaybeSync}, font::Weight, widget::{column, container, row, scrollable, text, Column, Container, Space}, Color, Command, Element, Font, Length};
use quaddlecl::{client::http::{self, Http}, model::{channel::ChannelId, message::Message as QMessage, snowflake::Snowflake}};
use quaddlecl::model::message::MessageId as QMessageId;
use crate::utils::Gaps;

/// A widget that represents a Quaddle message.
pub struct QMessageWidget<'a> {
    msg: &'a QMessage,
    extended_info: bool,
}

impl<'a> QMessageWidget<'a> {
    pub fn new(msg: &'a QMessage) -> Self {
        Self {
            msg,
            extended_info: false,
        }
    }

    pub fn extended_info(self, extended_info: bool) -> Self {
        Self { extended_info, ..self }
    }
}

impl<'a, Message: 'a> From<QMessageWidget<'a>> for Element<'a, Message> {
    fn from(qmw: QMessageWidget) -> Self {
        let content: Element<'a, Message> = text(&qmw.msg.content)
            .shaping(text::Shaping::Advanced)
            .into();
        if qmw.extended_info {
            column![
                Space::with_height(10),
                row![
                    text(&qmw.msg.author.name)
                        .shaping(text::Shaping::Advanced)
                        .font({
                            Font {
                                weight: Weight::Medium,
                                ..crate::DEFAULT_FONT
                            }
                        }),
                    text(qmw.msg.id.timestamp())
                        .size(10)
                        .style(iced::theme::Text::Color(Color::from_rgba8(0xff, 0xff, 0xff, 0.5)))
                ]
                    .spacing(5)
                    .align_items(iced::Alignment::End),
                content
            ]
                .spacing(3)
                .width(Length::Fill)
                .into()
        } else {
            container(content)
                .width(Length::Fill)
                .into()
        }
    }
}

pub fn qmessage_list<'a, Message: 'a>(
    messages: impl IntoIterator<Item = &'a QMessage>
) -> Element<'a, Message> {
    let el = scrollable({
        Column::with_children({
            Gaps::new(messages)
                .filter_map(|(lastmsg, curmsg_opt)| {
                    let curmsg = curmsg_opt?;
                    Some({
                        QMessageWidget::new(curmsg)
                            .extended_info({
                                !lastmsg.is_some_and(|msg| {
                                    msg.author.id == curmsg.author.id
                                })
                            })
                            .into()
                    })
                })
        })
    });

    container(el)
        .padding(20)
        .into()
}

pub fn retrieve_history<Message>(
    http: Arc<Http>,
    channel_id: ChannelId,
    before: Option<QMessageId>,
    on_success: impl FnOnce(Vec<QMessage>) -> Message + Send + Sync + 'static,
    on_error: impl FnOnce(http::Error) -> Message + Send + Sync + 'static
) -> Command<Message> {
    Command::perform(
        async move {
            http.message_history(channel_id, before).await
        },
        move |res| match res {
            Ok(msgs) => on_success(msgs),
            Err(err) => on_error(err),
        }
    )
}
