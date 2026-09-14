mod app;
mod event;
mod tui;
mod ui;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    ratatui::run(app::run)?;
    Ok(())
}
