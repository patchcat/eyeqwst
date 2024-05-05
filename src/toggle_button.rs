use iced::{widget, Theme};

struct PressedButtonStyle {
    inner: iced::theme::Button,
}

impl widget::button::StyleSheet for PressedButtonStyle {
    type Style = Theme;

    fn hovered(&self, style: &Self::Style) -> widget::button::Appearance {
        style.pressed(&self.inner)
    }

    fn pressed(&self, style: &Self::Style) -> widget::button::Appearance {
        style.pressed(&self.inner)
    }

    fn active(&self, style: &Self::Style) -> widget::button::Appearance {
        style.pressed(&self.inner)
    }
}

pub fn pressed_button_style(style: iced::theme::Button) -> iced::theme::Button {
    iced::theme::Button::custom(PressedButtonStyle { inner: style })
}
