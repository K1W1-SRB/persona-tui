mod app;
mod event;
mod tui;
mod ui;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    ratatui::run(|terminal| app::App::new().run(terminal))?;
    Ok(())
}
