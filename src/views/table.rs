use iced::{
    widget::{column, row, text, text_input::Style, Row, TextInput},
    Border, Color, Element,
};

use crate::{definitions::Table, FileGuard, Message};

#[derive(Debug, Clone, Copy)]
enum CellStyle {
    Head,
    Normal,
    Alt,
}

#[derive(Debug)]
pub struct TableView {
    pane_id: usize,
    pub table: Table,
    pub x_head: Vec<String>,
    pub y_head: Vec<String>,
    pub data: Vec<Vec<String>>,
    pub source: FileGuard,
}

#[derive(Debug, Copy, Clone)]
pub enum EditSource {
    YHead(usize),
    XHead(usize),
    Data { x: usize, y: usize },
}

impl TableView {
    pub fn new(pane_id: usize, table: Table, mut source: FileGuard) -> Self {
        let x_head: Vec<String> = table
            .x
            .read(&mut source)
            .unwrap()
            .iter()
            .map(f64::to_string)
            .collect();
        let y_head = table
            .y
            .read(&mut source)
            .unwrap()
            .iter()
            .map(f64::to_string)
            .collect();
        let data = table
            .z
            .read(&mut source)
            .unwrap()
            .chunks(x_head.len())
            .map(|chunk| chunk.iter().map(f64::to_string).collect())
            .collect();

        Self {
            pane_id,
            table,
            x_head,
            y_head,
            data,
            source,
        }
    }

    fn cell(&self, value: &str, source: EditSource, style: CellStyle) -> TextInput<Message> {
        TextInput::new("", value)
            .style(move |_, status| {
                let border = Border::default().width(match status {
                    iced::widget::text_input::Status::Hovered => 0.75,
                    iced::widget::text_input::Status::Focused => 1.0,
                    _ => 0.5,
                });
                let bg = Color::from_rgba(
                    1.0,
                    1.0,
                    1.0,
                    match style {
                        CellStyle::Head => 0.5,
                        CellStyle::Normal => 0.0,
                        CellStyle::Alt => 0.25,
                    },
                );

                Style {
                    background: bg.into(),
                    border,
                    icon: Color::TRANSPARENT,
                    placeholder: Color::TRANSPARENT,
                    value: Color::BLACK,
                    selection: Color::from_rgba(0.0, 0.0, 1.0, 0.25),
                }
            })
            .on_input(move |value| Message::EditCell {
                value,
                pane: self.pane_id,
                source,
            })
            .on_submit(Message::WriteTable { pane: self.pane_id })
    }

    fn row(&self, values: &[String], style: CellStyle, y: usize) -> Row<Message> {
        row(values
            .iter()
            .enumerate()
            .map(|(x, v)| Element::from(self.cell(v, EditSource::Data { x, y }, style))))
    }

    pub fn view(&self) -> Element<Message> {
        let mut first_col = vec![Element::from(text(""))];
        for (n, v) in self.y_head.iter().enumerate() {
            if self.table.y.writeable() {
                first_col.push(Element::from(self.cell(
                    v,
                    EditSource::YHead(n),
                    CellStyle::Head,
                )));
            } else {
                first_col.push(Element::from(text(v)))
            }
        }

        let mut first_row = Vec::new();

        for (n, v) in self.x_head.iter().enumerate() {
            if self.table.x.writeable() {
                first_row.push(Element::from(self.cell(
                    v,
                    EditSource::XHead(n),
                    CellStyle::Head,
                )));
            } else {
                first_row.push(Element::from(text(v)));
            }
        }

        let mut rows = vec![row(first_row).into()];
        let mut alt = false;

        for (n, drow) in self.data.iter().enumerate() {
            rows.push(Element::from(self.row(
                drow,
                if alt {
                    CellStyle::Alt
                } else {
                    CellStyle::Normal
                },
                n,
            )));
            alt = !alt;
        }

        row(vec![column(first_col).into(), column(rows).into()]).into()
    }
}
