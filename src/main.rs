mod app;
mod config;
mod cover;
mod event;
mod spotify;
mod tui;
mod ui;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // `cargo run -- spotify-test` checks the Spotify login without starting the TUI.
    if std::env::args().nth(1).as_deref() == Some("spotify-test") {
        let config = config::load()?;
        return spotify::run_test(&config.spotify);
    }

    // Connect before the TUI starts: a first login reads the redirect URL from the terminal.
    let music_source = connect_music();

    ratatui::run(|terminal| app::App::new(music_source).run(terminal))?;
    Ok(())
}

/// Spotify problems never stop the app; they're shown on the music tab instead.
/// On the first run this asks for a client ID, then opens the browser to log in.
fn connect_music() -> app::MusicSource {
    let spotify_config = match config::load_or_setup() {
        Ok(config::SpotifySetup::Enabled(spotify_config)) => spotify_config,
        Ok(config::SpotifySetup::Disabled) => {
            return app::MusicSource::Unavailable(
                "Spotify is turned off — set client_id in config.toml to connect".to_string(),
            );
        }
        Err(err) => return app::MusicSource::Unavailable(err.to_string()),
    };
    match spotify::connect(&spotify_config) {
        Ok(client) => app::MusicSource::Spotify(spotify::spawn_poller(client)),
        Err(err) => app::MusicSource::Unavailable(format!("couldn't connect to Spotify: {err}")),
    }
}
