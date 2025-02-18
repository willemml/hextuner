use std::fs::File;
use std::sync::{Arc, Mutex};

use definitions::{BinaryDefinition, Scalar, Table};

use iced::widget::{pane_grid, PaneGrid};
use iced::Task;
use views::map_nav::MapNav;
use views::table::TableView;
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
pub enum Pane {
    Table(TableView),
    Nav(MapNav),
}

// TODO: use internal IDs instead of filenames
// TODO: move binary to ram for editing, write to different filename
//       avoid keeping files open longer than necessary?
pub struct App {
    /// Definitions, mapped to their names
    definition: BinaryDefinition,
    /// Binaries, mapped to their names and corresponding definition
    binary: FileGuard,
    panes: pane_grid::State<Pane>,
}

impl App {
    fn new(bin: File, def: definitions::BinaryDefinition) -> Self {
        let mut nav = MapNav::default();
        nav.tables = def.tables.clone();
        nav.scalars = def.scalars.clone();
        let (panes, _) = pane_grid::State::new(Pane::Nav(nav));
        Self {
            definition: def,
            binary: FileGuard::from(bin),
            panes,
        }
    }
    fn view(&self) -> PaneGrid<Message> {
        pane_grid(&self.panes, |_state, pane, _| {
            pane_grid::Content::new(match pane {
                Pane::Table(v) => iced::Element::from(v.view()),
                Pane::Nav(m) => m.view().into(),
            })
        })
    }
    fn update(&mut self, message: Message) {
        match message {
            Message::OpenTable(t) => {
                self.panes.split(
                    pane_grid::Axis::Vertical,
                    self.panes.iter().last().unwrap().0.clone(),
                    Pane::Table(TableView::new(t, self.binary.clone())),
                );
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    OpenTable(Table),
    OpenScalar(Scalar),
    EditCell(Table, FileGuard),
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
