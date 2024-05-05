use iced::{
    theme,
    widget::{button, scrollable, text, Column},
    Element, Length,
};

use crate::{config::Channel, toggle_button::pressed_button_style};

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
                    button(text(format!("#{name}", name = channel.name)))
                        .on_press_maybe(Some(i).filter(|_| clist.on_selection.is_some()))
                        .style({
                            if i == clist.selected_channel {
                                pressed_button_style(theme::Button::Secondary)
                            } else {
                                theme::Button::Secondary
                            }
                        })
                        .padding(5)
                        .width(Length::Fill)
                        .into()
                })
            })
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
