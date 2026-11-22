//! A transparent wrapper that reports a plain-text copy of its content to
//! widget operations. iced's `rich_text` does not take part in operations,
//! so without this, lesson paragraphs would be invisible to the automation
//! probe (`state` texts). Layout, drawing and input are untouched.

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{tree, Operation, Tree, Widget};
use iced::advanced::{mouse, overlay, Clipboard, Shell};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

pub struct Reported<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    text: String,
    options: Option<Vec<String>>,
}

/// What a wrapped drop-down reports (via `Operation::custom`): the shown
/// text and its options, in menu order.
#[derive(Debug, Clone)]
pub struct ChoiceInfo {
    pub shown: String,
    pub options: Vec<String>,
}

/// Wrap `content`, reporting `text` (markdown markers removed) to operations.
pub fn reported<'a, Message: 'a, Theme: 'a, Renderer: renderer::Renderer + 'a>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    text: impl AsRef<str>,
) -> Element<'a, Message, Theme, Renderer> {
    Element::new(Reported {
        content: content.into(),
        text: text.as_ref().replace("**", "").replace('*', ""),
        options: None,
    })
}

/// Wrap a `pick_list`: reports the shown text and the options, so the
/// automation `select` command can open it and click an option.
pub fn reported_choice<'a, Message: 'a, Theme: 'a, Renderer: renderer::Renderer + 'a>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    shown: impl AsRef<str>,
    options: &[String],
) -> Element<'a, Message, Theme, Renderer> {
    Element::new(Reported {
        content: content.into(),
        text: shown.as_ref().to_string(),
        options: Some(options.to_vec()),
    })
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Reported<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }
    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }
    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }
    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }
    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        match &self.options {
            Some(options) => {
                let mut info = ChoiceInfo {
                    shown: self.text.clone(),
                    options: options.clone(),
                };
                operation.custom(None, layout.bounds(), &mut info);
            }
            None => operation.text(None, layout.bounds(), &self.text),
        }
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}
