use super::theme::{ACCENT, FAINT, MUTED, SELECTED_BG, TEXT, TRACK};
use super::{content_area, divider};
use crate::app::App;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Widget};

/// Everything the music tab draws. The UI only reads this; fill it from the real player.
pub(crate) struct MusicView<'a> {
    pub(crate) queue: &'a [TrackView<'a>],
    /// Index into `queue` of the track that's playing.
    pub(crate) current: usize,
    pub(crate) elapsed_secs: u32,
    pub(crate) playing: bool,
}

pub(crate) struct TrackView<'a> {
    pub(crate) title: &'a str,
    pub(crate) artist: &'a str,
    pub(crate) duration_secs: u32,
}

// Placeholder until the player logic exists.
const SAMPLE_QUEUE: [TrackView<'static>; 7] = [
    sample("Burn My Dread", 215),
    sample("Mass Destruction", 242),
    sample("Master of Tartarus", 228),
    sample("Color Your Night", 179),
    sample("Iwatodai Dorm", 107),
    sample("Living With Determination", 201),
    sample("Deep Breath Deep Breath", 189),
];

const fn sample(title: &'static str, duration_secs: u32) -> TrackView<'static> {
    TrackView {
        title,
        artist: "Shoji Meguro",
        duration_secs,
    }
}

fn sample_view() -> MusicView<'static> {
    MusicView {
        queue: &SAMPLE_QUEUE,
        current: 0,
        elapsed_secs: 134,
        playing: true,
    }
}

const COVER_WIDTH: u16 = 7;
const COVER_HEIGHT: u16 = 3;
const GLYPH_WIDTH: u16 = 5;
const QUEUE_ROW_HEIGHT: u16 = 2;
const QUEUE_ROW_GAP: u16 = 1;

pub fn render(frame: &mut Frame, app: &App, rows: &[Rect]) {
    let view = sample_view();

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // now playing
            Constraint::Length(1), // gap
            Constraint::Length(1), // divider
            Constraint::Length(1), // "QUEUE · 7 TRACKS"
            Constraint::Length(1), // gap
            Constraint::Min(0),    // queue
        ])
        .split(content_area(rows));

    render_now_playing(frame, &view, sections[0]);
    frame.render_widget(divider(), sections[2]);
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("QUEUE · {} TRACKS", view.queue.len()),
            Style::new().fg(FAINT),
        )),
        sections[3],
    );
    render_queue(frame, &view, sections[5]);

    frame.render_widget(super::footer(app.active_tab), rows[4]);
}

fn render_now_playing(frame: &mut Frame, view: &MusicView, area: Rect) {
    let Some(track) = view.queue.get(view.current) else {
        frame.render_widget(
            Paragraph::new(Span::styled("nothing playing", Style::new().fg(FAINT))),
            area,
        );
        return;
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(COVER_WIDTH),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(area);

    let cover_area = Rect {
        height: COVER_HEIGHT.min(cols[0].height),
        ..cols[0]
    };
    frame.render_widget(CoverArt::new(track.title), cover_area);

    let lines = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1); 4])
        .split(cols[2]);

    frame.render_widget(
        Paragraph::new(Span::styled(
            track.title,
            Style::new().fg(TEXT).add_modifier(Modifier::BOLD),
        )),
        lines[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(track.artist, Style::new().fg(MUTED))),
        lines[1],
    );

    let progress = if track.duration_secs == 0 {
        0.0
    } else {
        (view.elapsed_secs as f64 / track.duration_secs as f64).clamp(0.0, 1.0)
    };
    frame.render_widget(Paragraph::new(progress_bar(lines[2].width, progress)), lines[2]);

    // Elapsed | transport controls | total
    let play_pause = if view.playing { "❚❚" } else { "▶" };
    frame.render_widget(
        Paragraph::new(Span::styled(
            format_time(view.elapsed_secs),
            Style::new().fg(FAINT),
        )),
        lines[3],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("◀◀   {play_pause}   ▶▶"),
            Style::new().fg(MUTED),
        ))
        .alignment(Alignment::Center),
        lines[3],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format_time(track.duration_secs),
            Style::new().fg(FAINT),
        ))
        .alignment(Alignment::Right),
        lines[3],
    );
}

fn render_queue(frame: &mut Frame, view: &MusicView, area: Rect) {
    let visible = ((area.height + QUEUE_ROW_GAP) / (QUEUE_ROW_HEIGHT + QUEUE_ROW_GAP)) as usize;
    if visible == 0 {
        return;
    }
    // Scroll just enough to keep the playing track on screen.
    let first = view.current.saturating_sub(visible - 1);

    let row_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(QUEUE_ROW_HEIGHT); visible])
        .spacing(QUEUE_ROW_GAP)
        .split(area);

    for (offset, row_area) in row_areas.iter().enumerate() {
        let index = first + offset;
        if let Some(track) = view.queue.get(index) {
            render_queue_row(frame, track, index == view.current, *row_area);
        }
    }
}

/// Active row uses the same pattern as the Tasks tab: accent bar on the left plus a tinted row.
fn render_queue_row(frame: &mut Frame, track: &TrackView, is_current: bool, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1),           // accent bar
            Constraint::Length(1),           // gap
            Constraint::Length(GLYPH_WIDTH), // glyph
            Constraint::Length(2),           // gap
            Constraint::Min(0),              // title / artist
            Constraint::Length(6),           // duration
        ])
        .split(area);

    if is_current {
        frame.render_widget(Block::default().style(Style::new().bg(ACCENT)), cols[0]);
        let row = Rect {
            x: cols[1].x,
            width: area.width.saturating_sub(1),
            ..area
        };
        frame.render_widget(Block::default().style(Style::new().bg(SELECTED_BG)), row);
    }

    frame.render_widget(CoverArt::new(track.title), cols[2]);

    let title_style = if is_current {
        Style::new().fg(TEXT).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(TEXT)
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(track.title, title_style)),
            Line::from(Span::styled(track.artist, Style::new().fg(FAINT))),
        ]),
        cols[4],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format_time(track.duration_secs),
            Style::new().fg(FAINT),
        ))
        .alignment(Alignment::Right),
        Rect {
            width: cols[5].width.saturating_sub(1),
            ..cols[5]
        },
    );
}

fn progress_bar(width: u16, progress: f64) -> Line<'static> {
    let width = width as usize;
    let filled = ((width as f64) * progress).round() as usize;
    Line::from(vec![
        Span::styled("━".repeat(filled), Style::new().fg(ACCENT)),
        Span::styled("━".repeat(width - filled), Style::new().fg(TRACK)),
    ])
}

/// 215 → "3:35"
fn format_time(secs: u32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

const COVER_PALETTE: [Color; 6] = [
    Color::Rgb(130, 110, 230), // purple
    Color::Rgb(80, 120, 230),  // blue
    Color::Rgb(210, 70, 60),   // red
    Color::Rgb(60, 170, 90),   // green
    Color::Rgb(60, 170, 170),  // teal
    Color::Rgb(230, 200, 90),  // yellow
];

/// Pixel-art cover generated from a seed (the track title), mirrored left-right like an
/// identicon. The same title always gives the same picture. Drawn with half-blocks, so an
/// area of W × H cells is a W × 2H pixel grid.
struct CoverArt {
    hash: u64,
}

impl CoverArt {
    fn new(seed: &str) -> Self {
        Self { hash: fnv1a(seed) }
    }

    fn color(&self) -> Color {
        COVER_PALETTE[(self.hash >> 56) as usize % COVER_PALETTE.len()]
    }

    /// Whether pixel (x, y) is filled. Only the left half (plus middle column) comes from
    /// the hash; the right half mirrors it.
    fn filled(&self, x: u16, y: u16, width: u16) -> bool {
        let unique_cols = width.div_ceil(2);
        let x = if x < unique_cols { x } else { width - 1 - x };
        let bit = (y * unique_cols + x) as u32 % 56;
        (self.hash >> bit) & 1 == 1
    }
}

impl Widget for CoverArt {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let color = self.color();
        for row in 0..area.height {
            for col in 0..area.width {
                let top = self.filled(col, row * 2, area.width);
                let bottom = self.filled(col, row * 2 + 1, area.width);
                let symbol = match (top, bottom) {
                    (true, true) => '█',
                    (true, false) => '▀',
                    (false, true) => '▄',
                    (false, false) => continue,
                };
                if let Some(cell) = buf.cell_mut((area.x + col, area.y + row)) {
                    cell.set_char(symbol).set_fg(color);
                }
            }
        }
    }
}

/// Small stable hash, so covers don't change between Rust versions.
fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ byte as u64).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_times() {
        assert_eq!(format_time(215), "3:35");
        assert_eq!(format_time(5), "0:05");
    }

    #[test]
    fn covers_are_stable_and_mirrored() {
        let a = CoverArt::new("Burn My Dread");
        assert_eq!(a.hash, CoverArt::new("Burn My Dread").hash);
        for y in 0..6 {
            for x in 0..7 {
                assert_eq!(a.filled(x, y, 7), a.filled(6 - x, y, 7));
            }
        }
    }
}
