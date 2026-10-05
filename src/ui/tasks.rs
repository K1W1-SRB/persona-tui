use super::FOOTER_GRAY;
use crate::app::{
    App, CategoryTypes, NewTaskDraft, NewTaskStage, Task, completed_tasks, date_grouping_tasks,
};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

const SELECTED_BG: Color = Color::Rgb(35, 45, 90);

pub fn render(frame: &mut Frame, app: &App, rows: &[Rect]) {
    let tasks = &app.tasks;

    let no_open = tasks.iter().filter(|t| !t.completed).count();
    let no_done = tasks.iter().filter(|t| t.completed).count();

    let task_header_left = Line::from(Span::styled(
        "TASKS",
        Style::new().white().add_modifier(Modifier::BOLD),
    ));

    let task_header_right = Line::from(vec![
        Span::styled(format!("{no_open} open"), Style::new().gray()),
        Span::styled(" · ", Style::new().dark_gray()),
        Span::styled(format!("{no_done} done"), Style::new().dark_gray()),
    ])
    .alignment(Alignment::Right);

    //Header
    let task_header_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(rows[2]);

    let task_header_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::new().blue());
    let task_header_inner = task_header_block.inner(task_header_area[0]);
    frame.render_widget(task_header_block, task_header_area[0]);

    let task_header_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Fill(1)])
        .split(task_header_inner);

    frame.render_widget(task_header_left, task_header_cols[0]);
    frame.render_widget(task_header_right, task_header_cols[1]);

    //List
    let list_area = Rect {
        x: rows[3].x,
        y: task_header_area[1].y,
        width: rows[3].width,
        height: task_header_area[1].height + rows[3].height,
    };
    render_task_list(frame, list_area, tasks, app.selected);

    match &app.new_task_draft {
        Some(draft) => render_new_task_prompt(frame, rows[4], draft, FOOTER_GRAY),
        None => frame.render_widget(super::footer(app.active_tab), rows[4]),
    }
}

enum TaskRow {
    Header {
        label: &'static str,
        color: Color,
        count: usize,
    },
    Divider,
    Open {
        index: usize,
        row_number: usize,
        bar_color: Option<Color>,
    },
    Done {
        index: usize,
        row_number: usize,
    },
}

fn build_task_rows(tasks: &[Task]) -> Vec<TaskRow> {
    let groups = date_grouping_tasks(tasks);
    let done = completed_tasks(tasks);
    let mut rows = Vec::new();
    let mut row_number = 0;

    if !groups.today.is_empty() {
        rows.push(TaskRow::Header {
            label: "TODAY",
            color: Color::Rgb(235, 80, 80),
            count: groups.today.len(),
        });
        for index in groups.today {
            rows.push(TaskRow::Open {
                index,
                row_number,
                bar_color: Some(Color::Rgb(210, 60, 60)),
            });
            row_number += 1;
        }
    }

    if !groups.this_week.is_empty() {
        rows.push(TaskRow::Header {
            label: "THIS WEEK",
            color: Color::Rgb(205, 120, 140),
            count: groups.this_week.len(),
        });
        for index in groups.this_week {
            rows.push(TaskRow::Open {
                index,
                row_number,
                bar_color: Some(Color::Rgb(70, 110, 220)),
            });
            row_number += 1;
        }
    }

    if !groups.later.is_empty() {
        rows.push(TaskRow::Header {
            label: "LATER",
            color: Color::Gray,
            count: groups.later.len(),
        });
        for index in groups.later {
            rows.push(TaskRow::Open {
                index,
                row_number,
                bar_color: None,
            });
            row_number += 1;
        }
    }

    if !done.is_empty() {
        rows.push(TaskRow::Divider);
        rows.push(TaskRow::Header {
            label: "COMPLETED",
            color: Color::Rgb(120, 170, 255),
            count: done.len(),
        });
        for index in done {
            rows.push(TaskRow::Done { index, row_number });
            row_number += 1;
        }
    }

    rows
}

fn category_style(category: &CategoryTypes) -> (&'static str, Color) {
    match category {
        CategoryTypes::Knowledge => ("KNOWLEDGE", Color::Rgb(35, 70, 130)),
        CategoryTypes::Kindness => ("KINDNESS", Color::Rgb(35, 95, 55)),
        CategoryTypes::Guts => ("GUTS", Color::Rgb(100, 70, 150)),
    }
}

fn render_task_list(frame: &mut Frame, area: Rect, tasks: &[Task], selected: usize) {
    let task_rows = build_task_rows(tasks);

    let mut constraints: Vec<Constraint> = task_rows
        .iter()
        .map(|row| match row {
            TaskRow::Open { .. } => Constraint::Length(2),
            TaskRow::Header { .. } | TaskRow::Divider | TaskRow::Done { .. } => {
                Constraint::Length(1)
            }
        })
        .collect();
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (row, chunk) in task_rows.iter().zip(chunks.iter()) {
        match row {
            TaskRow::Header { label, color, count } => {
                let line = Line::from(vec![
                    Span::styled(*label, Style::new().fg(*color).add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" · {count}"), Style::new().dark_gray()),
                ]);
                frame.render_widget(Paragraph::new(line), *chunk);
            }
            TaskRow::Divider => {
                frame.render_widget(
                    Block::default()
                        .borders(Borders::BOTTOM)
                        .border_style(Style::new().blue()),
                    *chunk,
                );
            }
            TaskRow::Open {
                index,
                row_number,
                bar_color,
            } => {
                render_open_task(frame, *chunk, &tasks[*index], *bar_color, *row_number == selected);
            }
            TaskRow::Done { index, row_number } => {
                let task = &tasks[*index];
                let marker = if *row_number == selected { ">" } else { " " };
                let style = if *row_number == selected {
                    Style::new().bg(SELECTED_BG).dark_gray()
                } else {
                    Style::new().dark_gray()
                };
                let line = Line::from(Span::styled(
                    format!("{marker} {}", task.title),
                    style.add_modifier(Modifier::CROSSED_OUT),
                ));
                frame.render_widget(Paragraph::new(line).style(style), *chunk);
            }
        }
    }
}

fn render_open_task(frame: &mut Frame, area: Rect, task: &Task, bar_color: Option<Color>, selected: bool) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    if let Some(color) = bar_color {
        frame.render_widget(Block::default().style(Style::new().bg(color)), cols[0]);
    }

    let row_bg = if selected { SELECTED_BG } else { Color::Reset };
    frame.render_widget(Block::default().style(Style::new().bg(row_bg)), cols[1]);

    let lines = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(cols[1]);

    let title_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Length(12)])
        .split(lines[0]);

    let (tag_label, tag_color) = category_style(&task.category);
    let marker = if selected { "›" } else { "•" };

    let title_line = Line::from(vec![
        Span::styled(format!(" {marker} "), Style::new().bg(row_bg).gray()),
        Span::styled(
            &task.title,
            Style::new().bg(row_bg).white().add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(title_line), title_row[0]);

    let tag_line = Line::from(Span::styled(
        format!(" {tag_label} "),
        Style::new()
            .bg(tag_color)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ))
    .alignment(Alignment::Right);
    frame.render_widget(Paragraph::new(tag_line), title_row[1]);

    let date_part = task.due_date.format("%a %d %b").to_string().to_lowercase();
    let due_text = if task.due_date.time() == chrono::NaiveTime::MIN {
        format!("   {date_part}")
    } else {
        format!("   {date_part} {}", task.due_date.format("%H:%M"))
    };
    let due_line = Line::from(Span::styled(due_text, Style::new().bg(row_bg).dark_gray()));
    frame.render_widget(Paragraph::new(due_line), lines[1]);
}

pub(super) fn render_new_task_prompt(frame: &mut Frame, area: Rect, draft: &NewTaskDraft, bg: Color) {
    let (category_label, category_color) = category_style(&draft.category);

    let line = match draft.stage {
        NewTaskStage::Title => Line::from(vec![
            Span::styled(" new task: ", Style::new().bg(bg).bold().yellow()),
            Span::styled(format!("{}_", draft.title), Style::new().bg(bg).white()),
            Span::styled(
                format!("  [{category_label}]"),
                Style::new().bg(category_color).fg(Color::White).bold(),
            ),
            Span::styled(
                "  tab category · enter next · esc cancel",
                Style::new().bg(bg).dark_gray(),
            ),
        ]),
        NewTaskStage::DueDate => {
            let mut spans = vec![
                Span::styled(" due date: ", Style::new().bg(bg).bold().yellow()),
                Span::styled(format!("{}_", draft.date_input), Style::new().bg(bg).white()),
                Span::styled(
                    "  e.g. 11/10/2005 1400",
                    Style::new().bg(bg).dark_gray(),
                ),
            ];
            if let Some(error) = &draft.error {
                spans.push(Span::styled(
                    format!("  {error}"),
                    Style::new().bg(bg).red().bold(),
                ));
            } else {
                spans.push(Span::styled(
                    "  enter confirm · esc cancel",
                    Style::new().bg(bg).dark_gray(),
                ));
            }
            Line::from(spans)
        }
    };
    frame.render_widget(Paragraph::new(line).style(Style::new().bg(bg)), area);
}
