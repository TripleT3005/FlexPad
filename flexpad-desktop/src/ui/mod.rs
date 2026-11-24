use iced::widget::{Button, button, text};

pub mod choice;
pub mod header;
pub mod inspector;
pub mod workspace;

// Widgets

pub fn label_button<'a, Message: 'a>(content: String) -> Button<'a, Message> {
    button(text(content).wrapping(text::Wrapping::None))
}
