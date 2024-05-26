use std::fmt::Display;
use std::sync::Arc;

use iced::theme::palette;
use iced::widget::scrollable::{self, snap_to, RelativeOffset};
use iced::widget::{self, column, container, row, text, text_editor};
use iced::{theme, Background, Color, Command, Element, Length, Renderer, Theme};
use quaddlecl::client::gateway::{ClientGatewayMessage, GatewayEvent};
use quaddlecl::client::{self, http};
use quaddlecl::model::message::Message as QMessage;
use quaddlecl::model::user::User;
use quaddlecl::{client::http::Http, model::channel::ChannelId};
use url::Url;

use crate::channel_select::ChannelEditStrip;
use crate::channel_select::{ChannelEditMessage, ChannelList};
use crate::config::{Channel, Config};
use crate::editor::MessageEditor;
use crate::gateway::{self, Connection, GatewayMessage};
use crate::messageview::{
    qmessage_list, retrieve_history, HistoryQMessage, HistoryQMessageId, HistoryQMsgMessage,
    QMESSAGELIST_ID,
};
use crate::utils::{icon, ErrorWithCauses};
use crate::{CONNECTING, DEFAULT_FONT_MEDIUM, DISCONNECTED};

const CONNECTING_SIZE: u16 = 16;
const CONNECTING_ICON_SIZE: u16 = 17;

#[derive(Debug)]
pub enum GatewayState {
    Disconnected {
        error: Option<client::gateway::Error>,
    },
    Connected {
        user: User,
        conn: Connection,
    },
}

impl GatewayState {
    pub fn user(&self) -> Option<&User> {
        match self {
            GatewayState::Connected { user, .. } => Some(user),
            GatewayState::Disconnected { .. } => None,
        }
    }
}

#[derive(Debug)]
pub struct MainScreen {
    server: Url,
    http: Arc<Http>,
    selected_channel: usize,
    gateway_state: GatewayState,
    channel_edit_strip: ChannelEditStrip,
    // messages in the current channel
    messages: Vec<HistoryQMessage>,
    editor: text_editor::Content,
}

#[derive(Debug, Clone)]
pub enum EditorMessage {
    Action(text_editor::Action),
    SendInitiated,
}

#[derive(Debug)]
pub enum MainScreenMessage {
    HistoryRetrieved(ChannelId, Vec<QMessage>),
    HistoryRetrievalError(http::Error),
    HistoryMessageAction(usize, HistoryQMsgMessage),
    HistoryMessageEvent(HistoryQMessageId, HistoryQMsgMessage),
    ChannelSelected(usize),
    Editor(EditorMessage),
    ChannelEditStrip(ChannelEditMessage),
    SentSuccessfully,
    SendError(http::Error),
    Gateway(GatewayMessage),
}

fn connecting_indicator<'a, Message: 'a, T: Display, F>(
    ic: &'a str,
    message: T,
    color: F,
) -> Element<'a, Message>
where
    F: for<'b> Fn(&'b Theme) -> palette::Pair + 'static,
{
    container(
        row![
            icon(ic).size(CONNECTING_ICON_SIZE),
            text(message)
                .font(DEFAULT_FONT_MEDIUM)
                .size(CONNECTING_SIZE)
        ]
        .spacing(5)
        .padding(10),
    )
    .style(move |t: &Theme| {
        use container::StyleSheet;
        let pair = color(t);
        container::Appearance {
            text_color: Some(pair.text),
            background: Some(Background::Color(pair.color)),
            ..t.appearance(&theme::Container::Box)
        }
    })
    .width(Length::Fill)
    .center_x()
    .into()
}

impl MainScreen {
    pub fn new(http: Http, server: Url) -> Self {
        Self {
            server,
            http: Arc::new(http),
            selected_channel: 0,
            gateway_state: GatewayState::Disconnected { error: None },
            channel_edit_strip: ChannelEditStrip::default(),
            messages: Vec::new(),
            editor: text_editor::Content::new(),
        }
    }

    pub fn update(
        &mut self,
        message: MainScreenMessage,
        config: &mut Config,
    ) -> Command<MainScreenMessage> {
        log::debug!("main screen message: {message:?}");
        match message {
            MainScreenMessage::ChannelSelected(new_selected)
                if new_selected != self.selected_channel =>
            {
                if self.selected_channel(config).is_none() {
                    return Command::none();
                };

                self.selected_channel = new_selected;
                self.messages = Vec::new();
                self.refresh_messages(config)
            }
            MainScreenMessage::HistoryMessageAction(idx, msg) => self
                .messages
                .get_mut(idx)
                .map(|qmsg| qmsg.update(msg, &self.http))
                .unwrap_or_else(|| Command::none())
                .map(|(id, msg)| MainScreenMessage::HistoryMessageEvent(id, msg)),
            MainScreenMessage::HistoryMessageEvent(id, msg) => self
                .messages
                .iter_mut()
                .find(|qmsg| qmsg.id() == id)
                .map(|qmsg| qmsg.update(msg, &self.http))
                .unwrap_or_else(|| Command::none())
                .map(|(id, msg)| MainScreenMessage::HistoryMessageEvent(id, msg)),
            MainScreenMessage::Editor(EditorMessage::SendInitiated) => {
                let Some(channel) = self.selected_channel(config) else {
                    return Command::none();
                };

                let Some(user) = self.gateway_state.user().cloned() else {
                    return Command::none();
                };

                let msg = HistoryQMessage::sending(user, channel.id, self.editor.text());
                let send_message_cmd = msg
                    .send(Arc::clone(&self.http))
                    .map(|(id, msg)| MainScreenMessage::HistoryMessageEvent(id, msg));
                self.messages.push(msg);
                self.editor = text_editor::Content::new();

                Command::batch([
                    send_message_cmd,
                    snap_to(scrollable::Id::new(QMESSAGELIST_ID), RelativeOffset::START),
                ])
            }
            MainScreenMessage::Editor(EditorMessage::Action(action)) => {
                self.editor.perform(action);
                Command::none()
            }
            MainScreenMessage::ChannelEditStrip(msg) => {
                let GatewayState::Connected { user, conn } = &mut self.gateway_state else {
                    return Command::none();
                };

                let channels = &mut config
                    .get_account_config_mut(&self.server, user.id)
                    .channels;

                self.channel_edit_strip
                    .update(
                        msg,
                        channels,
                        &mut self.selected_channel,
                        &mut self.messages,
                        conn,
                        Arc::clone(&self.http),
                    )
                    .map(MainScreenMessage::ChannelEditStrip)
            }
            MainScreenMessage::HistoryRetrieved(channel_id, mut new_msgs) => {
                if !self
                    .selected_channel(config)
                    .is_some_and(|c| c.id == channel_id)
                {
                    return Command::none();
                }

                new_msgs.reverse();
                self.messages = new_msgs.into_iter().map(HistoryQMessage::new).collect();
                Command::none()
            }
            MainScreenMessage::Gateway(msg) => self.on_gateway_message(msg, config),
            // TODO: implement more messages
            _ => Command::none(),
        }
    }

    fn on_gateway_event(
        &mut self,
        event: GatewayEvent,
        config: &Config,
    ) -> Command<MainScreenMessage> {
        match event {
            GatewayEvent::MessageCreate { message } => {
                let is_relevant = self
                    .selected_channel(config)
                    .is_some_and(|c| c.id == message.channel)
                    && self
                        .gateway_state
                        .user()
                        .is_some_and(|u| u.id != message.author.id);
                if is_relevant {
                    self.messages.push(HistoryQMessage::new(message));
                }

                Command::none()
            }
            GatewayEvent::Error { reason } => {
                log::warn!("gateway error: {reason:?}");
                Command::none()
            }
            _ => Command::none(),
        }
    }

    pub fn on_gateway_message(
        &mut self,
        message: GatewayMessage,
        config: &Config,
    ) -> Command<MainScreenMessage> {
        match message {
            GatewayMessage::Connected { user, mut conn, .. } => {
                self.gateway_state = GatewayState::Connected {
                    user,
                    conn: conn.clone(),
                };
                for channel in self.channels(config) {
                    log::debug!("subscribing to {channel:?}");
                    conn.send(ClientGatewayMessage::Subscribe {
                        channel_id: channel.id,
                    });
                }
                self.refresh_messages(config)
            }
            GatewayMessage::DialError(error) => {
                self.gateway_state = GatewayState::Disconnected { error: Some(error) };
                Command::none()
            }
            GatewayMessage::Disconnected => {
                self.gateway_state = GatewayState::Disconnected { error: None };
                Command::none()
            }
            GatewayMessage::ReceiveError(err) => {
                log::warn!("gateway receive error: {err}", err = ErrorWithCauses(err));
                Command::none()
            }
            GatewayMessage::Event(ev) => self.on_gateway_event(ev, config),
        }
    }

    fn channel_at<'a>(&self, idx: usize, config: &'a Config) -> Option<&'a Channel> {
        config
            .get_account_config(&self.server, self.gateway_state.user()?.id)?
            .channels
            .get(idx)
    }

    fn channels<'a>(&self, config: &'a Config) -> impl Iterator<Item = &'a Channel> {
        let Some(user) = self.gateway_state.user() else {
            return None.into_iter().flatten();
        };
        config
            .get_account_config(&self.server, user.id)
            .map(|account| account.channels.iter())
            .into_iter()
            .flatten()
    }

    fn selected_channel<'a>(&self, config: &'a Config) -> Option<&'a Channel> {
        self.channel_at(self.selected_channel, config)
    }

    fn refresh_messages(&self, config: &Config) -> Command<MainScreenMessage> {
        match self.selected_channel(config) {
            Some(channel) => retrieve_history(
                Arc::clone(&self.http),
                channel.id,
                None,
                MainScreenMessage::HistoryRetrieved,
                MainScreenMessage::HistoryRetrievalError,
            ),
            None => Command::none(),
        }
    }

    pub fn view<'a, 'b>(
        &'a self,
        theme: &'b Theme,
        config: &'b Config,
    ) -> Element<'a, MainScreenMessage, Theme, Renderer> {
        let el = row([
            container({
                column([
                    self.channel_edit_strip
                        .view(theme)
                        .map(MainScreenMessage::ChannelEditStrip),
                    ChannelList::new(self.channels(config), self.selected_channel)
                        .on_selection(MainScreenMessage::ChannelSelected)
                        .into(),
                ])
                .width(Length::Fixed(200.0))
                .height(Length::Fill)
                .spacing(20)
            })
            .padding(10)
            .style(|t: &Theme| {
                use iced::widget::container::StyleSheet;
                let color = match t.extended_palette().is_dark {
                    true => Color::from_rgba8(255, 255, 255, 0.05),
                    false => Color::from_rgba8(0, 0, 0, 0.05),
                };
                widget::container::Appearance {
                    background: Some(iced::Background::Color(color)),
                    ..t.appearance(&theme::Container::Transparent)
                }
            })
            .into(),
            column([
                qmessage_list(theme, &self.messages)
                    .map(|(idx, a)| MainScreenMessage::HistoryMessageAction(idx, a)),
                Element::from({
                    container({
                        MessageEditor::new(&self.editor)
                            .on_action(EditorMessage::Action)
                            .on_enter(EditorMessage::SendInitiated)
                            .padding(10)
                    })
                    .padding(10)
                })
                .map(MainScreenMessage::Editor),
            ])
            .into(),
        ])
        .width(Length::Fill)
        .height(Length::Fill);

        match &self.gateway_state {
            GatewayState::Connected { .. } => el.into(),
            GatewayState::Disconnected { error } => {
                let row = match error {
                    Some(err) => connecting_indicator(DISCONNECTED, ErrorWithCauses(err), |t| {
                        t.extended_palette().danger.base
                    }),
                    None => connecting_indicator(CONNECTING, "Connecting...", |t| {
                        t.extended_palette().background.strong
                    }),
                };
                column![row, el]
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .into()
            }
        }
    }

    pub fn subscription(&self) -> iced::Subscription<MainScreenMessage> {
        gateway::connect(self.server.clone(), self.http.token().unwrap().to_string())
            .map(MainScreenMessage::Gateway)
    }
}
