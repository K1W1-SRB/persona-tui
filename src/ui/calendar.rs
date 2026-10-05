use super::moon::{self, SmallMoon};
use super::theme::{ACCENT, FAINT, MUTED, SELECTED_BG, TEXT};
use super::{FOOTER_GRAY, content_area, divider};
use crate::app::{App, Task};
use chrono::{Datelike, Local, Months, NaiveDate};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

const WEEKDAYS: [&str; 7] = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];

/// Each week row is a day number line plus a moon line.
const WEEK_HEIGHT: u16 = 2;
/// Blank lines between week rows.
const WEEK_GAP: u16 = 1;

/// Day-of-month moons are 2 braille characters wide.
const MOON_WIDTH: u16 = 2;

/// Most tasks listed under the grid for the selected day.
const MAX_LISTED_TASKS: u16 = 6;

pub fn render(frame: &mut Frame, app: &App, rows: &[Rect]) {
    let today = Local::now().date_naive();
    let selected = app.calendar_selected;
    let month = MonthGrid::new(selected);

    let day_tasks = tasks_on(&app.tasks, selected);
    let list_height = (day_tasks.len() as u16).clamp(1, MAX_LISTED_TASKS);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),                         // month title | prev · next
            Constraint::Length(1),                         // gap
            Constraint::Length(1),                         // divider
            Constraint::Length(1),                         // weekday labels
            Constraint::Length(1),                         // gap
            Constraint::Length(month.grid_height()),       // grid
            Constraint::Length(1),                         // divider
            Constraint::Length(1),                         // gap
            Constraint::Length(1),                         // "SELECTED — SEP 4"
            Constraint::Min(list_height),                  // that day's tasks
        ])
        .split(content_area(rows));

    render_title(frame, selected, sections[0]);
    frame.render_widget(divider(), sections[2]);
    render_weekday_labels(frame, sections[3]);
    render_grid(frame, app, &month, today, sections[5]);
    frame.render_widget(divider(), sections[6]);

    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("SELECTED — {}", selected.format("%b %-d")).to_uppercase(),
            Style::new().fg(FAINT),
        )),
        sections[8],
    );
    render_day_tasks(frame, &day_tasks, sections[9]);

    match &app.new_task_draft {
        Some(draft) => super::tasks::render_new_task_prompt(frame, rows[4], draft, FOOTER_GRAY),
        None => frame.render_widget(super::footer(app.active_tab), rows[4]),
    }
}

/// The month containing a date, laid out as Monday-first weeks.
struct MonthGrid {
    first: NaiveDate,
    days: u32,
    /// Empty cells before the 1st (0 when the month starts on a Monday).
    offset: u32,
    weeks: u16,
}

impl MonthGrid {
    fn new(date: NaiveDate) -> Self {
        let first = date.with_day(1).unwrap();
        let next_first = first + Months::new(1);
        let days = (next_first - first).num_days() as u32;
        let offset = first.weekday().num_days_from_monday();
        let weeks = (offset + days).div_ceil(7) as u16;
        Self {
            first,
            days,
            offset,
            weeks,
        }
    }

    fn grid_height(&self) -> u16 {
        self.weeks * WEEK_HEIGHT + (self.weeks - 1) * WEEK_GAP
    }

    /// Date shown in a grid cell, or None for the blank cells around the month.
    fn date_at(&self, week: u16, col: u16) -> Option<NaiveDate> {
        let index = week as u32 * 7 + col as u32;
        let day = index.checked_sub(self.offset)? + 1;
        if day > self.days {
            return None;
        }
        self.first.with_day(day)
    }
}

fn render_title(frame: &mut Frame, selected: NaiveDate, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Fill(1)])
        .split(area);

    frame.render_widget(
        Paragraph::new(Span::styled(
            selected.format("%B %Y").to_string().to_uppercase(),
            Style::new().fg(TEXT).add_modifier(Modifier::BOLD),
        )),
        cols[0],
    );
    // The brackets double as the keys: [ for previous month, ] for next.
    frame.render_widget(
        Paragraph::new(Span::styled("[ prev · next ]", Style::new().fg(FAINT)))
            .alignment(Alignment::Right),
        cols[1],
    );
}

fn day_columns(area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 7); 7])
        .split(area)
}

fn render_weekday_labels(frame: &mut Frame, area: Rect) {
    for (label, col) in WEEKDAYS.iter().zip(day_columns(area).iter()) {
        frame.render_widget(
            Paragraph::new(Span::styled(*label, Style::new().fg(FAINT)))
                .alignment(Alignment::Center),
            col.inner(Margin::new(1, 0)),
        );
    }
}

fn render_grid(frame: &mut Frame, app: &App, month: &MonthGrid, today: NaiveDate, area: Rect) {
    let week_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(WEEK_HEIGHT); month.weeks as usize])
        .spacing(WEEK_GAP)
        .split(area);

    for (week, week_area) in week_rows.iter().enumerate() {
        for (col, cell_area) in day_columns(*week_area).iter().enumerate() {
            if let Some(date) = month.date_at(week as u16, col as u16) {
                render_cell(frame, app, date, today, *cell_area);
            }
        }
    }
}

fn render_cell(frame: &mut Frame, app: &App, date: NaiveDate, today: NaiveDate, area: Rect) {
    let is_selected = date == app.calendar_selected;
    let is_today = date == today;

    let number_style = if is_selected {
        Style::new().fg(TEXT).add_modifier(Modifier::BOLD)
    } else if is_today {
        Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(TEXT)
    };

    let block = if is_selected {
        Block::default().style(Style::new().bg(SELECTED_BG))
    } else {
        Block::default()
    };

    // Inset so the selected box doesn't touch its neighbours.
    let cell = area.inner(Margin::new(1, 0));

    // Zero-padded so it's 2 wide like the moon below; a single digit can't centre over it.
    frame.render_widget(
        Paragraph::new(Span::styled(date.format("%d").to_string(), number_style))
            .alignment(Alignment::Center)
            .block(block),
        cell,
    );

    let has_open_deadline = app
        .tasks
        .iter()
        .any(|t| !t.completed && t.due_date.date() == date);

    if has_open_deadline && cell.height > 1 {
        let moon_area = Rect {
            x: cell.x + cell.width.saturating_sub(MOON_WIDTH) / 2,
            y: cell.y + 1,
            width: MOON_WIDTH.min(cell.width),
            height: 1,
        };
        frame.render_widget(
            SmallMoon::new(moon::proximity_fill(date, today)),
            moon_area,
        );
    }
}

/// Tasks due on `date`, open ones first, each group in due-time order.
fn tasks_on(tasks: &[Task], date: NaiveDate) -> Vec<&Task> {
    let mut day: Vec<&Task> = tasks.iter().filter(|t| t.due_date.date() == date).collect();
    day.sort_by_key(|t| (t.completed, t.due_date));
    day
}

fn render_day_tasks(frame: &mut Frame, tasks: &[&Task], area: Rect) {
    if tasks.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("nothing due", Style::new().fg(FAINT))),
            area,
        );
        return;
    }

    let lines: Vec<Line> = tasks
        .iter()
        .map(|task| {
            let (dot, title) = if task.completed {
                (
                    Style::new().fg(FAINT),
                    Style::new().fg(FAINT).add_modifier(Modifier::CROSSED_OUT),
                )
            } else {
                (Style::new().fg(ACCENT), Style::new().fg(TEXT))
            };
            Line::from(vec![
                Span::styled("● ", dot),
                Span::styled(task.title.clone(), title),
                Span::styled(
                    format!("  {}", task.due_date.format("%H:%M")),
                    Style::new().fg(MUTED),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn september_2026_starts_on_tuesday_and_spans_five_weeks() {
        let grid = MonthGrid::new(date(2026, 9, 17));
        assert_eq!(grid.offset, 1);
        assert_eq!(grid.days, 30);
        assert_eq!(grid.weeks, 5);
        assert_eq!(grid.date_at(0, 0), None);
        assert_eq!(grid.date_at(0, 1), Some(date(2026, 9, 1)));
        assert_eq!(grid.date_at(4, 2), Some(date(2026, 9, 30)));
        assert_eq!(grid.date_at(4, 3), None);
    }

    #[test]
    fn handles_february_and_six_week_months() {
        assert_eq!(MonthGrid::new(date(2028, 2, 1)).days, 29);
        // August 2026 starts on a Saturday and has 31 days.
        assert_eq!(MonthGrid::new(date(2026, 8, 1)).weeks, 6);
    }
}
