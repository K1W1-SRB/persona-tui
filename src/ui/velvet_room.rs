use super::moon::{self, Moon, SmallMoon};
use super::music;
use super::theme::{ACCENT, FAINT, MUTED, SELECTED_BG, TEXT, TRACK, URGENT};
use super::{content_area, divider};
use crate::app::{App, CategoryTypes, Task, ordered_task_indices};
use crate::cover::{self, CoverImage};
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

/// Week strip moons are 2 braille characters on one line.
const SMALL_MOON_WIDTH: u16 = 2;

const HEADER_DATE_FORMAT: &str = "%A · %d %b · %H:%M";

/// Completed tasks needed to go up one level in a stat.
const XP_PER_LEVEL: usize = 3;


pub fn render(frame: &mut Frame, app: &App, rows: &[Rect]) {
    let now = Local::now().naive_local();
    let today = now.date();

    let content = content_area(rows);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // header: date + next deadline | moon
            Constraint::Length(1), // gap
            Constraint::Length(1), // divider
            Constraint::Length(5), // week strip
            Constraint::Length(1), // divider
            Constraint::Length(1), // gap
            Constraint::Min(0),    // status | now playing
        ])
        .split(content);

    //Header
    let header_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(12)])
        .split(sections[0]);

    render_header(frame, app, now, header_cols[0]);
    frame.render_widget(
        Moon::new(moon::deadline_fill(&app.tasks, today)),
        header_cols[1],
    );

    frame.render_widget(divider(), sections[2]);

    //Week
    let day_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 7); 7])
        .split(sections[3]);

    for (date, day_area) in week_dates(today).into_iter().zip(day_cols.iter()) {
        render_day(frame, app, date, date == today, *day_area);
    }

    frame.render_widget(divider(), sections[4]);

    //Status / Now Playing
    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Fill(1)])
        .split(sections[6]);

    render_status(frame, &app.tasks, bottom_cols[0]);
    render_now_playing(frame, app, bottom_cols[1]);

    frame.render_widget(super::footer(app.active_tab), rows[4]);
}

fn render_header(frame: &mut Frame, app: &App, now: NaiveDateTime, area: Rect) {
    let date_line = Line::from(Span::styled(
        now.format(HEADER_DATE_FORMAT).to_string().to_uppercase(),
        Style::new().fg(MUTED),
    ));

    let next = ordered_task_indices(&app.tasks)
        .first()
        .map(|&i| &app.tasks[i])
        .filter(|task| !task.completed);

    let (title_line, due_line) = match next {
        Some(task) => {
            let (label, color) = due_label(task.due_date.date(), now.date());
            (
                Line::from(Span::styled(
                    task.title.clone(),
                    Style::new().fg(TEXT).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(label, Style::new().fg(color))),
            )
        }
        None => (
            Line::from(Span::styled("nothing due", Style::new().fg(MUTED))),
            Line::from(""),
        ),
    };

    let lines = vec![
        date_line,
        Line::from(""),
        Line::from(Span::styled("NEXT DEADLINE", Style::new().fg(FAINT))),
        title_line,
        due_line,
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

/// "tomorrow · 1 day left", plus the colour it should be shown in.
fn due_label(due: NaiveDate, today: NaiveDate) -> (String, Color) {
    let days = (due - today).num_days();
    match days {
        i64::MIN..=-1 => (format!("overdue · {} late", plural_days(-days)), URGENT),
        0 => ("today · due now".to_string(), URGENT),
        1 => ("tomorrow · 1 day left".to_string(), URGENT),
        _ => (
            format!(
                "{} · {} left",
                due.format("%A").to_string().to_lowercase(),
                plural_days(days)
            ),
            MUTED,
        ),
    }
}

fn plural_days(n: i64) -> String {
    if n == 1 {
        "1 day".to_string()
    } else {
        format!("{n} days")
    }
}

/// Monday to Sunday of the week containing `today`.
fn week_dates(today: NaiveDate) -> Vec<NaiveDate> {
    let monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    (0..7).map(|i| monday + Duration::days(i)).collect()
}

fn render_day(frame: &mut Frame, app: &App, date: NaiveDate, is_today: bool, area: Rect) {
    let label_style = if is_today {
        Style::new().fg(TEXT)
    } else {
        Style::new().fg(FAINT)
    };
    let number_style = if is_today {
        Style::new().fg(TEXT).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(MUTED)
    };

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            date.format("%a").to_string().to_uppercase(),
            label_style,
        )),
        // Zero-padded so it's 2 wide like the moon below; a single digit can't centre over it.
        Line::from(Span::styled(date.format("%d").to_string(), number_style)),
    ];

    let block = if is_today {
        Block::default().style(Style::new().bg(SELECTED_BG))
    } else {
        Block::default()
    };

    // Inset so the highlighted day doesn't touch its neighbours.
    let cell = area.inner(Margin::new(1, 0));

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(block),
        cell,
    );

    // Same lit/dark maths as the big moon, so both fill the same way. Drawn after
    // the paragraph so it sits on top of the today highlight.
    let moon_area = Rect {
        x: cell.x + cell.width.saturating_sub(SMALL_MOON_WIDTH) / 2,
        y: cell.y + 3,
        width: SMALL_MOON_WIDTH.min(cell.width),
        height: 1.min(cell.height.saturating_sub(3)),
    };
    frame.render_widget(
        SmallMoon::new(moon::deadline_fill(&app.tasks, date)),
        moon_area,
    );
}

fn render_status(frame: &mut Frame, tasks: &[Task], area: Rect) {
    let area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Max(36), Constraint::Min(0)])
        .split(area)[0];

    let stats = [
        ("KNOWLEDGE", CategoryTypes::Knowledge),
        ("KINDNESS", CategoryTypes::Kindness),
        ("GUTS", CategoryTypes::Guts),
    ];

    let mut lines = vec![
        Line::from(Span::styled("STATUS", Style::new().fg(MUTED))),
        Line::from(""),
    ];

    for (name, category) in stats {
        let (level, progress) = stat_level(tasks, category);
        let level_text = format!("lv {level}");
        let gap = (area.width as usize).saturating_sub(name.len() + level_text.len());

        lines.push(Line::from(vec![
            Span::styled(name, Style::new().fg(TEXT)),
            Span::raw(" ".repeat(gap)),
            Span::styled(level_text, Style::new().fg(MUTED)),
        ]));
        lines.push(progress_bar(area.width, progress));
        lines.push(Line::from(""));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

/// Level and progress (0.0..1.0) towards the next one, from completed tasks in `category`.
fn stat_level(tasks: &[Task], category: CategoryTypes) -> (usize, f64) {
    let xp = tasks
        .iter()
        .filter(|t| t.completed && t.category == category)
        .count();
    let level = xp / XP_PER_LEVEL + 1;
    let progress = (xp % XP_PER_LEVEL) as f64 / XP_PER_LEVEL as f64;
    (level, progress)
}

fn progress_bar(width: u16, progress: f64) -> Line<'static> {
    let width = width as usize;
    let filled = ((width as f64) * progress).round() as usize;
    Line::from(vec![
        Span::styled("━".repeat(filled), Style::new().fg(ACCENT)),
        Span::styled("━".repeat(width - filled), Style::new().fg(TRACK)),
    ])
}

/// Compact version of the Music tab's player: thumbnail, title and time.
fn render_now_playing(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(cover::SMALL_ROWS),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(Span::styled("NOW PLAYING", Style::new().fg(MUTED))),
        rows[0],
    );

    let track_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(cover::SMALL_COLS),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(rows[2]);

    let playing = app
        .music
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.now_playing.as_ref().map(|item| (snapshot, item)));

    let Some((snapshot, item)) = playing else {
        frame.render_widget(
            Paragraph::new(Span::styled(
                music::status_message(&app.music),
                Style::new().fg(FAINT),
            ))
            .wrap(Wrap { trim: true }),
            Rect {
                height: area.height.saturating_sub(2),
                ..rows[2]
            },
        );
        return;
    };

    // Real album art once downloaded, generated art until then.
    match &item.cover {
        Some(cover) => frame.render_widget(CoverImage::new(&cover.small), track_cols[0]),
        None => frame.render_widget(music::CoverArt::new(&item.title), track_cols[0]),
    }

    let elapsed = snapshot.current_progress_secs().max(0) as u32;
    let duration = item.duration_secs.max(0) as u32;
    let mut time = vec![Span::styled(
        format!(
            "{} / {}",
            music::format_time(elapsed),
            music::format_time(duration)
        ),
        Style::new().fg(FAINT),
    )];
    if !snapshot.is_playing {
        time.push(Span::styled(" · paused", Style::new().fg(FAINT)));
    }

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(item.title.as_str(), Style::new().fg(TEXT))),
            Line::from(time),
        ]),
        track_cols[2],
    );
}
