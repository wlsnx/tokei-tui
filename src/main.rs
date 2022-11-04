mod utils;

use crate::utils::{longest_common_prefix, parse, print_languages};
use anyhow::Result;
use std::{
    io::{self, Read},
    path::Path,
};
use tokei::Languages;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use ansi_to_tui::IntoText;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

/// abc
fn main() -> Result<()> {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s)?;

    let mut languages = Languages::new();
    let parsed = parse(&s)?;
    languages.extend(parsed.languages);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut list_files = false;
    let mut compact = false;

    let dirs = languages
        .iter()
        .flat_map(|(_, lan)| lan.reports.iter().map(|report| report.name.to_owned()))
        .collect();
    let root = longest_common_prefix(&dirs);

    loop {
        terminal.draw(|f| ui(f, &languages, &root, list_files, compact))?;

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('f') => list_files = !list_files,
                KeyCode::Char('c') => compact = !compact,
                _ => (),
            },
            _ => (),
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}

fn ui<B: Backend>(
    f: &mut Frame<B>,
    languages: &Languages,
    root: &Path,
    list_files: bool,
    compact: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(f.size());
    let s = print_languages(&languages, list_files, compact, chunks[1].width.into()).unwrap();
    let text = s.into_text().unwrap();
    let paragraph = Paragraph::new(text).block(
        Block::default()
            .title(root.to_str().unwrap())
            .borders(Borders::ALL),
    );
    f.render_widget(paragraph, chunks[1]);
}
