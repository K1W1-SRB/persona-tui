use crate::app::Task;
use chrono::NaiveDate;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

pub(crate) const MOON_LIT: Color = Color::Rgb(232, 200, 110);
pub(crate) const MOON_DARK: Color = Color::Rgb(40, 44, 62);

/// How many days out a deadline starts to show on the moon.
const DAYS_VISIBLE: i64 = 7;

/// 0.0 = black (nothing due within a week of `date`), 1.0 = full (a task is due on `date`).
pub(crate) fn deadline_fill(tasks: &[Task], date: NaiveDate) -> f64 {
    let nearest = tasks
        .iter()
        .filter(|t| !t.completed)
        .map(|t| (t.due_date.date() - date).num_days())
        .filter(|&days| days >= 0)
        .min();

    match nearest {
        Some(days) => days_to_fill(days),
        None => 0.0,
    }
}

/// Fill for a single deadline seen from `today`: full when due (or overdue), black a week out.
pub(crate) fn proximity_fill(due: NaiveDate, today: NaiveDate) -> f64 {
    days_to_fill((due - today).num_days())
}

fn days_to_fill(days: i64) -> f64 {
    (DAYS_VISIBLE - days).clamp(0, DAYS_VISIBLE) as f64 / DAYS_VISIBLE as f64
}

/// Moon drawn with half-block characters, filling from the right as `fill` goes 0 → 1.
/// Scales to whatever area it's given.
pub(crate) struct Moon {
    fill: f64,
}

impl Moon {
    pub(crate) fn new(fill: f64) -> Self {
        Self {
            fill: fill.clamp(0.0, 1.0),
        }
    }

    /// Colour of one "pixel", or None if it's outside the disc.
    /// Pixels are half a cell tall, which makes them roughly square.
    fn pixel(&self, px: f64, py: f64, width: f64, height: f64) -> Option<Color> {
        let radius = width.min(height) / 2.0;
        let x = px + 0.5 - width / 2.0;
        let y = py + 0.5 - height / 2.0;

        if x * x + y * y > radius * radius {
            return None;
        }

        // The terminator is an ellipse: at fill 0 it sits on the right edge, at 1 on the left.
        let half_width = (radius * radius - y * y).sqrt();
        let terminator = (1.0 - 2.0 * self.fill) * half_width;

        Some(if x > terminator { MOON_LIT } else { MOON_DARK })
    }
}

impl Widget for Moon {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = area.width as f64;
        let height = area.height as f64 * 2.0;

        for row in 0..area.height {
            for col in 0..area.width {
                let top = self.pixel(col as f64, row as f64 * 2.0, width, height);
                let bottom = self.pixel(col as f64, row as f64 * 2.0 + 1.0, width, height);

                let Some(cell) = buf.cell_mut((area.x + col, area.y + row)) else {
                    continue;
                };

                match (top, bottom) {
                    (Some(t), Some(b)) => {
                        cell.set_char('▀').set_fg(t).set_bg(b);
                    }
                    (Some(t), None) => {
                        cell.set_char('▀').set_fg(t);
                    }
                    (None, Some(b)) => {
                        cell.set_char('▄').set_fg(b);
                    }
                    (None, None) => {}
                }
            }
        }
    }
}

/// Braille dots are thin, so the dark side needs to be a bit brighter to stay visible.
const SMALL_MOON_DARK: Color = Color::Rgb(70, 76, 100);

/// Bit for each braille dot, indexed [column][row] within one character (2 × 4 dots).
const BRAILLE_DOTS: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];

/// One-line moon for the week strip: 2 braille characters making a 4×4 dot disc.
/// Uses the same lit/dark maths as `Moon`.
pub(crate) struct SmallMoon {
    moon: Moon,
}

impl SmallMoon {
    pub(crate) fn new(fill: f64) -> Self {
        Self {
            moon: Moon::new(fill),
        }
    }
}

impl Widget for SmallMoon {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for char_index in 0..area.width.min(2) {
            let mut lit = 0u8;
            let mut dark = 0u8;

            for (dot_col, rows) in BRAILLE_DOTS.iter().enumerate() {
                for (dot_row, bit) in rows.iter().enumerate() {
                    let px = (char_index as usize * 2 + dot_col) as f64;
                    match self.moon.pixel(px, dot_row as f64, 4.0, 4.0) {
                        Some(MOON_LIT) => lit |= bit,
                        Some(_) => dark |= bit,
                        None => {}
                    }
                }
            }

            // One colour per character: if it's partly lit, show just the lit dots.
            let (bits, color) = if lit != 0 {
                (lit, MOON_LIT)
            } else {
                (dark, SMALL_MOON_DARK)
            };

            let Some(symbol) = char::from_u32(0x2800 + bits as u32) else {
                continue;
            };
            if let Some(cell) = buf.cell_mut((area.x + char_index, area.y)) {
                cell.set_char(symbol).set_fg(color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::CategoryTypes;

    fn task_due(date: NaiveDate, completed: bool) -> Task {
        Task {
            title: "t".to_string(),
            category: CategoryTypes::Knowledge,
            due_date: date.and_hms_opt(12, 0, 0).unwrap(),
            completed,
        }
    }

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, d).unwrap()
    }

    #[test]
    fn full_on_due_date() {
        assert_eq!(deadline_fill(&[task_due(day(10), false)], day(10)), 1.0);
    }

    #[test]
    fn black_a_week_out() {
        assert_eq!(deadline_fill(&[task_due(day(17), false)], day(10)), 0.0);
    }

    #[test]
    fn half_way_is_between() {
        let fill = deadline_fill(&[task_due(day(14), false)], day(10));
        assert!(fill > 0.0 && fill < 1.0);
    }

    #[test]
    fn nearest_task_wins_and_completed_ignored() {
        let tasks = [task_due(day(11), true), task_due(day(12), false), task_due(day(16), false)];
        assert_eq!(deadline_fill(&tasks, day(10)), 5.0 / 7.0);
    }

    #[test]
    fn past_tasks_ignored() {
        assert_eq!(deadline_fill(&[task_due(day(5), false)], day(10)), 0.0);
    }

    #[test]
    fn proximity_full_when_due_or_overdue_black_a_week_out() {
        assert_eq!(proximity_fill(day(10), day(10)), 1.0);
        assert_eq!(proximity_fill(day(8), day(10)), 1.0);
        assert_eq!(proximity_fill(day(17), day(10)), 0.0);
    }
}
