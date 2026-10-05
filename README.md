# persona-tui

A Persona-style terminal dashboard written in Rust with [ratatui](https://ratatui.rs). It has four tabs:

| Tab | What it shows |
|---|---|
| **1 · Velvet Room** | Today's date, your next deadline, a week strip, stat levels, and what's playing on Spotify |
| **2 · Calendar** | A month grid. Days with deadlines show a moon that fills as the deadline gets closer |
| **3 · Tasks** | Your tasks grouped by Today, This week, Later and Done |
| **4 · Music** | Your current Spotify track and queue, with album covers drawn in the terminal |

The Spotify part is optional. Everything else works without it.

## Requirements

- **Rust 1.88 or newer.** Install it from [rustup.rs](https://rustup.rs), or update an existing install with `rustup update`.
- **A terminal with true colour (24-bit) support**, for the album covers and theme colours. Windows Terminal, iTerm2, WezTerm, Alacritty, Kitty and most modern Linux terminals all work. The old Windows console window (`conhost`) doesn't.
- **A font that includes braille characters** (`⢾⡷`), used for the small moons. Cascadia Code, JetBrains Mono and most Nerd Fonts include them.
- For the Music tab: **Spotify Premium** on the account that owns your Spotify developer app (see below).

## Getting started

```sh
git clone https://github.com/K1W1-SRB/persona-tui.git
cd persona-tui
cargo run --release
```

The first build takes a minute or two. On the first launch, persona-tui asks whether you want to connect Spotify. If you don't, press **Enter** to skip; you can connect it later.

> **Run it from the project folder.** persona-tui reads and writes `config.toml`, `tasks.json` and the Spotify login in the folder you start it from.

## Connecting Spotify (optional)

The Music tab reads your playback through the Spotify Web API. Spotify doesn't allow one shared app for everyone, so you create your own (free, about 2 minutes) and give persona-tui its **Client ID**.

### 1. Create a Spotify app

1. Go to the [Spotify Developer Dashboard](https://developer.spotify.com/dashboard) and log in **with your Premium account**. The app must be owned by a Premium account, or every request fails with `403`.
2. Click **Create app** and fill in:
   - **App name / description:** anything, e.g. `persona-tui`
   - **Redirect URI:** `http://127.0.0.1:8888/callback`. Click **Add**. It must match exactly: use `127.0.0.1`, not `localhost`.
   - **Which API/SDKs are you planning to use?** Tick **Web API**.
3. Save, then open the app's **Settings** and copy the **Client ID**.

You don't need the client secret. persona-tui logs in with PKCE, which only uses the client ID.

### 2. Give persona-tui the Client ID

Start the app (`cargo run --release`). When it asks, paste the Client ID and press Enter. It's saved to `config.toml`.

You can also set it by hand. Copy the example file and fill it in:

```sh
cp config.example.toml config.toml
```

```toml
[spotify]
client_id = "your-client-id-here"
redirect_uri = "http://127.0.0.1:8888/callback"
```

### 3. Log in

On the first launch with a Client ID, your browser opens on Spotify's approval page. Click **Agree**. persona-tui receives the login automatically on `127.0.0.1:8888`, and the TUI starts.

The login is saved in `.spotify_token_cache.json` and refreshed automatically, so you only do this once.

### Checking the connection

To test Spotify without starting the TUI, play something in Spotify, then run:

```sh
cargo run -- spotify-test
```

It prints the current track and the next few tracks in your queue.

## Controls

| Key | Where | Action |
|---|---|---|
| `1` `2` `3` `4` | everywhere | switch tab |
| `q` | everywhere | quit (saves tasks) |
| `j` / `k` | Tasks | move down / up |
| `space` | Tasks | mark the selected task done / not done |
| `n` | Tasks | new task |
| `h` `l` or `←` `→` | Calendar | previous / next day |
| `k` `j` or `↑` `↓` | Calendar | previous / next week |
| `[` / `]` | Calendar | previous / next month |
| `t` | Calendar | jump to today |
| `n` | Calendar | new task on the selected day |

**Adding a task:** type the title, press `Tab` to change the category (Knowledge, Kindness or Guts), and press `Enter`. Then type the due date as `dd/mm/yyyy hhmm`, e.g. `24/10/2026 1400`, and press `Enter`. From the Calendar, the date is filled in for you, so you only type the time. `Esc` cancels.

## Files persona-tui creates

All of these live in the folder you run persona-tui from, and the private ones are already in `.gitignore`.

| File | What it is | Private? |
|---|---|---|
| `config.toml` | Your Spotify Client ID and redirect URI | Yes, git-ignored |
| `.spotify_token_cache.json` | Your Spotify login, including the refresh token. **Never share or commit it.** | Yes, git-ignored |
| `tasks.json` | Your tasks. If it's missing, you start with sample tasks; it's saved when you quit | Your choice |

## Troubleshooting

**`403` / "Active premium subscription required for the owner of the app"**
The account that *created the app* on the developer dashboard doesn't have Premium. It may not be the account you logged in with in the browser. Create the app again while logged in to the dashboard with your Premium account, put the new Client ID in `config.toml`, and delete `.spotify_token_cache.json`. If you've only just subscribed, Spotify says it can take a few hours to take effect.

**`INVALID_CLIENT: Invalid redirect URI`**
The redirect URI in `config.toml` doesn't exactly match the one in your Spotify app. Check `127.0.0.1` vs `localhost`, the port and any trailing `/`.

**Changed your Client ID and now get errors**
The saved login belongs to the old app. Delete `.spotify_token_cache.json` and start persona-tui again to log in fresh.

**Someone else wants to use your Spotify app**
Spotify apps in development mode only work for accounts you add under **User Management** in the dashboard. It's usually easier for them to create their own app.

**Port 8888 is already in use**
Pick another port, e.g. `http://127.0.0.1:8899/callback`. Set it both in your Spotify app's Redirect URIs and in `config.toml`.

**"nothing playing — start something in Spotify"**
Spotify only reports a player while something is playing or paused on one of your devices. Start a song and it appears within a few seconds.

**Album covers look like coloured blocks, or the colours are wrong**
Your terminal probably doesn't support true colour. Try Windows Terminal or another modern terminal.

**Moons show as boxes or question marks**
Your font doesn't have braille characters. Switch to a font such as Cascadia Code or JetBrains Mono.

**Turning Spotify off**
Set `client_id = ""` in `config.toml`. persona-tui won't ask again.

## Development

```sh
cargo test     # run the tests
cargo run      # debug build
```

| Path | What's there |
|---|---|
| `src/app.rs` | App state, tasks, key handling |
| `src/ui/` | One file per tab, plus shared pieces (`theme.rs`, `moon.rs`) |
| `src/spotify.rs` | Spotify login, background polling and cover downloads |
| `src/cover.rs` | Turns album art into terminal pixels |
| `src/config.rs` | `config.toml` loading and first-run setup |
