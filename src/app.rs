use crate::ui;
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{DefaultTerminal, Frame};

fn quit(key: KeyEvent) -> bool {
    match key {
        KeyEvent {
            code: KeyCode::Char('q'),
            ..
        } => true,
        _ => false,
    }
}

pub fn run(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    loop {
        terminal.draw(ui::render)?;
        if let Event::Key(key) = crossterm::event::read()? {
            if quit(key) {
                break Ok(());
            }
        }
    }
}
