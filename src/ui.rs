use color_eyre::owo_colors::AnsiColors::Blue;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style, Styled, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListDirection, Paragraph, Tabs};

pub fn render(frame: &mut Frame) {
    let area = frame.area();

    let bgblue = Color::Rgb(8, 11, 28);
    let footergray = Color::Rgb(23, 35, 75);

    frame.render_widget(Block::default().style(Style::new().bg(bgblue)), area);

    let tabList = Tabs::new(vec![
        "[1] VELVET ROOM".white(),
        "[2] CALENDAR".white(),
        "[3] TASKS".white(),
        "[4] MUSIC".white(),
    ])
    .block(Block::default())
    .divider(" ")
    .style(Style::new().white())
    .highlight_style(Style::new().underlined().underline_color(Color::Blue));

    let footer_entries: Vec<Line> = vec![
        ("1-4", " tab"),
        ("j/k", " move"),
        ("enter", " open"),
        ("space", " play/pause"),
        ("q", " quit"),
    ]
    .into_iter()
    .map(|(key, label)| {
        Line::from(vec![
            Span::styled(key, Style::new().bold().yellow()),
            Span::styled(label, Style::new().dark_gray()),
        ])
    })
    .collect();

    let footerList = Tabs::new(footer_entries)
        .block(Block::default())
        .divider(" ")
        .style(Style::new().bg(footergray))
        .highlight_style(Style::default());

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
    frame.render_widget(tabList, rows[1]);
    // frame.render_widget(Block::default().borders(Borders::ALL), rows[1]);
    // frame.render_widget(
    //     Block::default().borders(Borders::ALL).title("week"),
    //     rows[2],
    // );
    // frame.render_widget(
    //     Block::default().borders(Borders::ALL).title("status/music"),
    //     rows[3],
    // );
    frame.render_widget(footerList, rows[4]);
}
