use iced::{
    widget::{
        container,
        scrollable::{Direction, Scrollbar},
        text_input::Status,
        TextInput,
    },
    Element,
    Length::{self, Fill},
    Padding,
};
use iced_aw::{Grid, GridRow};

use crate::{definitions::Table, FileGuard, Message};

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
        writeable: bool,
    ) -> Element<'a, Message> {
        let mut text_box = TextInput::new("", value).width(Length::Fixed(100.0));

        if writeable {
            text_box = text_box
                .on_submit(Message::WriteTable { pane: self.pane_id })
                .on_input(move |value| Message::EditCell {
                    value,
                    pane: self.pane_id,
                    source,
                });
        } else {
            text_box = text_box.style(|theme, status| {
                let mut style = iced::widget::text_input::default(theme, status);
                style.value = iced::widget::text_input::default(theme, Status::Active).value;
                style
            });
        }

        text_box.into()
    }

    pub fn view(&self) -> Element<Message> {
        let x_writeable = self.table.x.writeable();
        let y_writeable = self.table.y.writeable();
        let data_writeable = self.table.z.writeable();

        let mut rows: Vec<GridRow<Message>> = Vec::new();
        let mut first_row = GridRow::with_elements(vec![Element::from("")]);
        for x in self
            .x_head
            .iter()
            .enumerate()
            .map(|(x, xv)| self.cell(xv, EditSource::XHead(x), x_writeable))
        {
            first_row = first_row.push(x);
        }
        rows.push(first_row);

        let mut i = 0;
        for (y, yv) in self.y_head.iter().enumerate() {
            let mut grid_row = Vec::new();
            grid_row.push(self.cell(yv, EditSource::YHead(y), y_writeable));

            for _ in 0..self.x_head.len() {
                grid_row.push(self.cell(&self.data[i], EditSource::Data(i), data_writeable));
                i += 1;
            }

            rows.push(GridRow::with_elements(grid_row));
        }

        container(
            iced::widget::scrollable(
                container(Grid::with_rows(rows)).padding(Padding::new(0.0).bottom(15).right(15)),
            )
            .direction(Direction::Both {
                vertical: Scrollbar::new(),
                horizontal: Scrollbar::new(),
            })
            .width(Fill)
            .height(Fill),
        )
        .padding(5)
        .into()
    }
}
