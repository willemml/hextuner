use iced::{
    advanced::graphics::color,
    clipboard::write,
    widget::{
        column, container, responsive, row,
        scrollable::{self, AbsoluteOffset},
        text,
        text_input::Style,
        Row, TextInput,
    },
    Border, Color, Element, Length, Renderer, Theme,
};
use iced_aw::{Grid, GridRow};
use iced_table::table;

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
    columns: Vec<Column>,
    rows: Vec<Vec<Cell>>,
    header: scrollable::Id,
    body: scrollable::Id,
    footer: scrollable::Id,
    table: Table,
    pub source: FileGuard,
}

#[derive(Debug, Copy, Clone)]
pub enum EditSource {
    YHead(usize),
    XHead(usize),
    Data(usize),
}

impl TableView {
    pub fn new(pane_id: usize, table: Table, mut source: FileGuard) -> Self {
        let col_write = table.x.writeable();
        let columns = table
            .x
            .read_strings(&mut source)
            .unwrap()
            .into_iter()
            .map(|s| {
                Column::new(
                    if col_write {
                        Cell::Edit(s)
                    } else {
                        Cell::Constant(s)
                    },
                    pane_id,
                )
            })
            .collect();

        let row_write = table.y.writeable();
        let mut rows: Vec<Vec<Cell>> = table
            .y
            .read_strings(&mut source)
            .unwrap()
            .into_iter()
            .map(|s| {
                vec![if row_write {
                    Cell::Edit(s)
                } else {
                    Cell::Constant(s)
                }]
            })
            .collect();
        let mut data = table.z.read_strings(&mut source).unwrap().into_iter();

        for y in 0..rows.len() {
            for _ in 0..table.x.len() {
                rows[y].push(Cell::Edit(data.next().unwrap()));
            }
        }

        Self {
            pane_id,
            columns,
            rows,
            table,
            source,
            header: scrollable::Id::unique(),
            body: scrollable::Id::unique(),
            footer: scrollable::Id::unique(),
        }
    }

    pub fn view(&self) -> Element<Message> {
        let table = responsive(|size| {
            let pane_id = self.pane_id;

            let message = |offset: AbsoluteOffset| Message::TableUpdate {
                pane: pane_id,
                message: crate::TableUpdate::SyncHeader(o),
            };

            let mut table = table(
                self.header.clone(),
                self.body.clone(),
                &self.columns,
                &self.rows,
                message,
            );

            table.into()
        });
        container(container(table).width(Length::Fill).height(Length::Fill))
            .padding(20)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}

#[derive(Debug)]
enum Cell {
    Blank,
    Edit(String),
    Constant(String),
}

impl Cell {
    fn writeable(&self) -> bool {
        match self {
            Self::Edit(_) => true,
            _ => false,
        }
    }

    fn value<'a>(&'a self) -> &'a str {
        match &self {
            Self::Edit(v) | Self::Constant(v) => v,
            Self::Blank => "",
        }
    }
}

#[derive(Debug)]
struct Column {
    width: f32,
    header: Cell,
    pane: usize,
    resize_offset: Option<f32>,
}

impl Column {
    fn new(header: Cell, pane: usize) -> Self {
        Self {
            header,
            pane,
            width: 400.0,
            resize_offset: None,
        }
    }
}
fn cell_view<'a>(
    pane: usize,
    cell: &'a Cell,
    source: EditSource,
    writeable: bool,
) -> Element<'a, Message> {
    let value = cell.value();
    if writeable {
        TextInput::new("", value)
            .on_input(move |value| Message::EditCell {
                value,
                pane,
                source,
            })
            .on_submit(Message::WriteTable { pane })
            .into()
    } else {
        Element::from(value)
    }
}

impl<'a> table::Column<'a, Message, Theme, Renderer> for Column {
    type Row = Vec<Cell>;

    fn header(&'a self, col_index: usize) -> Element<'a, Message, Theme, Renderer> {
        cell_view(
            self.pane,
            &self.header,
            EditSource::XHead(col_index - 1),
            true,
        )
    }

    fn cell(
        &'a self,
        col_index: usize,
        row_index: usize,
        row: &'a Self::Row,
    ) -> Element<'a, Message, Theme, Renderer> {
        let cell = &row[col_index];
        let write = cell.writeable();
        if col_index == 0 {
            cell_view(self.pane, &cell, EditSource::YHead(row_index), write)
        } else {
            cell_view(self.pane, &cell, EditSource::Data(row_index), write)
        }
    }

    fn width(&self) -> f32 {
        self.width
    }

    fn resize_offset(&self) -> Option<f32> {
        self.resize_offset
    }
}
