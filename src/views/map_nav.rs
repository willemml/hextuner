use std::collections::HashMap;

use iced::{
    widget::{
        self,
        button::{Status, Style},
        column, scrollable, text,
    },
    Color, Element, Length, Theme,
};

use crate::{
    definitions::{Scalar, Table},
    Message, Open,
};

#[derive(Default, Clone, Debug)]
pub struct MapNav {
    pub tables: Vec<Table>,
    pub scalars: Vec<Scalar>,
    pub categories: HashMap<u32, String>,
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
        let categories = column(self.categories.iter().map(|(index, name)| {
            let mut column = column![text(name).size(30)];
            let scalars: Vec<Element<Message>> = self
                .scalars
                .iter()
                .filter_map(|s| {
                    if s.categories.contains(index) {
                        Some(Element::from(
                            widget::button(text(s.name.clone()))
                                .on_press(Message::Open(Open::Scalar(s.clone())))
                                .width(Length::Fill)
                                .style(button_color),
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            let tables: Vec<Element<Message>> = self
                .tables
                .iter()
                .filter_map(|t| {
                    if t.categories.contains(index) {
                        Some(Element::from(
                            widget::button(text(t.name.clone()))
                                .on_press(Message::Open(Open::Table(t.clone())))
                                .width(Length::Fill)
                                .style(button_color),
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            let scalars_empty = scalars.is_empty();

            if !scalars_empty {
                if !tables.is_empty() {
                    column = column.push(text("Scalars").size(20));
                }
                column = column.extend(scalars);
            }

            if !tables.is_empty() {
                if !scalars_empty {
                    column = column.push(text("Tables").size(20));
                }
                column = column.extend(tables);
            }

            Element::from(column)
        }));

        scrollable(categories).into()
    }
}
