use color_eyre::eyre::{Result, WrapErr, bail};
use serde::Deserialize;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

const CONFIG_FILE: &str = "config.toml";
const CLIENT_ID_PLACEHOLDER: &str = "your-client-id-here";
const DEFAULT_REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";
const DASHBOARD_URL: &str = "https://developer.spotify.com/dashboard";

#[derive(Debug, Deserialize)]
pub struct Config {
    pub spotify: SpotifyConfig,
}

#[derive(Debug, Deserialize)]
pub struct SpotifyConfig {
    /// Empty means Spotify was skipped during setup and is turned off.
    pub client_id: String,
    pub redirect_uri: String,
}

/// Reads `config.toml` from the current directory.
pub fn load() -> Result<Config> {
    let contents = fs::read_to_string(CONFIG_FILE).wrap_err_with(|| {
        format!("couldn't read {CONFIG_FILE}; copy config.example.toml to {CONFIG_FILE} and fill it in")
    })?;
    let config: Config =
        toml::from_str(&contents).wrap_err_with(|| format!("{CONFIG_FILE} isn't valid"))?;

    let client_id = config.spotify.client_id.trim();
    if client_id.is_empty() || client_id == CLIENT_ID_PLACEHOLDER {
        bail!("set spotify.client_id in {CONFIG_FILE} to your Spotify app's Client ID");
    }

    Ok(config)
}

/// What startup should do about Spotify.
pub enum SpotifySetup {
    /// Configured: go ahead and log in.
    Enabled(SpotifyConfig),
    /// The user chose to skip Spotify; don't ask again.
    Disabled,
}

/// Loads the config, running first-time setup in the terminal when there isn't one yet.
/// Must run before the TUI starts, since it reads from stdin.
pub fn load_or_setup() -> Result<SpotifySetup> {
    if !Path::new(CONFIG_FILE).exists() {
        return run_setup(DEFAULT_REDIRECT_URI);
    }

    let contents = fs::read_to_string(CONFIG_FILE)?;
    let config: Config =
        toml::from_str(&contents).wrap_err_with(|| format!("{CONFIG_FILE} isn't valid"))?;

    match config.spotify.client_id.trim() {
        "" => Ok(SpotifySetup::Disabled),
        // Copied from the example but never filled in: treat it as a first run.
        CLIENT_ID_PLACEHOLDER => run_setup(&config.spotify.redirect_uri),
        _ => Ok(SpotifySetup::Enabled(config.spotify)),
    }
}

/// Explains how to get a client ID, asks for one, and saves `config.toml`.
fn run_setup(redirect_uri: &str) -> Result<SpotifySetup> {
    println!();
    println!("persona-tui · first-time Spotify setup");
    println!();
    println!("The Music tab shows what's playing on your Spotify account. To connect it:");
    println!("  1. Open {DASHBOARD_URL} and create an app.");
    println!("  2. Add this Redirect URI to the app, exactly as written:");
    println!("       {redirect_uri}");
    println!("  3. Copy the app's Client ID.");
    println!();
    println!("Spotify only allows this for apps owned by a Premium account.");
    println!();

    let stdin = io::stdin();
    let client_id = loop {
        print!("Paste your Client ID (or press Enter to skip Spotify): ");
        io::stdout().flush()?;

        let mut line = String::new();
        // EOF (no terminal attached) counts as skipping.
        if stdin.lock().read_line(&mut line)? == 0 {
            break String::new();
        }

        let input = line.trim();
        if input.is_empty() || is_valid_client_id(input) {
            break input.to_string();
        }
        println!("That doesn't look like a Client ID: it should be 32 letters and numbers.");
    };

    fs::write(CONFIG_FILE, config_contents(&client_id, redirect_uri))
        .wrap_err_with(|| format!("couldn't save {CONFIG_FILE}"))?;

    if client_id.is_empty() {
        println!("Skipped. To connect Spotify later, set client_id in {CONFIG_FILE}.");
        return Ok(SpotifySetup::Disabled);
    }

    println!("Saved to {CONFIG_FILE}.");
    Ok(SpotifySetup::Enabled(SpotifyConfig {
        client_id,
        redirect_uri: redirect_uri.to_string(),
    }))
}

/// Spotify client IDs are 32 hex characters.
fn is_valid_client_id(input: &str) -> bool {
    input.len() == 32 && input.chars().all(|c| c.is_ascii_hexdigit())
}

/// The saved file, with comments so it still makes sense when opened by hand.
fn config_contents(client_id: &str, redirect_uri: &str) -> String {
    format!(
        r#"# persona-tui configuration (git-ignored). See config.example.toml for details.

[spotify]
# Client ID from your app at {DASHBOARD_URL}
# Leave empty to turn Spotify off.
client_id = "{client_id}"

# Must match a Redirect URI registered on your Spotify app exactly.
redirect_uri = "{redirect_uri}"
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_32_hex_client_ids() {
        assert!(is_valid_client_id("72c7b0a7d8ee4cffa8343351e29a38a2"));
        assert!(!is_valid_client_id("your-client-id-here"));
        assert!(!is_valid_client_id("72c7b0a7d8ee4cffa8343351e29a38a")); // 31 chars
        assert!(!is_valid_client_id("72c7b0a7d8ee4cffa8343351e29a38zz"));
    }

    #[test]
    fn saved_config_parses_back() {
        let text = config_contents("72c7b0a7d8ee4cffa8343351e29a38a2", DEFAULT_REDIRECT_URI);
        let config: Config = toml::from_str(&text).unwrap();
        assert_eq!(config.spotify.client_id, "72c7b0a7d8ee4cffa8343351e29a38a2");
        assert_eq!(config.spotify.redirect_uri, DEFAULT_REDIRECT_URI);
    }

    #[test]
    fn skipped_config_parses_back_as_empty() {
        let config: Config = toml::from_str(&config_contents("", DEFAULT_REDIRECT_URI)).unwrap();
        assert!(config.spotify.client_id.is_empty());
    }
}
