use std::fs::File;

use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use definitions::BinaryDefinition;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::Text,
    widgets::{Block, Borders, Paragraph, Row, Table, Widget},
    DefaultTerminal, Frame,
};

use xdftuneparser::data_types::XDFElement;
use xdftuneparser::parse_buffer;

pub mod definitions;
pub mod eval;

// TODO: use internal IDs instead of filenames
// TODO: move binary to ram for editing, write to different filename
//       avoid keeping files open longer than necessary?
pub struct App {
    /// Definitions, mapped to their names
    definition: BinaryDefinition,
    /// Binaries, mapped to their names and corresponding definition
    binary: File,
    const_index: usize,
    table_index: usize,
    current_const: (String, String),
    current_table: (String, Vec<Vec<String>>, usize),
    /// If true exit on next event loop
    exit: bool,
}

fn build_table(bin: &mut File, def: &definitions::Table) -> io::Result<Vec<Vec<String>>> {
    // add one to length for row/column headers
    let xl = def.x.len();
    let yl = def.y.len();

    // read rows headers into buffer with one cell of padding for column headers
    let mut buf = vec![0.0];
    buf.append(&mut def.x.read(bin)?);

    let mut buf: Vec<String> = buf.into_iter().map(|f| f.to_string()).collect();

    buf[0] = "".into();

    let row_head = def.y.read(bin)?;

    let mut data = def.z.read(bin)?;
    // reverse data so we can use pop to put it in the table in order correctly
    data.reverse();

    let mut table = Vec::new();
    table.push(buf.split_off(0));

    for y in 0..yl {
        // add the row "header"
        buf.push(row_head[y].to_string());

        for _ in 0..xl {
            if let Some(d) = data.pop() {
                buf.push(d.to_string());
            } else {
                break;
            }
        }

        table.push(buf.split_off(0));
    }

    Ok(table)
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.update()?;
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)?
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_key_event(&mut self, event: KeyEvent) -> io::Result<()> {
        match event.code {
            KeyCode::Char('q' | 'Q') => self.exit = true,
            KeyCode::Left => {
                if self.const_index >= self.definition.constants.len() - 1 {
                    self.const_index = 0;
                } else {
                    self.const_index += 1;
                }
            }
            KeyCode::Right => {
                if self.const_index == 0 {
                    self.const_index = self.definition.constants.len() - 1;
                } else {
                    self.const_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.table_index >= self.definition.tables.len() - 1 {
                    self.table_index = 0;
                } else {
                    self.table_index += 1;
                }
            }

            KeyCode::Up => {
                if self.table_index == 0 {
                    self.table_index = self.definition.tables.len() - 1;
                } else {
                    self.table_index -= 1;
                }
            }
            _ => {}
        }
        self.update()
    }
    fn update(&mut self) -> io::Result<()> {
        if !self.definition.constants.is_empty() {
            let constant = &self.definition.constants[self.const_index];
            self.current_const = (
                constant.name.clone(),
                constant.read(&mut self.binary)?.to_string(),
            );
        }

        if !self.definition.tables.is_empty() {
            let table = &self.definition.tables[self.table_index];
            self.current_table = (
                table.name.clone(),
                build_table(&mut self.binary, table)?,
                table.x.len() + 1,
            )
        }

        Ok(())
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(15),
            ])
            .split(area);

        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default());

        let title =
            Paragraph::new(Text::styled(" HEXTuner ", Style::default().bold())).block(title_block);

        let constant_block = Block::default()
            .title(self.current_const.0.clone())
            .borders(Borders::ALL)
            .style(Style::default());
        let constant =
            Paragraph::new(Text::from(self.current_const.1.clone().yellow())).block(constant_block);

        let rows = self.current_table.1.iter().map(|r| Row::new(r.clone()));

        let table_block = Block::default()
            .title(self.current_table.0.clone())
            .borders(Borders::ALL)
            .style(Style::default());
        let table =
            Table::new(rows, vec![Constraint::Length(5); self.current_table.2]).block(table_block);

        title.render(chunks[0], buf);
        constant.render(chunks[1], buf);
        table.render(chunks[2], buf);
    }
}

fn main() -> std::io::Result<()> {
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

    // let krkte_t = def
    //     .tables
    //     .iter()
    //     .find(|t| t.name == "KRKTE")
    //     .unwrap()
    //     .clone();
    // let krkte = krkte_t.z.read(&mut bin).unwrap();

    // dbg!(&krkte_t, krkte);

    // let mut definitions = HashMap::new();
    // definitions.insert("def1".into(), def);

    // let mut binaries = HashMap::new();
    // binaries.insert("bin1".into(), ("def1".into(), bin));

    let mut app = App {
        definition: def,
        const_index: 0,
        table_index: 0,
        binary: bin,
        current_const: ("None".into(), "N/A".into()),
        current_table: ("None".into(), Vec::new(), 0),
        exit: false,
    };

    let mut terminal = ratatui::init();

    let result = app.run(&mut terminal);

    ratatui::restore();

    result
    // Ok(())

    // for constant in definitions.constants {
    //     constant
    //         .write(&mut new_file, constant.read(&mut stock_bin).unwrap())
    //         .unwrap();
    // }
}
