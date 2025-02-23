use iced::{
    advanced::graphics::color,
    clipboard::write,
    widget::{column, container, row, scrollable, text, text_input::Style, Row, TextInput},
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
    pub table: Table,
    pub x_head: Vec<String>,
    pub y_head: Vec<String>,
    pub data: Vec<String>,
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
        let x_head: Vec<String> = table.x.read_strings(&mut source).unwrap();
        let y_head = table.y.read_strings(&mut source).unwrap();
        let data = table.z.read_strings(&mut source).unwrap();

        Self {
            pane_id,
            table,
            x_head,
            y_head,
            data,
            source,
        }
    }

    fn cell<'a>(
        &'a self,
        value: &'a str,
        source: EditSource,
        _style: CellStyle,
        writeable: bool,
    ) -> Element<'a, Message> {
        if writeable {
            TextInput::new("", value)
                .on_input(move |value| Message::EditCell {
                    value,
                    pane: self.pane_id,
                    source,
                })
                .on_submit(Message::WriteTable { pane: self.pane_id })
                .into()
        } else {
            Element::from(value)
        }
    }

    pub fn view(&self) -> Element<Message> {
        let x_writeable = self.table.x.writeable() && false;
        let y_writeable = self.table.y.writeable() && false;
        let data_writeable = self.table.z.writeable() && false;

        let mut rows: Vec<GridRow<Message>> = Vec::new();
        let mut first_row = GridRow::with_elements(vec![Element::from("")]);
        for x in self
            .x_head
            .iter()
            .enumerate()
            .map(|(x, xv)| self.cell(xv, EditSource::XHead(x), CellStyle::Head, x_writeable))
        {
            first_row = first_row.push(x);
        }
        rows.push(first_row);

        let mut i = 0;
        for (y, yv) in self.y_head.iter().enumerate() {
            let mut grid_row = Vec::new();
            grid_row.push(self.cell(yv, EditSource::YHead(y), CellStyle::Head, y_writeable));

            for _ in 0..self.x_head.len() {
                grid_row.push(self.cell(
                    &self.data[i],
                    EditSource::Data(i),
                    CellStyle::Normal,
                    data_writeable,
                ));
                i += 1;
            }

            rows.push(GridRow::with_elements(grid_row));
        }

        Grid::with_rows(rows).into()
    }
}

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
