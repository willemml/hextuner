use iced::{
    widget::{
        canvas::{Cache, Frame, Geometry},
        column, container, row,
        scrollable::{Direction, Scrollbar},
        text_input::Status,
        TextInput,
    },
    Element, Length, Padding, Size,
};
use iced_aw::{Grid, GridRow};
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
    pub chart: Chart2D,
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

        row![
            iced::widget::scrollable(
                container(Grid::with_rows(rows)).padding(Padding::new(0.0).bottom(15).right(15)),
            )
            .direction(Direction::Both {
                vertical: Scrollbar::new(),
                horizontal: Scrollbar::new(),
            }),
            column![
                iced::widget::text("Pitch:"),
                iced::widget::slider(0.0..=std::f64::consts::PI, self.chart.pitch, |v| {
                    Message::GraphPitch(self.pane_id, v)
                })
                .step(std::f64::consts::PI / 300.0)
                .width(Length::Fixed(300.0)),
                iced::widget::text("Yaw:"),
                iced::widget::slider(0.0..=std::f64::consts::PI, self.chart.yaw, |v| {
                    Message::GraphYaw(self.pane_id, v)
                })
                .step(std::f64::consts::PI / 300.0)
                .width(Length::Fixed(300.0))
            ],
            ChartWidget::new(&self.chart)
        ]
        .padding(5)
        .into()
    }
}

#[derive(Debug)]
pub struct Chart2D {
    x: Vec<f64>,
    y: Vec<f64>,
    z: Vec<Vec<f64>>,
    cache: Cache,
    pitch: f64,
    yaw: f64,
}

impl Chart2D {
    fn new(x: &[String], y: &[String], z: &[String]) -> Self {
        let x: Vec<f64> = x.iter().map(|f| f.parse().unwrap()).collect();
        let y: Vec<f64> = y.iter().map(|f| f.parse().unwrap()).collect();
        let z_flat: Vec<f64> = z.iter().map(|f| f.parse().unwrap()).collect();

        let z = z_flat.chunks(x.len()).map(|c| c.to_vec()).collect();

        Self {
            x,
            y,
            z,
            pitch: 0.5,
            yaw: 0.5,
            cache: Cache::new(),
        }
    }
    pub fn update(&mut self, x: &[String], y: &[String], z: &[String]) {
        self.x = x.iter().map(|f| f.parse().unwrap()).collect();
        self.y = y.iter().map(|f| f.parse().unwrap()).collect();

        let z_flat: Vec<f64> = z.iter().map(|f| f.parse().unwrap()).collect();
        self.z = z_flat.chunks(x.len()).map(|c| c.to_vec()).collect();

        self.cache.clear();
    }
    pub fn yaw(&mut self, yaw: f64) {
        self.yaw = yaw;
        self.cache.clear();
    }
    pub fn pitch(&mut self, pitch: f64) {
        self.pitch = pitch;
        self.cache.clear();
    }
    fn x_range(&self) -> std::ops::Range<f64> {
        *self.x.iter().min_by(|a, b| a.total_cmp(b)).unwrap()
            ..*self.x.iter().max_by(|a, b| a.total_cmp(b)).unwrap()
    }
    fn y_range(&self) -> std::ops::Range<f64> {
        *self.y.iter().min_by(|a, b| a.total_cmp(b)).unwrap()
            ..*self.y.iter().max_by(|a, b| a.total_cmp(b)).unwrap()
    }
    fn z_range(&self) -> std::ops::Range<f64> {
        *self
            .z
            .iter()
            .filter_map(|r| r.iter().min_by(|a, b| a.total_cmp(b)))
            .min_by(|a, b| a.total_cmp(b))
            .unwrap()
            ..*self
                .z
                .iter()
                .filter_map(|r| r.iter().max_by(|a, b| a.total_cmp(b)))
                .max_by(|a, b| a.total_cmp(b))
                .unwrap()
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

        if self.x.len() == 1 || self.y.len() == 1 {
            let x = if self.x.len() == 1 { &self.y } else { &self.x };
            let y = if self.y.len() == 1 {
                &self.z[0]
            } else {
                &self.y
            };
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
                .build_cartesian_3d(self.x_range(), self.z_range(), self.y_range())
                .expect("failed to build chart");

            chart.with_projection(|mut pb| {
                pb.pitch = self.pitch;
                pb.yaw = self.yaw;
                pb.scale = 0.7;
                pb.into_matrix()
            });

            chart
                .configure_axes()
                .bold_grid_style(plotters::style::colors::BLUE.mix(0.1))
                .light_grid_style(plotters::style::colors::BLUE.mix(0.05))
                // .axis_panel_style(
                //     ShapeStyle::from(plotters::style::colors::BLUE.mix(0.45)).stroke_width(1),
                // )
                .draw()
                .expect("failed to draw chart mesh");

            let iter = (0..(self.y.len() - 1))
                .map(|y| std::iter::repeat(y).zip(0..(self.x.len() - 1)))
                .flatten();

            chart
                .draw_series(iter.clone().map(|(y, x)| {
                    Polygon::new(
                        [
                            (self.x[x], self.z[y][x], self.y[y]),
                            (self.x[x + 1], self.z[y][x + 1], self.y[y]),
                            (self.x[x + 1], self.z[y + 1][x + 1], self.y[y + 1]),
                            (self.x[x], self.z[y + 1][x], self.y[y + 1]),
                        ],
                        ShapeStyle {
                            color: RGBAColor(
                                (((self.z[y][x] + self.z[y + 1][x + 1]) / 2.0
                                    - self.z_range().start)
                                    / (self.z_range().end - self.z_range().start)
                                    * 255.0) as u8,
                                ((1.0
                                    - (((self.z[y][x] + self.z[y + 1][x + 1]) / 2.0
                                        - self.z_range().start)
                                        / (self.z_range().end - self.z_range().start)))
                                    * 255.0) as u8,
                                0,
                                0.5,
                            ),
                            filled: false,
                            stroke_width: 10,
                        },
                    )
                }))
                .expect("failed to draw chart data");
            let x_int = (self.x_range().end - self.x_range().start) / 300.0;
            let y_int = (self.y_range().end - self.y_range().start) / 300.0;
            chart
                .draw_series(iter.clone().map(|(y, x)| {
                    Polygon::new(
                        [
                            (self.x[x], self.z[y][x], self.y[y]),
                            (self.x[x + 1], self.z[y][x + 1], self.y[y]),
                            (self.x[x + 1], self.z[y][x + 1], self.y[y] + y_int),
                            (self.x[x], self.z[y][x], self.y[y] + y_int),
                        ],
                        BLACK,
                    )
                }))
                .unwrap();
            chart
                .draw_series(iter.clone().map(|(y, x)| {
                    Polygon::new(
                        [
                            (self.x[x], self.z[y][x], self.y[y]),
                            (self.x[x], self.z[y + 1][x], self.y[y + 1]),
                            (self.x[x] + x_int, self.z[y + 1][x], self.y[y + 1]),
                            (self.x[x] + x_int, self.z[y][x], self.y[y]),
                        ],
                        BLACK,
                    )
                }))
                .unwrap();
            chart
                .draw_series(iter.clone().map(|(y, x)| {
                    Polygon::new(
                        [
                            (self.x[x], self.z[y + 1][x], self.y[y + 1]),
                            (self.x[x + 1], self.z[y + 1][x + 1], self.y[y + 1]),
                            (self.x[x + 1], self.z[y + 1][x + 1], self.y[y + 1] + y_int),
                            (self.x[x], self.z[y + 1][x], self.y[y + 1] + y_int),
                        ],
                        BLACK,
                    )
                }))
                .unwrap();
            chart
                .draw_series(iter.map(|(y, x)| {
                    Polygon::new(
                        [
                            (self.x[x + 1], self.z[y][x + 1], self.y[y]),
                            (self.x[x + 1], self.z[y + 1][x + 1], self.y[y + 1]),
                            (self.x[x + 1] + x_int, self.z[y + 1][x + 1], self.y[y + 1]),
                            (self.x[x + 1] + x_int, self.z[y][x + 1], self.y[y]),
                        ],
                        BLACK,
                    )
                }))
                .unwrap();
            chart
                .draw_series(
                    (0..self.y.len())
                        .map(|y| std::iter::repeat(y).zip(0..self.x.len()))
                        .flatten()
                        .map(|(y, x)| {
                            Circle::new((self.x[x], self.z[y][x], self.y[y]), 4, BLACK.filled())
                        }),
                )
                .unwrap();
        }
    }
}
