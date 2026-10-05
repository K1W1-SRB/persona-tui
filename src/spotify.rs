use crate::config::SpotifyConfig;
use crate::cover::Cover;
use color_eyre::eyre::Result;
use rspotify::model::PlayableItem;
use rspotify::prelude::*;
use rspotify::{AuthCodePkceSpotify, Credentials, OAuth, scopes};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

/// Saved login (includes the refresh token). Git-ignored; never share it.
const TOKEN_CACHE_FILE: &str = ".spotify_token_cache.json";

/// How often the background thread asks Spotify for the player state.
/// Progress between polls is estimated locally, so this can be fairly slow.
const POLL_INTERVAL: Duration = Duration::from_secs(3);

/// Player state at one moment, as sent from the polling thread to the UI.
pub(crate) struct PlayerSnapshot {
    pub(crate) now_playing: Option<ItemInfo>,
    pub(crate) progress_secs: i64,
    pub(crate) is_playing: bool,
    /// Upcoming tracks, not including `now_playing`.
    pub(crate) queue: Vec<ItemInfo>,
    /// When this was fetched, so the UI can advance `progress_secs` between polls.
    pub(crate) fetched_at: Instant,
}

impl PlayerSnapshot {
    /// Progress right now: the fetched progress plus time since, while playing.
    pub(crate) fn current_progress_secs(&self) -> i64 {
        let elapsed = if self.is_playing {
            self.fetched_at.elapsed().as_secs() as i64
        } else {
            0
        };
        let duration = self.now_playing.as_ref().map_or(i64::MAX, |item| item.duration_secs);
        (self.progress_secs + elapsed).min(duration)
    }
}

/// New covers downloaded per poll at most, so the first poll with a long queue doesn't hold
/// up the snapshot. The rest arrive over the next few polls.
const MAX_NEW_COVERS_PER_POLL: usize = 6;

/// Covers by URL, kept between polls. `None` remembers a failed download so it isn't retried
/// every poll.
type CoverCache = HashMap<String, Option<Arc<Cover>>>;

/// Polling result: a fresh snapshot, or a message describing why the poll failed.
pub(crate) type PlayerUpdate = Result<PlayerSnapshot, String>;

/// Logs in with PKCE. Uses the cached token when there is one; otherwise opens the
/// browser for the user to approve the app.
/// Must run before the TUI starts, since it reads from the terminal.
pub fn connect(config: &SpotifyConfig) -> Result<AuthCodePkceSpotify> {
    let creds = Credentials::new_pkce(config.client_id.trim());
    let oauth = OAuth {
        redirect_uri: config.redirect_uri.clone(),
        scopes: scopes!("user-read-currently-playing", "user-read-playback-state"),
        ..Default::default()
    };
    let client_config = rspotify::Config {
        token_cached: true,
        token_refreshing: true,
        cache_path: PathBuf::from(TOKEN_CACHE_FILE),
        ..Default::default()
    };

    if !Path::new(TOKEN_CACHE_FILE).exists() {
        println!();
        println!("Logging in to Spotify: approve persona-tui in the browser window that opens.");
        println!("This only happens once; the login is saved for next time.");
    }

    let mut spotify = AuthCodePkceSpotify::with_config(creds, oauth, client_config);
    let url = spotify.get_authorize_url(None)?;
    // For a 127.0.0.1 redirect, rspotify listens on that port and catches the browser's
    // redirect itself, so there's nothing to paste.
    spotify.prompt_for_token(&url)?;
    Ok(spotify)
}

/// Fetches the player state and the queue once.
fn fetch(spotify: &AuthCodePkceSpotify) -> Result<PlayerSnapshot> {
    let playback = spotify.current_playback(None, None::<Vec<_>>)?;
    let queue = spotify.current_user_queue()?;
    let fetched_at = Instant::now();

    Ok(match playback {
        Some(playback) => PlayerSnapshot {
            now_playing: playback.item.as_ref().and_then(item_info),
            progress_secs: playback.progress.map_or(0, |p| p.num_seconds()),
            is_playing: playback.is_playing,
            queue: queue.queue.iter().filter_map(item_info).collect(),
            fetched_at,
        },
        // Nothing is playing; Spotify's queue is meaningless without a player.
        None => PlayerSnapshot {
            now_playing: None,
            progress_secs: 0,
            is_playing: false,
            queue: Vec::new(),
            fetched_at,
        },
    })
}

/// Polls Spotify on a background thread so network calls never block the UI.
/// The thread stops by itself once the receiver is dropped.
pub(crate) fn spawn_poller(spotify: AuthCodePkceSpotify) -> Receiver<PlayerUpdate> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut covers = CoverCache::new();
        loop {
            let update = fetch(&spotify)
                .map(|mut snapshot| {
                    attach_covers(&mut snapshot, &mut covers);
                    snapshot
                })
                .map_err(|err| err.to_string());
            if sender.send(update).is_err() {
                return;
            }
            thread::sleep(POLL_INTERVAL);
        }
    });
    receiver
}

/// Fills in `cover` on every item: from the cache when possible, otherwise by downloading
/// (now playing first, then the queue in order, up to `MAX_NEW_COVERS_PER_POLL` downloads).
/// Then forgets covers no longer in the snapshot so the cache doesn't grow forever.
fn attach_covers(snapshot: &mut PlayerSnapshot, cache: &mut CoverCache) {
    let mut downloads_left = MAX_NEW_COVERS_PER_POLL;
    let mut in_use = HashSet::new();

    for item in snapshot.now_playing.iter_mut().chain(snapshot.queue.iter_mut()) {
        let Some(url) = item.cover_url.clone() else {
            continue;
        };

        if !cache.contains_key(&url) {
            if downloads_left == 0 {
                // Try again next poll.
                continue;
            }
            downloads_left -= 1;
            // A failed download just means this item keeps its generated fallback art.
            cache.insert(url.clone(), Cover::download(&url).ok().map(Arc::new));
        }

        item.cover = cache.get(&url).cloned().flatten();
        in_use.insert(url);
    }

    cache.retain(|url, _| in_use.contains(url));
}

/// `cargo run -- spotify-test`: log in, then print what's playing and the queue.
pub fn run_test(config: &SpotifyConfig) -> Result<()> {
    let spotify = connect(config)?;

    println!("== Now playing ==");
    match spotify.current_playback(None, None::<Vec<_>>)? {
        Some(playback) => {
            let progress = playback.progress.map_or(0, |p| p.num_seconds());
            let state = if playback.is_playing { "playing" } else { "paused" };
            match playback.item.as_ref().and_then(item_info) {
                Some(info) => {
                    println!(
                        "{} — {}  ({} / {})  {state}",
                        info.title,
                        info.by,
                        format_secs(progress),
                        format_secs(info.duration_secs),
                    );
                    if let Some(url) = info.cover_url {
                        println!("cover: {url}");
                    }
                }
                None => println!("(unrecognised item)  {state}"),
            }
        }
        None => println!("nothing playing (start a song in Spotify and try again)"),
    }

    println!("\n== Queue ==");
    let queue = spotify.current_user_queue()?;
    if queue.queue.is_empty() {
        println!("(empty)");
    }
    for (i, item) in queue.queue.iter().take(10).enumerate() {
        match item_info(item) {
            Some(info) => println!(
                "{:>2}. {} — {}  {}",
                i + 1,
                info.title,
                info.by,
                format_secs(info.duration_secs)
            ),
            None => println!("{:>2}. (unrecognised item)", i + 1),
        }
    }

    Ok(())
}

/// The few fields we actually use from a track or episode.
///
/// rspotify's own `FullTrack` requires fields Spotify has since removed (e.g. `external_ids`),
/// so items often arrive as `PlayableItem::Unknown(json)`. Parsing that JSON ourselves with
/// only the fields we need keeps working when Spotify drops fields we don't use.
#[derive(Debug, Deserialize)]
struct RawItem {
    name: String,
    #[serde(default)]
    duration_ms: i64,
    /// Tracks only.
    #[serde(default)]
    artists: Vec<RawNamed>,
    /// Tracks only.
    album: Option<RawAlbum>,
    /// Episodes only.
    show: Option<RawNamed>,
    /// Episodes only.
    #[serde(default)]
    images: Vec<RawImage>,
}

#[derive(Debug, Deserialize)]
struct RawNamed {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RawAlbum {
    #[serde(default)]
    images: Vec<RawImage>,
}

#[derive(Debug, Deserialize)]
struct RawImage {
    url: String,
    width: Option<u32>,
}

/// Normalised view of whatever is playing, whichever way rspotify managed to parse it.
pub(crate) struct ItemInfo {
    pub(crate) title: String,
    /// Artists for a track, show name for a podcast episode.
    pub(crate) by: String,
    pub(crate) duration_secs: i64,
    /// Smallest cover image; plenty for terminal art.
    pub(crate) cover_url: Option<String>,
    /// Downloaded cover, filled in by the poller. `Arc` so snapshots share covers instead of
    /// copying pixels, since most of a queue comes from a few albums.
    pub(crate) cover: Option<Arc<Cover>>,
}

fn item_info(item: &PlayableItem) -> Option<ItemInfo> {
    match item {
        PlayableItem::Track(track) => Some(ItemInfo {
            title: track.name.clone(),
            by: join_names(track.artists.iter().map(|a| a.name.as_str())),
            duration_secs: track.duration.num_seconds(),
            cover_url: smallest(track.album.images.iter().map(|i| (i.url.as_str(), i.width))),
            cover: None,
        }),
        PlayableItem::Episode(episode) => Some(ItemInfo {
            title: episode.name.clone(),
            by: episode.show.name.clone(),
            duration_secs: episode.duration.num_seconds(),
            cover_url: smallest(episode.images.iter().map(|i| (i.url.as_str(), i.width))),
            cover: None,
        }),
        PlayableItem::Unknown(json) => {
            let raw: RawItem = serde_json::from_value(json.clone()).ok()?;
            let by = match &raw.show {
                Some(show) => show.name.clone(),
                None => join_names(raw.artists.iter().map(|a| a.name.as_str())),
            };
            let images = raw.album.as_ref().map_or(&raw.images, |album| &album.images);
            Some(ItemInfo {
                cover_url: smallest(images.iter().map(|i| (i.url.as_str(), i.width))),
                title: raw.name,
                by,
                duration_secs: raw.duration_ms / 1000,
                cover: None,
            })
        }
    }
}

fn join_names<'a>(names: impl Iterator<Item = &'a str>) -> String {
    names.collect::<Vec<_>>().join(", ")
}

fn smallest<'a>(images: impl Iterator<Item = (&'a str, Option<u32>)>) -> Option<String> {
    images
        .min_by_key(|(_, width)| width.unwrap_or(u32::MAX))
        .map(|(url, _)| url.to_string())
}

fn format_secs(secs: i64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}
