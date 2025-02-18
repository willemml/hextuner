use std::any::Any;

use iced::{
    widget::{
        self,
        button::{Status, Style},
        column, scrollable, text, Column,
    },
    Background, Color, Element, Length, Renderer, Theme,
};

use crate::{
    definitions::{Scalar, Table},
    Message,
};

#[derive(Default, Clone, Debug)]
pub struct MapNav {
    pub tables: Vec<Table>,
    pub scalars: Vec<Scalar>,
}

fn button_color(_: &Theme, status: Status) -> Style {
    Style::default().with_background(match status {
        Status::Hovered => Color::from_rgba(0.0, 1.0, 1.0, 0.5),
        Status::Pressed => Color::from_rgb(0.0, 1.0, 1.0),
        _ => Color::TRANSPARENT,
    })
}

impl MapNav {
    pub fn view(&self) -> Element<Message> {
        let scalars: Vec<Element<Message>> = self
            .scalars
            .iter()
            .map(|s| {
                Element::from(
                    widget::button(text(s.name.clone()))
                        .on_press(Message::OpenScalar(s.clone()))
                        .width(Length::Fill)
                        .style(button_color),
                )
            })
            .collect();

        let tables: Vec<Element<Message>> = self
            .tables
            .iter()
            .map(|t| {
                Element::from(
                    widget::button(text(t.name.clone()))
                        .on_press(Message::OpenTable(t.clone()))
                        .width(Length::Fill)
                        .style(button_color),
                )
            })
            .collect();

        scrollable(column![
            text("Scalars").size(20),
            column(scalars),
            text("Tables").size(20),
            column(tables)
        ])
        .into()
    }
}
