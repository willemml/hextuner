use iced::{widget::text_input, Element};

use crate::{definitions::Scalar, FileGuard, Message};

#[derive(Debug)]
pub struct ScalarView {
    pane_id: usize,
    pub scalar: Scalar,
    pub value: String,
    pub source: FileGuard,
}

impl ScalarView {
    pub fn new(pane_id: usize, scalar: Scalar, mut source: FileGuard) -> Self {
        let value = scalar.read(&mut source).unwrap().to_string();

        Self {
            pane_id,
            scalar,
            value,
            source,
        }
    }

    pub fn view(&self) -> Element<Message> {
        text_input("", &self.value)
            .on_input(|value| Message::EditScalar {
                value,
                pane: self.pane_id,
            })
            .on_submit(Message::WriteScalar { pane: self.pane_id })
            .width(100)
            .into()
    }
}
