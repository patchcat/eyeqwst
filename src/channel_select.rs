use std::{mem, sync::Arc};

use iced::{
    font::Weight,
    theme,
    widget::{button, container, row, rule, scrollable, text, text_input, tooltip, Column, Rule},
    Command, Element, Font, Length,
};
use iced::{Alignment, Border, Theme};
use iced_aw::native::DropDown;
use quaddlecl::client::{
    gateway::ClientGatewayMessage,
    http::{self, Http},
};
use quaddlecl::model::channel::ChannelId;
use quaddlecl::model::message::Message as QMessage;

use crate::{config::Channel, toggle_button::pressed_button_style, utils::icon};
use crate::{gateway::Connection, utils::TextInputExt};

pub enum ChannelListMessage {
    SelectChannel(usize),
}

pub struct ChannelList<'a, Message, It> {
    selected_channel: usize,
    on_selection: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    channels: It,
    width: Length,
    height: Length,
}

impl<'a, Message, It> ChannelList<'a, Message, It>
where
    It: IntoIterator<Item = &'a Channel>,
{
    pub fn new(channels: It, selected_channel: usize) -> Self {
        ChannelList {
            selected_channel,
            on_selection: None,
            channels,
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    pub fn width(self, width: Length) -> Self {
        Self { width, ..self }
    }

    pub fn height(self, height: Length) -> Self {
        Self { height, ..self }
    }

    pub fn on_selection(self, on_selection: impl Fn(usize) -> Message + 'a) -> Self {
        Self {
            on_selection: Some(Box::new(on_selection)),
            ..self
        }
    }
}

impl<'a, Message: 'a, It> From<ChannelList<'a, Message, It>> for Element<'a, Message>
where
    It: IntoIterator<Item = &'a Channel>,
{
    fn from(clist: ChannelList<'a, Message, It>) -> Self {
        let el: Element<'a, usize> = scrollable({
            Column::with_children({
                clist.channels.into_iter().enumerate().map(|(i, channel)| {
                    button({
                        row![
                            Rule::vertical(3.0).style(move |t: &Theme| {
                                use iced::widget::rule::StyleSheet;
                                rule::Appearance {
                                    color: if clist.selected_channel == i {
                                        t.extended_palette().primary.base.color
                                    } else {
                                        t.extended_palette().secondary.base.color
                                    },
                                    width: 3,
                                    fill_mode: rule::FillMode::Full,
                                    ..t.appearance(&theme::Rule::Default)
                                }
                            }),
                            row![
                                icon("\u{f292}").size(20),
                                text(&channel.name).font(Font {
                                    weight: Weight::Medium,
                                    ..crate::DEFAULT_FONT
                                })
                            ]
                            .spacing(5)
                            .padding(5)
                            .align_items(Alignment::Center)
                        ]
                        .height(40)
                        .align_items(Alignment::Center)
                    })
                    .on_press_maybe(Some(i).filter(|_| clist.on_selection.is_some()))
                    .style(theme::Button::Secondary)
                    .padding(0)
                    .width(Length::Fill)
                    .into()
                })
            })
            .spacing(10)
            .width(Length::Fill)
            .height(Length::Shrink)
        })
        .width(clist.width)
        .height(clist.height)
        .into();

        el.map(move |i| match &clist.on_selection {
            Some(select) => select(i),
            None => panic!("disabled clist produced a message"),
        })
    }
}

const ADD_ICON: &str = "\u{f067}";

#[derive(Debug, Clone)]
pub enum ChannelEditMessage {
    Expanded,
    Dismissed,
    NewChannelNameEdited(String),
    NewChannelIdEdited(String),
    ChannelAddRequested,
    ChannelExists(Vec<QMessage>),
    ChannelError(Arc<http::Error>),
}

#[derive(Debug)]
enum ChannelEditStripState {
    Idle {
        last_error: Option<Arc<http::Error>>,
    },
    Confirming(Channel),
}

impl ChannelEditStripState {
    fn is_idle(&self) -> bool {
        matches!(self, Self::Idle { .. })
    }
}

impl Default for ChannelEditStripState {
    fn default() -> Self {
        ChannelEditStripState::Idle { last_error: None }
    }
}

#[derive(Debug, Default)]
pub struct ChannelEditStrip {
    state: ChannelEditStripState,
    expanded: bool,
    new_channel_name: String,
    new_channel_id: Option<ChannelId>,
}

impl ChannelEditStrip {
    pub fn view(&self, theme: &Theme) -> Element<'_, ChannelEditMessage> {
        let add_icon = tooltip(
            button(
                container(icon(ADD_ICON).size(16))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y(),
            )
            .on_press({
                match self.expanded {
                    true => ChannelEditMessage::Dismissed,
                    false => ChannelEditMessage::Expanded,
                }
            })
            .width(40)
            .height(40)
            .style({
                if self.expanded {
                    pressed_button_style(theme::Button::Secondary)
                } else {
                    theme::Button::Secondary
                }
            }),
            "Add channel",
            tooltip::Position::FollowCursor,
        );

        let add_menu = container({
            Column::new()
                .push_maybe({
                    match &self.state {
                        ChannelEditStripState::Idle {
                            last_error: Some(e),
                        } => Some(text(e).style(theme::Text::Color(theme.palette().danger))),
                        _ => None,
                    }
                })
                .push({
                    text_input("Name", &self.new_channel_name).on_input_if(
                        self.state.is_idle(),
                        ChannelEditMessage::NewChannelNameEdited,
                    )
                })
                .push({
                    text_input(
                        "ID",
                        &self.new_channel_id.map_or(String::new(), |s| s.to_string()),
                    )
                    .on_input_if(self.state.is_idle(), ChannelEditMessage::NewChannelIdEdited)
                })
                .push({
                    button("Add channel").on_press_maybe({
                        Some(ChannelEditMessage::ChannelAddRequested)
                            .filter(|_| self.state.is_idle())
                            .filter(|_| self.new_channel_id.is_some())
                            .filter(|_| !self.new_channel_name.is_empty())
                    })
                })
                .spacing(10)
        })
        .style(|t: &Theme| {
            use iced::widget::container::StyleSheet;
            container::Appearance {
                border: Border {
                    color: t.extended_palette().background.base.text,
                    width: 1.0,
                    radius: 3.into(),
                },
                ..t.appearance(&theme::Container::Box)
            }
        })
        .padding(10);
        container(
            DropDown::new(add_icon, add_menu, self.expanded)
                .alignment(iced_aw::drop_down::Alignment::Bottom)
                .on_dismiss(ChannelEditMessage::Dismissed)
                .width(200),
        )
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .into()
    }

    pub fn update(
        &mut self,
        msg: ChannelEditMessage,
        channels: &mut Vec<Channel>,
        selected_channel: &mut usize,
        messages: &mut Vec<QMessage>,
        gateway_conn: &mut Connection,
        http: Arc<Http>,
    ) -> Command<ChannelEditMessage> {
        use ChannelEditStripState::{Confirming, Idle};
        match (&mut self.state, msg) {
            (_, ChannelEditMessage::Expanded) => self.expanded = true,
            (_, ChannelEditMessage::Dismissed) => self.expanded = false,
            (Idle { .. }, ChannelEditMessage::NewChannelNameEdited(s)) => self.new_channel_name = s,
            (Idle { .. }, ChannelEditMessage::NewChannelIdEdited(id)) => {
                if id.is_empty() {
                    self.new_channel_id = None;
                } else if let Ok(num) = id.parse::<ChannelId>() {
                    self.new_channel_id = Some(num);
                }
            }
            (Idle { .. }, ChannelEditMessage::ChannelAddRequested) => {
                let Some(channel_id) = self.new_channel_id.take() else {
                    return Command::none();
                };

                self.state = ChannelEditStripState::Confirming(Channel {
                    id: channel_id,
                    name: mem::take(&mut self.new_channel_name),
                });

                return Command::perform(
                    async move { http.message_history(channel_id, None).await },
                    |res| {
                        log::debug!("{res:?}");
                        match res {
                            Ok(msgs) => ChannelEditMessage::ChannelExists(msgs),
                            Err(e) => ChannelEditMessage::ChannelError(Arc::new(e)),
                        }
                    },
                );
            }
            (s @ Confirming(_), ChannelEditMessage::ChannelExists(mut msgs)) => {
                let Confirming(chan) =
                    mem::replace(s, ChannelEditStripState::Idle { last_error: None })
                else {
                    unreachable!()
                };
                gateway_conn.send(ClientGatewayMessage::Subscribe {
                    channel_id: chan.id,
                });
                channels.push(chan);
                self.expanded = false;
                *selected_channel = channels.len() - 1;
                msgs.reverse();
                *messages = msgs;
            }
            (Confirming(_), ChannelEditMessage::ChannelError(err)) => {
                self.state = ChannelEditStripState::Idle {
                    last_error: Some(err),
                };
            }
            _ => {}
        }
        Command::none()
    }
}
