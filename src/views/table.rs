use std::ops::Range;

use iced::{
    widget::{
        canvas::{Cache, Frame, Geometry},
        column, container,
        scrollable::{Direction, Scrollbar},
        text_input::Status,
        TextInput,
    },
    Element,
    Length::{self, Fill},
    Padding, Size,
};
use iced_aw::{Grid, GridRow};
use plotters::{coord::types::RangedCoordf64, style::full_palette::BLACK};
use plotters_iced::{Chart, ChartWidget};

use crate::{definitions::Table, FileGuard, Message};

#[derive(Debug)]
pub struct TableView {
    pane_id: usize,
    pub table: Table,
    pub x_head: Vec<String>,
    pub y_head: Vec<String>,
    pub data: Vec<String>,
    pub source: FileGuard,
    chart: Chart2D,
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
        let chart = Chart2D::new(x_head.as_slice(), y_head.as_slice(), data.as_slice());

        Self {
            chart,
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

        column![
            iced::widget::scrollable(
                container(Grid::with_rows(rows)).padding(Padding::new(0.0).bottom(15).right(15)),
            )
            .direction(Direction::Both {
                vertical: Scrollbar::new(),
                horizontal: Scrollbar::new(),
            })
            .width(Fill)
            .height(Fill),
            ChartWidget::new(&self.chart)
        ]
        .padding(5)
        .into()
    }
}

#[derive(Debug)]
struct Chart2D {
    x: Vec<f64>,
    y: Vec<f64>,
    z: Vec<f64>,
    cache: Cache,
}

impl Chart2D {
    fn new(x: &[String], y: &[String], z: &[String]) -> Self {
        let x: Vec<f64> = x.iter().map(|f| f.parse().unwrap()).collect();
        let y: Vec<f64> = y.iter().map(|f| f.parse().unwrap()).collect();
        let z: Vec<f64> = z.iter().map(|f| f.parse().unwrap()).collect();
        Self {
            x,
            y,
            z,
            cache: Cache::new(),
        }
    }
    fn x_range(&self) -> RangedCoordf64 {
        (*self.x.iter().min_by(|a, b| a.total_cmp(b)).unwrap()
            ..*self.x.iter().max_by(|a, b| a.total_cmp(b)).unwrap())
            .into()
    }
    fn y_range(&self) -> RangedCoordf64 {
        (*self.y.iter().min_by(|a, b| a.total_cmp(b)).unwrap()
            ..*self.y.iter().max_by(|a, b| a.total_cmp(b)).unwrap())
            .into()
    }
    fn z_range(&self) -> RangedCoordf64 {
        (*self.z.iter().min_by(|a, b| a.total_cmp(b)).unwrap()
            ..*self.z.iter().max_by(|a, b| a.total_cmp(b)).unwrap())
            .into()
    }
}

impl Chart<Message> for Chart2D {
    type State = ();

    #[inline]
    fn draw<R: plotters_iced::Renderer, F: Fn(&mut Frame)>(
        &self,
        renderer: &R,
        bounds: Size,
        draw_fn: F,
    ) -> Geometry {
        renderer.draw_cache(&self.cache, bounds, draw_fn)
    }
    fn build_chart<DB: plotters_iced::DrawingBackend>(
        &self,
        _state: &Self::State,
        mut builder: plotters_iced::ChartBuilder<DB>,
    ) {
        use plotters::prelude::*;

        const PLOT_LINE_COLOR: RGBColor = RGBColor(0, 175, 255);

        if self.x.len() == 1 || self.y.len() == 1 {
            let x = if self.x.len() == 1 { &self.y } else { &self.x };
            let y = if self.y.len() == 1 { &self.z } else { &self.y };
            let mut chart = builder
                .x_label_area_size(28)
                .y_label_area_size(28)
                .margin(20)
                .build_cartesian_2d(
                    if self.x.len() == 1 {
                        self.y_range()
                    } else {
                        self.x_range()
                    },
                    if self.y.len() == 1 {
                        self.z_range()
                    } else {
                        self.y_range()
                    },
                )
                .expect("failed to build chart");
            chart
                .configure_mesh()
                .bold_line_style(plotters::style::colors::BLUE.mix(0.1))
                .light_line_style(plotters::style::colors::BLUE.mix(0.05))
                .axis_style(
                    ShapeStyle::from(plotters::style::colors::BLUE.mix(0.45)).stroke_width(1),
                )
                .draw()
                .expect("failed to draw chart mesh");
            let series = LineSeries::new(x.clone().into_iter().zip(y.clone()), BLACK);

            chart
                .draw_series(series)
                .expect("failed to draw chart data");
        } else {
            let mut chart = builder
                .x_label_area_size(28)
                .y_label_area_size(28)
                .margin(20)
                .build_cartesian_3d(0..self.x.len(), 0..self.y.len(), self.z_range())
                .expect("failed to build chart");
            chart
                .configure_axes()
                .bold_grid_style(plotters::style::colors::BLUE.mix(0.1))
                .light_grid_style(plotters::style::colors::BLUE.mix(0.05))
                .axis_panel_style(
                    ShapeStyle::from(plotters::style::colors::BLUE.mix(0.45)).stroke_width(1),
                )
                .draw()
                .expect("failed to draw chart mesh");

            let series = SurfaceSeries::xoy(0..self.x.len(), 0..self.y.len(), |x, y| self.z[x + y]);

            chart
                .draw_series(series)
                .expect("failed to draw chart data");
        }
    }
}
