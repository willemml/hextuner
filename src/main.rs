#![feature(iterator_try_collect)]
#![feature(iter_map_windows)]

use std::collections::HashMap;
use std::fs::File;
use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use anyhow::bail;
use definitions::{Scalar, Table};

use iced::widget::pane_grid;
use iced::{Element, Task};
use rfd::FileDialog;
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

macro_rules! get_pane_content {
    ($type:ident, $app:ident, $pane:ident) => {{
        let pane = $app
            .pane_id_map
            .get(&$pane)
            .ok_or(anyhow!("Fatal: Pane ID not in map"))?;
        if let PaneContent::$type(content) = &mut $app
            .panes
            .get_mut(*pane)
            .ok_or(anyhow!("Fatal: Pane has been deleted"))?
            .content
        {
            content
        } else {
            bail!("Fatal: Wrong pane")
        }
    }};
}

macro_rules! write_table_axis {
    ($axis:expr, $data:expr, $file:expr) => {{
        if $axis.writeable() {
            $axis.write(&mut $file, $data.map(|s| s.parse()).try_collect()?)?;
        }
    }};
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
        if let Err(e) = self.try_update(message) {
            let pane = views::panes::open(self, Open::Error(e.to_string()), self.binary.clone())
                .expect("Failed to display error message!");
            self.panes.maximize(pane);
        }
    }
    fn try_update(&mut self, message: Message) -> anyhow::Result<()> {
        match message {
            Message::Open(kind) => {
                views::panes::open(self, kind, self.binary.clone());
            }
            Message::EditCell {
                value,
                pane,
                source,
            } => {
                let table_view = get_pane_content!(Table, self, pane);
                match source {
                    EditSource::YHead(n) => table_view.y_head[n] = value,
                    EditSource::XHead(n) => table_view.x_head[n] = value,
                    EditSource::Data(n) => table_view.data[n] = value,
                }
            }

            Message::WriteTable { pane } => {
                let table_view = get_pane_content!(Table, self, pane);
                write_table_axis!(
                    table_view.table.x,
                    table_view.x_head.iter(),
                    table_view.source
                );
                write_table_axis!(
                    table_view.table.y,
                    table_view.y_head.iter(),
                    table_view.source
                );
                write_table_axis!(
                    table_view.table.z,
                    table_view.data.iter(),
                    table_view.source
                );
            }
            Message::EditScalar { value, pane } => {
                let scalar_view = get_pane_content!(Scalar, self, pane);
                scalar_view.value = value;
            }
            Message::WriteScalar { pane } => {
                let scalar_view = get_pane_content!(Scalar, self, pane);
                scalar_view
                    .scalar
                    .write(&mut scalar_view.source, scalar_view.value.parse()?)?;
            }
            Message::PaneAction(action) => views::panes::update_panes(self, action),
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Open {
    // Nav(BinaryDefinition),
    Table(Table),
    Scalar(Scalar),
    Error(String),
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
    // let xdf_path = FileDialog::new()
    //     .add_filter("XDF", &["xdf"])
    //     .set_directory("/")
    //     .pick_file()
    //     .unwrap();

    let xdf = File::open("testfiles/8E0909518AK_368072_TylerW.xdf").unwrap();

    let xdf_parsed = parse_buffer(xdf).unwrap().unwrap();

    // let bin_path = FileDialog::new()
    //     .add_filter("BIN", &["bin"])
    //     .set_directory("/")
    //     .pick_file()
    //     .unwrap();

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
