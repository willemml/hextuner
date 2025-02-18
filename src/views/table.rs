use iced::{
    widget::{column, row, text, text_input::Style, PaneGrid, Row, TextInput},
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
    table: Table,
    col_head: Vec<String>,
    row_head: Vec<String>,
    data: Vec<Vec<String>>,
    source: FileGuard,
}

impl TableView {
    pub fn new(table: Table, mut source: FileGuard) -> Self {
        let col_head: Vec<String> = table
            .x
            .read(&mut source)
            .unwrap()
            .iter()
            .map(f64::to_string)
            .collect();
        let row_head = table
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
            .chunks(col_head.len())
            .map(|chunk| chunk.iter().map(f64::to_string).collect())
            .collect();

        Self {
            table,
            col_head,
            row_head,
            data,
            source,
        }
    }

    fn cell(&self, text: String, style: CellStyle) -> TextInput<Message> {
        TextInput::new("", &text)
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
            .on_submit(Message::EditCell(self.table.clone(), self.source.clone()))
    }

    fn row(&self, values: Vec<String>, style: CellStyle) -> Row<Message> {
        row(values
            .into_iter()
            .map(|v| Element::from(self.cell(v, style))))
    }

    pub fn view(&self) -> Element<Message> {
        let mut first_col = vec![Element::from(text(""))];
        for v in self.row_head.iter() {
            if self.table.y.writeable() {
                first_col.push(Element::from(self.cell(v.clone(), CellStyle::Head)));
            } else {
                first_col.push(Element::from(text(v)))
            }
        }

        let mut first_row = Vec::new();

        for v in self.col_head.iter() {
            if self.table.x.writeable() {
                first_row.push(Element::from(self.cell(v.clone(), CellStyle::Head)));
            } else {
                first_row.push(Element::from(text(v)));
            }
        }

        let mut rows = vec![row(first_row).into()];
        let mut alt = false;

        for drow in self.data.iter() {
            rows.push(Element::from(self.row(
                drow.clone(),
                if alt {
                    CellStyle::Alt
                } else {
                    CellStyle::Normal
                },
            )));
            alt = !alt;
        }

        row(vec![column(first_col).into(), column(rows).into()]).into()
    }
}
