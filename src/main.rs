use std::fs::File;

use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use definitions::BinaryDefinition;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
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
    index: usize,
    current_const: (String, String),
    /// If true exit on next event loop
    exit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
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
                if self.index >= self.definition.constants.len() - 1 {
                    self.index = 0;
                } else {
                    self.index += 1;
                }
            }
            KeyCode::Right => {
                if self.index == 0 {
                    self.index = self.definition.constants.len() - 1;
                } else {
                    self.index -= 1;
                }
            }
            _ => {}
        }
        if !self.definition.constants.is_empty() {
            let constant = &self.definition.constants[self.index];
            self.current_const = (
                constant.name.clone(),
                constant.read(&mut self.binary)?.to_string(),
            );
        }
        Ok(())
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" HEXTuner ".bold());
        let instructions = Line::from(vec![
            " Previous Constant ".into(),
            "<Left>".blue().bold(),
            " Next Constant ".into(),
            "<Right>".blue().bold(),
            " Quit ".into(),
            "<Q>".blue().bold(),
        ]);

        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let value_text = Text::from(vec![Line::from(vec![
            (&self.current_const.0).into(),
            ": ".into(),
            self.current_const.1.clone().yellow(),
        ])]);

        Paragraph::new(value_text)
            .centered()
            .block(block)
            .render(area, buf)
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

    // let mut definitions = HashMap::new();
    // definitions.insert("def1".into(), def);

    // let mut binaries = HashMap::new();
    // binaries.insert("bin1".into(), ("def1".into(), bin));

    let mut app = App {
        definition: def,
        index: 0,
        binary: bin,
        current_const: ("None".into(), "N/A".into()),
        exit: false,
    };

    let mut terminal = ratatui::init();

    let result = app.run(&mut terminal);

    ratatui::restore();

    result

    // for constant in definitions.constants {
    //     constant
    //         .write(&mut new_file, constant.read(&mut stock_bin).unwrap())
    //         .unwrap();
    // }
}
