mod calendar;
mod moon;
mod music;
mod tasks;
mod theme;
mod velvet_room;

use crate::app::{App, Tab};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Tabs};

const BG_BLUE: Color = Color::Rgb(8, 11, 28);
const FOOTER_GRAY: Color = Color::Rgb(23, 35, 75);

impl Tab {
    fn index(&self) -> usize {
        match self {
            Tab::VelvetRoom => 0,
            Tab::Calendar => 1,
            Tab::Tasks => 2,
            Tab::Music => 3,
        }
    }
}

pub fn render(frame: &mut Frame, app: &App) {
    // Reusable Components
    let area = frame.area();

    frame.render_widget(Block::default().style(Style::new().bg(BG_BLUE)), area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(tab_list(app.active_tab), rows[1]);

    //Rendering Tabs
    match app.active_tab {
        Tab::VelvetRoom => velvet_room::render(frame, app, &rows),
        Tab::Calendar => calendar::render(frame, app, &rows),
        Tab::Tasks => tasks::render(frame, app, &rows),
        Tab::Music => music::render(frame, app, &rows),
    }
}

fn tab_list(active_tab: Tab) -> Tabs<'static> {
    Tabs::new(vec![
        "[1] VELVET ROOM".white(),
        "[2] CALENDAR".white(),
        "[3] TASKS".white(),
        "[4] MUSIC".white(),
    ])
    .block(Block::default())
    .divider(" ")
    .style(Style::new().white())
    .highlight_style(Style::new().underlined().underline_color(Color::Blue))
    .select(active_tab.index())
}

/// Key hints for the active tab, between the shared "tab" and "quit" entries.
fn footer(active_tab: Tab) -> Tabs<'static> {
    let tab_keys: &[(&'static str, &'static str)] = match active_tab {
        Tab::Calendar => &[
            ("h/l", " day"),
            ("j/k", " week"),
            ("[/]", " month"),
            ("t", " today"),
            ("n", " new"),
        ],
        Tab::Tasks => &[("j/k", " move"), ("space", " toggle"), ("n", " new")],
        Tab::VelvetRoom | Tab::Music => &[],
    };

    let footer_entries: Vec<Line> = std::iter::once(("1-4", " tab"))
        .chain(tab_keys.iter().copied())
        .chain(std::iter::once(("q", " quit")))
        .map(|(key, label)| {
            Line::from(vec![
                Span::styled(key, Style::new().bold().yellow()),
                Span::styled(label, Style::new().dark_gray()),
            ])
        })
        .collect();

    Tabs::new(footer_entries)
        .block(Block::default())
        .divider(" ")
        .style(Style::new().bg(FOOTER_GRAY))
        .highlight_style(Style::default())
}

fn content_area(rows: &[Rect]) -> Rect {
    let below_tabs = rows[1].union(rows[3]);
    let below_tabs = Rect {
        y: below_tabs.y + 1,
        height: below_tabs.height.saturating_sub(1),
        ..below_tabs
    };
    Block::default()
        .padding(Padding::new(2, 2, 1, 0))
        .inner(below_tabs)
}

fn divider() -> Block<'static> {
    Block::default()
        .borders(Borders::TOP)
        .border_style(Style::new().fg(theme::DIVIDER))
}
