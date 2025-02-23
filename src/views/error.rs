use iced::{widget::text, Element};

use crate::Message;

pub struct ErrorView {
    text: String,
}
impl ErrorView {
    pub fn new(text: String) -> Self {
        Self { text }
    }
    pub fn view(&self) -> Element<Message> {
        text(&self.text).into()
    }
}
