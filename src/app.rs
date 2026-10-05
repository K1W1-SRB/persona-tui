use crate::spotify::{PlayerSnapshot, PlayerUpdate};
use crate::ui;
use chrono::{Days, Duration, Local, Months, NaiveDate, NaiveDateTime};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{DefaultTerminal, Frame};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::mpsc::{Receiver, TryRecvError};

const TASKS_FILE: &str = "tasks.json";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    VelvetRoom,
    Calendar,
    Tasks,
    Music,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum CategoryTypes {
    Knowledge,
    Kindness,
    Guts,
}

impl CategoryTypes {
    pub(crate) fn next(&self) -> CategoryTypes {
        match self {
            CategoryTypes::Knowledge => CategoryTypes::Kindness,
            CategoryTypes::Kindness => CategoryTypes::Guts,
            CategoryTypes::Guts => CategoryTypes::Knowledge,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Task {
    pub(crate) title: String,
    pub(crate) category: CategoryTypes,
    pub(crate) due_date: NaiveDateTime,
    pub(crate) completed: bool,
}

#[derive(Debug)]
pub(crate) struct TaskGroups {
    pub(crate) today: Vec<usize>,
    pub(crate) this_week: Vec<usize>,
    pub(crate) later: Vec<usize>,
}

pub(crate) fn date_grouping_tasks(tasks: &[Task]) -> TaskGroups {
    let today = chrono::Local::now().date_naive();

    let mut groups = TaskGroups {
        today: Vec::new(),
        this_week: Vec::new(),
        later: Vec::new(),
    };

    for (index, task) in tasks.iter().enumerate() {
        if task.completed {
            continue;
        }
        let days_until = (task.due_date.date() - today).num_days();
        match days_until {
            i64::MIN..=0 => groups.today.push(index),
            1..=6 => groups.this_week.push(index),
            _ => groups.later.push(index),
        }
    }

    groups.today.sort_by_key(|&i| tasks[i].due_date);
    groups.this_week.sort_by_key(|&i| tasks[i].due_date);
    groups.later.sort_by_key(|&i| tasks[i].due_date);

    groups
}

pub(crate) fn completed_tasks(tasks: &[Task]) -> Vec<usize> {
    tasks
        .iter()
        .enumerate()
        .filter(|(_, t)| t.completed)
        .map(|(index, _)| index)
        .collect()
}

/// Row order used by both selection (j/k) and rendering, so they always agree.
pub(crate) fn ordered_task_indices(tasks: &[Task]) -> Vec<usize> {
    let groups = date_grouping_tasks(tasks);
    let done = completed_tasks(tasks);

    let mut order = Vec::new();
    order.extend(groups.today);
    order.extend(groups.this_week);
    order.extend(groups.later);
    order.extend(done);
    order
}

fn sample_tasks() -> Vec<Task> {
    let today = Local::now().date_naive().and_hms_opt(0, 0, 0).unwrap();

    vec![
        Task {
            title: "ratatui calendar widget".to_string(),
            category: CategoryTypes::Guts,
            due_date: today,
            completed: false,
        },
        Task {
            title: "finish TUI layout".to_string(),
            category: CategoryTypes::Knowledge,
            due_date: today + Duration::days(3),
            completed: false,
        },
        Task {
            title: "write velvet room copy".to_string(),
            category: CategoryTypes::Knowledge,
            due_date: today + Duration::days(5),
            completed: false,
        },
        Task {
            title: "call mara re: playtesting".to_string(),
            category: CategoryTypes::Kindness,
            due_date: today + Duration::days(10),
            completed: false,
        },
        Task {
            title: "ASCII banner v2".to_string(),
            category: CategoryTypes::Knowledge,
            due_date: today + Duration::days(12),
            completed: false,
        },
        Task {
            title: "plan v2 features".to_string(),
            category: CategoryTypes::Knowledge,
            due_date: today + Duration::days(14),
            completed: false,
        },
        Task {
            title: "pick color palette".to_string(),
            category: CategoryTypes::Knowledge,
            due_date: today - Duration::days(5),
            completed: true,
        },
        Task {
            title: "set up ratatui project".to_string(),
            category: CategoryTypes::Knowledge,
            due_date: today - Duration::days(10),
            completed: true,
        },
    ]
}

fn load_tasks() -> Vec<Task> {
    fs::read_to_string(TASKS_FILE)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_else(sample_tasks)
}

fn save_tasks(tasks: &Vec<Task>) -> std::io::Result<()> {
    let contents = serde_json::to_string_pretty(tasks).map_err(std::io::Error::other)?;
    fs::write(TASKS_FILE, contents)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NewTaskStage {
    Title,
    DueDate,
}

#[derive(Debug)]
pub(crate) struct NewTaskDraft {
    pub(crate) title: String,
    pub(crate) category: CategoryTypes,
    pub(crate) date_input: String,
    pub(crate) stage: NewTaskStage,
    pub(crate) error: Option<String>,
}

const DUE_DATE_FORMAT: &str = "%d/%m/%Y %H%M";

pub struct App {
    pub active_tab: Tab,
    pub tasks: Vec<Task>,
    pub(crate) selected: usize,
    pub(crate) new_task_draft: Option<NewTaskDraft>,
    /// Day highlighted on the calendar tab; the month shown is the month this falls in.
    pub(crate) calendar_selected: NaiveDate,
    pub(crate) music: MusicState,
}

/// Where the music tab's data comes from.
pub(crate) enum MusicSource {
    /// Connected; snapshots arrive from the polling thread.
    Spotify(Receiver<PlayerUpdate>),
    /// Spotify isn't set up or failed to connect; the message says why.
    Unavailable(String),
}

pub(crate) struct MusicState {
    pub(crate) source: MusicSource,
    /// Most recent successful poll, kept while later polls fail.
    pub(crate) snapshot: Option<PlayerSnapshot>,
    /// Why the latest poll failed, cleared by the next success.
    pub(crate) error: Option<String>,
}

impl MusicState {
    /// Applies every update that has arrived since the last frame.
    fn receive_updates(&mut self) {
        let MusicSource::Spotify(receiver) = &self.source else {
            return;
        };
        loop {
            match receiver.try_recv() {
                Ok(Ok(snapshot)) => {
                    self.snapshot = Some(snapshot);
                    self.error = None;
                }
                Ok(Err(message)) => self.error = Some(message),
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.source = MusicSource::Unavailable("Spotify polling stopped".to_string());
                    return;
                }
            }
        }
    }
}

impl App {
    pub fn new(music_source: MusicSource) -> Self {
        Self {
            active_tab: Tab::default(),
            tasks: load_tasks(),
            selected: 0,
            new_task_draft: None,
            calendar_selected: Local::now().date_naive(),
            music: MusicState {
                source: music_source,
                snapshot: None,
                error: None,
            },
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        loop {
            self.music.receive_updates();
            terminal.draw(|frame| self.render(frame))?;

            // Timeout with no input: loop back and redraw so the clock stays current.
            if !crossterm::event::poll(std::time::Duration::from_millis(100))? {
                continue;
            }

            if let Event::Key(key) = crossterm::event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if self.new_task_draft.is_some() {
                    self.handle_new_task_key(key);
                    continue;
                }

                if quit(key) {
                    save_tasks(&self.tasks)?;
                    return Ok(());
                }
                if let Some(tab) = tab_switch(key) {
                    self.active_tab = tab;
                }
                if self.active_tab == Tab::Tasks {
                    self.handle_task_key(key);
                }
                if self.active_tab == Tab::Calendar {
                    self.handle_calendar_key(key);
                }
            }
        }
    }

    fn handle_task_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Char(' ') => self.toggle_selected(),
            KeyCode::Char('n') => self.start_new_task(String::new()),
            _ => {}
        }
    }

    /// Opens the new-task prompt. `date_input` pre-fills the due date field.
    fn start_new_task(&mut self, date_input: String) {
        self.new_task_draft = Some(NewTaskDraft {
            title: String::new(),
            category: CategoryTypes::Knowledge,
            date_input,
            stage: NewTaskStage::Title,
            error: None,
        });
    }

    fn handle_calendar_key(&mut self, key: KeyEvent) {
        let selected = self.calendar_selected;
        let moved = match key.code {
            KeyCode::Char('h') | KeyCode::Left => selected.checked_sub_days(Days::new(1)),
            KeyCode::Char('l') | KeyCode::Right => selected.checked_add_days(Days::new(1)),
            KeyCode::Char('k') | KeyCode::Up => selected.checked_sub_days(Days::new(7)),
            KeyCode::Char('j') | KeyCode::Down => selected.checked_add_days(Days::new(7)),
            // Same day in the neighbouring month, clamped to that month's last day.
            KeyCode::Char('[') => selected.checked_sub_months(Months::new(1)),
            KeyCode::Char(']') => selected.checked_add_months(Months::new(1)),
            KeyCode::Char('t') => Some(Local::now().date_naive()),
            KeyCode::Char('n') => {
                // Due date is the selected day, so only the time needs typing.
                self.start_new_task(selected.format("%d/%m/%Y ").to_string());
                None
            }
            _ => None,
        };
        if let Some(date) = moved {
            self.calendar_selected = date;
        }
    }

    fn handle_new_task_key(&mut self, key: KeyEvent) {
        let Some(draft) = &mut self.new_task_draft else {
            return;
        };

        match draft.stage {
            NewTaskStage::Title => match key.code {
                KeyCode::Enter => {
                    if !draft.title.trim().is_empty() {
                        draft.stage = NewTaskStage::DueDate;
                    }
                }
                KeyCode::Esc => self.new_task_draft = None,
                KeyCode::Tab => draft.category = draft.category.next(),
                KeyCode::Backspace => {
                    draft.title.pop();
                }
                KeyCode::Char(c) => draft.title.push(c),
                _ => {}
            },
            NewTaskStage::DueDate => match key.code {
                KeyCode::Enter => {
                    match NaiveDateTime::parse_from_str(draft.date_input.trim(), DUE_DATE_FORMAT) {
                        Ok(due_date) => {
                            let title = draft.title.trim().to_string();
                            let category = draft.category;
                            self.new_task_draft = None;
                            self.tasks.push(Task {
                                title,
                                category,
                                due_date,
                                completed: false,
                            });
                        }
                        Err(_) => {
                            draft.error = Some("expected dd/mm/yyyy hhmm".to_string());
                        }
                    }
                }
                KeyCode::Esc => self.new_task_draft = None,
                KeyCode::Backspace => {
                    draft.date_input.pop();
                    draft.error = None;
                }
                KeyCode::Char(c) => {
                    draft.date_input.push(c);
                    draft.error = None;
                }
                _ => {}
            },
        }
    }

    fn move_selection(&mut self, delta: i32) {
        let order = ordered_task_indices(&self.tasks);
        if order.is_empty() {
            self.selected = 0;
            return;
        }
        let current = self.selected.min(order.len() - 1) as i32;
        self.selected = (current + delta).rem_euclid(order.len() as i32) as usize;
    }

    fn toggle_selected(&mut self) {
        let order = ordered_task_indices(&self.tasks);
        if let Some(&index) = order.get(self.selected) {
            self.tasks[index].completed = !self.tasks[index].completed;
        }
    }

    fn render(&self, frame: &mut Frame) {
        ui::render(frame, self);
    }
}

fn quit(key: KeyEvent) -> bool {
    match key {
        KeyEvent {
            code: KeyCode::Char('q'),
            ..
        } => true,
        _ => false,
    }
}

fn tab_switch(key: KeyEvent) -> Option<Tab> {
    match key {
        KeyEvent {
            code: KeyCode::Char('1'),
            ..
        } => Some(Tab::VelvetRoom),
        KeyEvent {
            code: KeyCode::Char('2'),
            ..
        } => Some(Tab::Calendar),
        KeyEvent {
            code: KeyCode::Char('3'),
            ..
        } => Some(Tab::Tasks),
        KeyEvent {
            code: KeyCode::Char('4'),
            ..
        } => Some(Tab::Music),
        _ => None,
    }
}
