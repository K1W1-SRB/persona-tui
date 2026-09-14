use crate::{
    app::Tab::{Calendar, VelvetRoom},
    ui,
};
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{DefaultTerminal, Frame};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    VelvetRoom,
    Calendar,
    Tasks,
    Music,
}

pub struct App {
    pub active_tab: Tab,
}

impl App {
    pub fn new() -> Self {
        Self {
            active_tab: Tab::default(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;

            if let Event::Key(key) = crossterm::event::read()? {
                if quit(key) {
                    return Ok(());
                }
                if let Some(tab) = tab_switch(key) {
                    self.active_tab = tab;
                }
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        ui::render(frame, self.active_tab);
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
