use std::time::Duration;

use iced::advanced::widget::text::StyleSheet as TextStyleSheet;
use iced::widget::TextInput;
use iced::{advanced::widget::Text, widget::text, Font};

pub async fn sleep(d: Duration) {
    tokio::time::sleep(d).await;
}

pub trait TextInputExt<'a, Message: Clone> {
    fn on_input_if(self, cond: bool, msg: impl Fn(String) -> Message + 'a) -> Self;
}

impl<'a, Message> TextInputExt<'a, Message> for TextInput<'a, Message>
where
    Message: Clone + 'a,
{
    fn on_input_if(self, cond: bool, msg: impl Fn(String) -> Message + 'a) -> Self {
        if cond {
            self.on_input(msg)
        } else {
            self
        }
    }
}

pub fn icon<Theme>(s: &str) -> Text<'_, Theme, iced::Renderer>
where
    Theme: TextStyleSheet,
{
    text(s).font(Font {
        family: iced::font::Family::Name("Symbols Nerd Font"),
        ..Font::DEFAULT
    })
}

/// iterator over the gaps between neighboring elements in an iterator
pub struct Gaps<It>
where
    It: Iterator,
{
    inner: It,
    prev: Option<It::Item>,
}

impl<It> Iterator for Gaps<It>
where
    It: Iterator,
    It::Item: Clone,
{
    type Item = (Option<It::Item>, Option<It::Item>);
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.inner.next();
        if next.is_none() && self.prev.is_none() {
            None
        } else {
            Some((std::mem::replace(&mut self.prev, next.clone()), next))
        }
    }
}

impl<It> Gaps<It>
where
    It: Iterator,
{
    pub fn new<I>(it: I) -> Self
    where
        I: IntoIterator<IntoIter = It>,
    {
        Self {
            inner: it.into_iter(),
            prev: None,
        }
    }
}
