use std::sync::Arc;

use iced::advanced::text::highlighter::PlainText;
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{text, Widget};
use iced::event::Status;
use iced::keyboard::key::Named;
use iced::keyboard::Key;
use iced::mouse::Button;

use iced::widget::text_editor::{Action, Content, Motion};
use iced::widget::TextEditor;
use iced::{keyboard, widget, Command, Element, Event, Length, Padding, Renderer};
use quaddlecl::client::http::{self, Http};
use quaddlecl::model::channel::ChannelId;
use quaddlecl::model::message::Message as QMessage;

pub fn send_message<Message>(
    http: Arc<Http>,
    editor: &mut Content<Renderer>,
    channel_id: ChannelId,
    on_success: impl FnOnce(QMessage) -> Message + Send + Sync + 'static,
    on_error: impl FnOnce(http::Error) -> Message + Send + Sync + 'static,
) -> Command<Message> {
    let msgtext = editor.text();
    *editor = Content::new();
    Command::perform(
        async move { http.create_message(channel_id, &msgtext).await },
        |res| match res {
            Ok(x) => on_success(x),
            Err(e) => on_error(e),
        },
    )
}

pub struct MessageEditor<'a, Highlighter, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Highlighter: text::Highlighter,
    Theme: widget::text_editor::StyleSheet,
    Renderer: text::Renderer,
{
    text_editor: TextEditor<'a, Highlighter, Message, Theme, Renderer>,
    on_enter: Option<Message>,
    on_action: Option<fn(Action) -> Message>,
    is_disabled: bool,
}

struct State {
    is_focused: bool, // goofy ahh hack
}

impl<'a, Message, Theme, Renderer> MessageEditor<'a, PlainText, Message, Theme, Renderer>
where
    Theme: widget::text_editor::StyleSheet,
    Renderer: text::Renderer,
    Message: Clone,
{
    pub fn new(content: &'a Content<Renderer>) -> Self {
        Self {
            text_editor: TextEditor::new(content),
            on_enter: None,
            on_action: None,
            is_disabled: true,
        }
    }
}

impl<'a, Highlighter, Message, Theme, Renderer>
    MessageEditor<'a, Highlighter, Message, Theme, Renderer>
where
    Highlighter: text::Highlighter,
    Theme: widget::text_editor::StyleSheet,
    Renderer: text::Renderer,
    Message: Clone + 'a,
{
    pub fn on_action(self, f: fn(Action) -> Message) -> Self {
        Self {
            text_editor: self.text_editor.on_action(f),
            on_action: Some(f),
            is_disabled: false,
            ..self
        }
    }

    pub fn on_enter(self, msg: Message) -> Self {
        Self {
            on_enter: Some(msg),
            ..self
        }
    }

    pub fn padding(self, p: impl Into<Padding>) -> Self {
        Self {
            text_editor: self.text_editor.padding(p),
            ..self
        }
    }
}

impl<'a, Highlighter, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for MessageEditor<'a, Highlighter, Message, Theme, Renderer>
where
    Highlighter: text::Highlighter,
    Theme: widget::text_editor::StyleSheet,
    Renderer: text::Renderer,
    Message: Clone,
{
    fn size(&self) -> iced::Size<Length> {
        self.text_editor.size()
    }

    fn layout(
        &self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        self.text_editor
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        self.text_editor.draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        )
    }

    fn size_hint(&self) -> iced::Size<Length> {
        self.text_editor.size_hint()
    }

    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        tree::State::new(State { is_focused: false })
    }

    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        let el = &self.text_editor as &dyn Widget<_, _, _>;
        vec![Tree::new(el)]
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        let widgets = &[&self.text_editor as &dyn Widget<_, _, _>];
        tree.diff_children(widgets)
    }

    fn operate(
        &self,
        state: &mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation<Message>,
    ) {
        self.text_editor.operate(state, layout, renderer, operation)
    }

    fn on_event(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) -> iced::advanced::graphics::core::event::Status {
        let state = tree.state.downcast_mut::<State>();
        if !self.is_disabled {
            match (&self, &event) {
                (
                    Self {
                        is_disabled: false, ..
                    },
                    Event::Mouse(iced::mouse::Event::ButtonPressed(Button::Left)),
                ) => {
                    state.is_focused = cursor.position_in(layout.bounds()).is_some();
                }
                (
                    Self {
                        is_disabled: false, ..
                    },
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        key: Key::Named(Named::Enter),
                        modifiers,
                        ..
                    }),
                ) if modifiers.is_empty() && state.is_focused => {
                    if let Some(on_enter) = self.on_enter.clone() {
                        shell.publish(on_enter);
                        return Status::Captured;
                    }
                }
                (
                    Self {
                        on_action: Some(on_action),
                        ..
                    },
                    Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }),
                ) if matches!(key.as_ref(), Key::Character("a"))
                    && modifiers.command()
                    && state.is_focused =>
                {
                    shell.publish(on_action(Action::Move(Motion::DocumentStart)));
                    shell.publish(on_action(Action::Select(Motion::DocumentEnd)));
                    return Status::Captured;
                }
                _ => {}
            }
        }
        self.text_editor.on_event(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        self.text_editor
            .mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        renderer: &Renderer,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.text_editor
            .overlay(&mut tree.children[0], layout, renderer, translation)
    }
}

impl<'a, Highlighter, Message, Theme, Renderer>
    From<MessageEditor<'a, Highlighter, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Highlighter: text::Highlighter,
    Theme: widget::text_editor::StyleSheet + 'a,
    Renderer: text::Renderer,
    Message: Clone + 'a,
{
    fn from(editor: MessageEditor<'a, Highlighter, Message, Theme, Renderer>) -> Self {
        Self::new(editor)
    }
}
