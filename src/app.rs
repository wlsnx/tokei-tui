use std::{
    cmp,
    collections::HashMap,
    io::{self, Read},
    iter::once,
    path::Path,
};

use crate::utils::{longest_common_prefix, parse, print_languages};
use ansi_to_tui::IntoText;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use tokei::Languages;

use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::Text,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

pub fn run<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s)?;

    let mut languages = Languages::new();
    let parsed = parse(&s)?;
    languages.extend(parsed.languages);

    let mut list_files = false;
    let mut compact = false;

    let files = languages
        .iter()
        .flat_map(|(_, lan)| lan.reports.iter().map(|report| report.name.as_path()))
        .collect();
    let prefix = longest_common_prefix(&files);
    let mut root = prefix.as_path();
    let paths = path_map(files);
    let mut state = ListState::default();
    let mut offset = 0;

    loop {
        let children = &paths[&root];
        let curdir = match state.selected() {
            Some(0) => root,
            Some(n) => children[n - 1],
            None => {
                state.select(Some(0));
                root
            }
        };
        let height = terminal.size()?.height - 2;

        terminal.draw(|f| {
            ui(
                f,
                &languages,
                &curdir,
                &children,
                &mut state,
                &mut offset,
                list_files,
                compact,
            )
        })?;

        match event::read()? {
            Event::Key(key) => match (key.code, key.modifiers) {
                (KeyCode::Char('c'), KeyModifiers::NONE) => compact = !compact,

                (KeyCode::Char('f'), KeyModifiers::NONE) => list_files = !list_files,
                (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                    offset = offset.saturating_add(height);
                }
                (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                    offset = offset.saturating_sub(height);
                }

                (KeyCode::Char('j'), KeyModifiers::NONE) => {
                    let n = cmp::min(children.len(), state.selected().unwrap().saturating_add(1));
                    state.select(Some(n));
                    offset = 0;
                }
                (KeyCode::Char('J'), KeyModifiers::SHIFT) => state.select(Some(children.len())),

                (KeyCode::Char('k'), KeyModifiers::NONE) => {
                    let n = state.selected().unwrap().saturating_sub(1);
                    state.select(Some(n));
                    offset = 0;
                }
                (KeyCode::Char('K'), KeyModifiers::SHIFT) => state.select(Some(0)),

                (KeyCode::Char('h'), KeyModifiers::NONE) => {
                    if root != prefix {
                        root = root.parent().unwrap();
                        state.select(Some(0));
                    }
                }
                (KeyCode::Char('l'), KeyModifiers::NONE) => {
                    let n = state.selected().unwrap();
                    if n != 0 {
                        let new_root = children[n - 1];
                        if paths.contains_key(new_root) {
                            root = children[n - 1];
                            state.select(Some(0));
                        }
                    }
                }

                (KeyCode::Char('g'), KeyModifiers::NONE) => offset = 0,
                (KeyCode::Char('G'), KeyModifiers::SHIFT) => offset = u16::MAX,

                (KeyCode::Char('q'), KeyModifiers::NONE) => break,
                _ => (),
            },
            _ => (),
        }
    }
    Ok(())
}

fn path_map(dirs: Vec<&Path>) -> HashMap<&Path, Vec<&Path>> {
    let mut map = HashMap::new();
    for dir in dirs.iter() {
        for ancestor in dir.ancestors() {
            if let Some(parent) = ancestor.parent() {
                let entry = map.entry(parent).or_insert(vec![]);
                if !entry.contains(&ancestor) {
                    entry.push(ancestor);
                }
            }
        }
    }
    for (_, paths) in map.iter_mut() {
        paths.sort();
    }
    map
}

fn ui<B: Backend>(
    f: &mut Frame<B>,
    languages: &Languages,
    curdir: &Path,
    paths: &Vec<&Path>,
    state: &mut ListState,
    offset: &mut u16,
    list_files: bool,
    compact: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(f.size());

    let items = once(ListItem::new(Text::raw(".")))
        .chain(
            paths
                .iter()
                .map(|path| ListItem::new(Text::raw(path.file_name().unwrap().to_str().unwrap()))),
        )
        .collect::<Vec<_>>();

    let list = List::new(items)
        .block(Block::default().title("Files").borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_stateful_widget(list, chunks[0], state);

    let output = print_languages(
        &languages,
        curdir,
        list_files,
        compact,
        chunks[1].width.into(),
    )
    .unwrap();
    let text = output.into_text().unwrap();

    let height = f.size().height - 2;
    let max_offset = (text.lines.len() as u16).saturating_sub(height);
    *offset = cmp::min(*offset, max_offset);

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(curdir.to_str().unwrap())
                .borders(Borders::ALL),
        )
        .scroll((*offset, 0))
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, chunks[1]);
}
