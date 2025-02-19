use std::collections::HashMap;
use std::fs::File;
use std::sync::{Arc, Mutex};

use definitions::{Scalar, Table};

use iced::widget::pane_grid;
use iced::{Element, Task};
use views::map_nav::MapNav;
use views::panes::{PaneAction, PaneContent};
use views::table::EditSource;
use xdftuneparser::data_types::XDFElement;
use xdftuneparser::parse_buffer;

pub mod definitions;
pub mod eval;

mod views;

#[derive(Debug)]
pub struct RWGuarded<RW> {
    inner: Arc<Mutex<RW>>,
}

impl<RW> Clone for RWGuarded<RW> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub type FileGuard = RWGuarded<File>;

impl From<File> for RWGuarded<File> {
    fn from(value: File) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
        }
    }
}

impl<RW: std::io::Read> std::io::Read for RWGuarded<RW> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // TODO: handle unwrap?
        self.inner.lock().unwrap().read(buf)
    }
}

impl<RW: std::io::Seek> std::io::Seek for RWGuarded<RW> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.lock().unwrap().seek(pos)
    }
}

impl<RW: std::io::Write> std::io::Write for RWGuarded<RW> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.lock().unwrap().flush()
    }
}

// TODO: use internal IDs instead of filenames
// TODO: move binary to ram for editing, write to different filename
//       avoid keeping files open longer than necessary?
pub struct App {
    /// Binaries, mapped to their names and corresponding definition
    binary: FileGuard,
    panes: pane_grid::State<views::panes::Pane>,
    panes_created: usize,
    pane_id_map: HashMap<usize, pane_grid::Pane>,
    focus: Option<pane_grid::Pane>,
}

impl App {
    fn new(bin: File, def: definitions::BinaryDefinition) -> Self {
        let mut nav = MapNav::default();
        nav.tables = def.tables.clone();
        nav.scalars = def.scalars.clone();
        let (panes, nav_pane) = pane_grid::State::new(views::panes::Pane::nav(def.clone()));
        let mut pane_id_map = HashMap::new();
        pane_id_map.insert(0, nav_pane.clone());
        Self {
            binary: FileGuard::from(bin),
            panes,
            panes_created: 1,
            pane_id_map,
            focus: Some(nav_pane),
        }
    }
    fn view(&self) -> Element<Message> {
        views::panes::view_grid(self)
    }
    fn update(&mut self, message: Message) {
        match message {
            Message::Open(kind) => views::panes::open(self, kind, self.binary.clone()),
            Message::EditCell {
                value,
                pane,
                source,
            } => {
                let pane = self.pane_id_map.get(&pane).unwrap();
                if let PaneContent::Table(table_view) =
                    &mut self.panes.get_mut(*pane).unwrap().content
                {
                    match source {
                        EditSource::YHead(n) => table_view.y_head[n] = value,
                        EditSource::XHead(n) => table_view.x_head[n] = value,
                        EditSource::Data { x, y } => table_view.data[y][x] = value,
                    }
                }
            }
            Message::WriteTable { pane } => {
                let pane = self.pane_id_map.get(&pane).unwrap();
                if let PaneContent::Table(table_view) =
                    &mut self.panes.get_mut(*pane).unwrap().content
                {
                    table_view
                        .table
                        .x
                        .write(
                            &mut table_view.source,
                            table_view
                                .x_head
                                .iter()
                                .map(|s| s.parse().unwrap())
                                .collect(),
                        )
                        .unwrap();
                    table_view
                        .table
                        .y
                        .write(
                            &mut table_view.source,
                            table_view
                                .y_head
                                .iter()
                                .map(|s| s.parse().unwrap())
                                .collect(),
                        )
                        .unwrap();
                    table_view
                        .table
                        .z
                        .write(
                            &mut table_view.source,
                            table_view
                                .data
                                .concat()
                                .iter()
                                .map(|s| s.parse().unwrap())
                                .collect(),
                        )
                        .unwrap();
                }
            }
            Message::EditScalar { value, pane } => {
                let pane = self.pane_id_map.get(&pane).unwrap();
                if let PaneContent::Scalar(scalar_view) =
                    &mut self.panes.get_mut(*pane).unwrap().content
                {
                    scalar_view.value = value;
                }
            }
            Message::WriteScalar { pane } => {
                let pane = self.pane_id_map.get(&pane).unwrap();
                if let PaneContent::Scalar(scalar_view) =
                    &mut self.panes.get_mut(*pane).unwrap().content
                {
                    scalar_view
                        .scalar
                        .write(&mut scalar_view.source, scalar_view.value.parse().unwrap())
                        .unwrap();
                }
            }
            Message::PaneAction(action) => views::panes::update_panes(self, action),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Open {
    // Nav(BinaryDefinition),
    Table(Table),
    Scalar(Scalar),
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Open(Open),
    EditCell {
        value: String,
        pane: usize,
        source: EditSource,
    },
    WriteTable {
        pane: usize,
    },
    EditScalar {
        value: String,
        pane: usize,
    },
    WriteScalar {
        pane: usize,
    },
    PaneAction(PaneAction),
}

fn main() -> iced::Result {
    let xdf = File::open("testfiles/8E0909518AK_368072_NEF_STG_1v7.xdf").unwrap();

    let xdf_parsed = parse_buffer(xdf).unwrap().unwrap();

    let bin = File::options()
        .write(true)
        .read(true)
        .open("testfiles/test.bin")
        .unwrap();

    let def = if let XDFElement::XDFFormat(xdf) = xdf_parsed {
        definitions::BinaryDefinition::from_xdf(xdf)
    } else {
        panic!("Expected full XDF file.");
    };

    iced::application("HEXTuner", App::update, App::view)
        .run_with(|| (App::new(bin, def), Task::none()))
}
