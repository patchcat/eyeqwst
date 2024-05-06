use iced::time::Duration;
use std::fmt;

use std::error::Error;

use iced::advanced::widget::text::StyleSheet as TextStyleSheet;
use iced::widget::TextInput;
use iced::{advanced::widget::Text, widget::text, Font};

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(d: Duration) {
    tokio::time::sleep(d).await;
}

#[cfg(target_arch = "wasm32")]
pub async fn sleep(d: Duration) {
    let mut cb = |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve,
                d.as_millis().try_into().unwrap_or(i32::MAX),
            )
            .unwrap();
    };

    let fut = js_sys::Promise::new(&mut cb);
    wasm_bindgen_futures::JsFuture::from(fut).await.unwrap();
}

pub struct ErrorWithCauses<E>(pub E);

impl<E> fmt::Display for ErrorWithCauses<E>
where
    E: Error,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)?;
        let mut cur: &dyn Error = &self.0;
        while let Some(next) = cur.source() {
            cur = next;
            write!(f, ": {}", cur)?;
        }
        Ok(())
    }
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
