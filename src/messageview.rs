use std::sync::Arc;

use crate::utils::Gaps;
use chrono::{Local, TimeDelta};

use iced::font::Weight;
use iced::widget::scrollable::Properties;
use iced::widget::{column, container, row, scrollable};
use iced::widget::{text, Column, Space};
use iced::{Color, Command, Element, Font, Length};
use quaddlecl::model::message::MessageId as QMessageId;
use quaddlecl::{
    client::http::{self, Http},
    model::{channel::ChannelId, message::Message as QMessage, snowflake::Snowflake},
};

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
        Self {
            extended_info,
            ..self
        }
    }
}

impl<'a, Message: 'a> From<QMessageWidget<'a>> for Element<'a, Message> {
    fn from(qmw: QMessageWidget) -> Self {
        let content: Element<'a, Message> = text(&qmw.msg.content)
            .shaping(text::Shaping::Advanced)
            .into();
        let date_str = qmw
            .msg
            .id
            .timestamp()
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M");
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
                    text(date_str)
                        .size(10)
                        .style(iced::theme::Text::Color(Color::from_rgba8(
                            0xff, 0xff, 0xff, 0.5
                        )))
                ]
                .align_items(iced::Alignment::Center)
                .spacing(5),
                content
            ]
            .spacing(3)
            .width(Length::Fill)
            .into()
        } else {
            container(content).width(Length::Fill).into()
        }
    }
}

pub const QMESSAGELIST_ID: &str = "qmessage_list";

pub fn qmessage_list<'a, Message: 'a>(
    messages: impl IntoIterator<Item = &'a QMessage>,
) -> Element<'a, Message> {
    let el = scrollable({
        Column::with_children({
            Gaps::new(messages).filter_map(|(lastmsg, curmsg_opt)| {
                let curmsg = curmsg_opt?;
                Some({
                    QMessageWidget::new(curmsg)
                        .extended_info({
                            !lastmsg.is_some_and(|msg| {
                                msg.author.id == curmsg.author.id
                                    && (curmsg.id.timestamp() - msg.id.timestamp())
                                        < TimeDelta::minutes(5)
                            })
                        })
                        .into()
                })
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
    on_success: impl FnOnce(Vec<QMessage>) -> Message + Send + Sync + 'static,
    on_error: impl FnOnce(http::Error) -> Message + Send + Sync + 'static,
) -> Command<Message> {
    Command::perform(
        async move { http.message_history(channel_id, before).await },
        move |res| match res {
            Ok(msgs) => on_success(msgs),
            Err(err) => on_error(err),
        },
    )
}
