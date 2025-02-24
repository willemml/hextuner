use iced::{
    widget::{
        button, container,
        pane_grid::{self, DragEvent, ResizeEvent},
        row, text, PaneGrid,
    },
    Element,
    Length::Fill,
};

use crate::{
    definitions::{BinaryDefinition, Scalar, Table},
    FileGuard, Message,
};

use super::{error::ErrorView, map_nav::MapNav, scalar::ScalarView, table::TableView};

pub struct Pane {
    is_pinned: bool,
    pub content: PaneContent,
    title: String,
}
impl Pane {
    pub fn nav(bin_def: BinaryDefinition) -> Self {
        Self {
            is_pinned: true,
            content: PaneContent::Nav(MapNav {
                tables: bin_def.tables,
                scalars: bin_def.scalars,
            }),
            title: bin_def.info.name,
        }
    }

    pub fn table(table: Table, file: FileGuard, id: usize) -> Self {
        Self {
            is_pinned: false,
            title: table.name.clone(),
            content: PaneContent::Table(TableView::new(id, table, file)),
        }
    }
    pub fn scalar(scalar: Scalar, file: FileGuard, id: usize) -> Self {
        Self {
            is_pinned: false,
            title: scalar.name.clone(),
            content: PaneContent::Scalar(ScalarView::new(id, scalar, file)),
        }
    }
    pub fn error(error: String) -> Self {
        Self {
            is_pinned: false,
            title: "Error!".to_string(),
            content: PaneContent::Error(ErrorView::new(error)),
        }
    }
}
pub enum PaneContent {
    Table(TableView),
    Nav(MapNav),
    Scalar(ScalarView),
    Error(ErrorView),
}

#[derive(Debug, Clone)]
pub(crate) enum PaneAction {
    Close(pane_grid::Pane),
    Maximize(pane_grid::Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    TogglePin(pane_grid::Pane),
    Clicked(pane_grid::Pane),
    Restore,
}

impl Into<Message> for PaneAction {
    fn into(self) -> Message {
        Message::PaneAction(self)
    }
}

pub fn update_panes(app: &mut crate::App, action: PaneAction) {
    match action {
        PaneAction::Close(pane) => {
            if let Some((_, sibling)) = app.panes.close(pane) {
                app.focus = Some(sibling);
            }
        }
        PaneAction::Maximize(pane) => app.panes.maximize(pane),
        PaneAction::Dragged(DragEvent::Dropped { pane, target }) => app.panes.drop(pane, target),
        PaneAction::Dragged(_) => {}
        PaneAction::Resized(ResizeEvent { split, ratio }) => app.panes.resize(split, ratio),
        PaneAction::TogglePin(pane) => {
            if let Some(Pane { is_pinned, .. }) = app.panes.get_mut(pane) {
                *is_pinned = !*is_pinned;
            }
        }
        PaneAction::Clicked(pane) => app.focus = Some(pane),
        PaneAction::Restore => app.panes.restore(),
    }
}

pub fn open(app: &mut crate::App, kind: crate::Open, binary: FileGuard) -> Option<pane_grid::Pane> {
    let id = app.panes_created;
    app.panes_created += 1;

    if let Some((pane, _)) = app.panes.split(
        pane_grid::Axis::Horizontal,
        app.focus
            .unwrap_or(app.panes.iter().last().unwrap().0.clone()),
        match kind {
            // crate::Open::Nav(binary_definition) => Pane::nav(binary_definition),
            crate::Open::Error(error) => Pane::error(error),
            crate::Open::Table(table) => Pane::table(table, binary, id),
            crate::Open::Scalar(scalar) => Pane::scalar(scalar, binary, id),
        },
    ) {
        app.pane_id_map.insert(id, pane);
        Some(pane)
    } else {
        None
    }
}

pub fn view_grid<'a>(app: &crate::App) -> Element<Message> {
    let focus = app.focus;
    let total_panes = app.panes.len();

    let pane_grid = PaneGrid::new(&app.panes, |id, pane, is_maximized| {
        let is_focused = focus == Some(id);

        let pin_button = button(text(if pane.is_pinned { "Unpin" } else { "Pin" }).size(14))
            .on_press(Message::PaneAction(PaneAction::TogglePin(id)))
            .padding(3);

        let title = row![pin_button, text(pane.title.clone())].spacing(5);

        let title_bar = pane_grid::TitleBar::new(title)
            .controls(pane_grid::Controls::dynamic(
                view_controls(id, total_panes, pane.is_pinned, is_maximized),
                button(text("X").size(14))
                    .style(button::danger)
                    .padding(3)
                    .on_press_maybe(if total_panes > 1 && !pane.is_pinned {
                        Some(PaneAction::Close(id).into())
                    } else {
                        None
                    }),
            ))
            .padding(10)
            .style(if is_focused {
                style::title_bar_focused
            } else {
                style::title_bar_active
            });

        pane_grid::Content::new(iced::widget::responsive(|_size| {
            container(match &pane.content {
                PaneContent::Table(v) => v.view(),
                PaneContent::Nav(m) => m.view(),
                PaneContent::Scalar(s) => s.view(),
                PaneContent::Error(e) => e.view(),
            })
            .clip(true)
            .into()
        }))
        .style(if is_focused {
            style::pane_focused
        } else {
            style::pane_active
        })
        .title_bar(title_bar)
    })
    .width(Fill)
    .height(Fill)
    .spacing(10)
    .on_click(|p| PaneAction::Clicked(p).into())
    .on_drag(|d| PaneAction::Dragged(d).into())
    .on_resize(10, |r| PaneAction::Resized(r).into());

    iced::widget::container(pane_grid)
        .width(Fill)
        .height(Fill)
        .padding(10)
        .into()
}

fn view_controls<'a>(
    pane: pane_grid::Pane,
    total_panes: usize,
    is_pinned: bool,
    is_maximized: bool,
) -> Element<'a, Message> {
    let row = row![].spacing(5).push_maybe(if total_panes > 1 {
        let (content, message) = if is_maximized {
            ("Restore", PaneAction::Restore.into())
        } else {
            ("Maximize", PaneAction::Maximize(pane).into())
        };

        Some(
            button(text(content).size(14))
                .style(button::secondary)
                .padding(3)
                .on_press(message),
        )
    } else {
        None
    });

    let close = button(text("Close").size(14))
        .style(button::danger)
        .padding(3)
        .on_press_maybe(if total_panes > 1 && !is_pinned {
            Some(PaneAction::Close(pane).into())
        } else {
            None
        });

    row.push(close).into()
}

mod style {
    use iced::widget::container;
    use iced::{Border, Theme};

    pub fn title_bar_active(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            text_color: Some(palette.background.strong.text),
            background: Some(palette.background.strong.color.into()),
            ..Default::default()
        }
    }

    pub fn title_bar_focused(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            text_color: Some(palette.primary.strong.text),
            background: Some(palette.primary.strong.color.into()),
            ..Default::default()
        }
    }

    pub fn pane_active(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            background: Some(palette.background.weak.color.into()),
            border: Border {
                width: 2.0,
                color: palette.background.strong.color,
                ..Border::default()
            },
            ..Default::default()
        }
    }

    pub fn pane_focused(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            background: Some(palette.background.weak.color.into()),
            border: Border {
                width: 2.0,
                color: palette.primary.strong.color,
                ..Border::default()
            },
            ..Default::default()
        }
    }
}
