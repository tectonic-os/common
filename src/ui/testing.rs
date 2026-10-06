//! Calling tests use the production draw functions, so snapshots cover the
//! frames shown to users without needing a host terminal.

use crate::ui::choose::{draw as draw_picker, initial_state};
use crate::ui::chrome::{has_box, in_frame_box, in_overlay_frame};
use crate::ui::form::{draw_table_menu, form_frame_rows};
use crate::ui::layout::laid_out;
use crate::ui::progress::{progress_head, working, ROWS};
use crate::ui::review::{decide_content_height, decide_draw, sheet_of, written, LINE_ROWS};
use crate::ui::term::{height as picker_height, render_with_title};
use crate::ui::{Choice, Field, HeaderLine, LINE_KEYS, SIDE, SUB_KEYS};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::{Frame, Terminal};
use std::fmt;

pub use crate::ui::form::Mode as FormMode;

/// The opaque buffer lets a calling test reuse a rendered form as an overlay
/// backdrop.
pub struct Snapshot(Buffer);

impl fmt::Display for Snapshot {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "{:?}", self.0)
    }
}

struct Overlay {
    width: u16,
    height: u16,
    title: Option<String>,
    backdrop: Buffer,
}

pub struct Screen {
    width: u16,
    height: u16,
    title: Option<String>,
    overlay: Option<Overlay>,
}

impl Screen {
    pub fn inline(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            title: None,
            overlay: None,
        }
    }

    pub fn full_screen(width: u16, height: u16, title: impl Into<String>) -> Self {
        Self {
            width,
            height,
            title: Some(title.into()),
            overlay: None,
        }
    }

    /// A production overlay starts on a blank terminal frame, so the calling
    /// test supplies the frame that remains visible behind its window.
    pub fn overlay(
        &self,
        width: u16,
        height: u16,
        title: Option<&str>,
        backdrop: &Snapshot,
    ) -> Self {
        Self {
            width: self.width,
            height: self.height,
            title: self.title.clone(),
            overlay: Some(Overlay {
                width,
                height,
                title: title.map(str::to_string),
                backdrop: backdrop.0.clone(),
            }),
        }
    }

    fn render(
        &self,
        keys: &str,
        frame_spec: impl FnOnce() -> (u16, String),
        body: impl FnOnce(&mut Frame, Rect),
    ) -> Snapshot {
        let mut terminal = Terminal::new(TestBackend::new(self.width, self.height))
            .expect("the in-memory terminal has a valid size");
        let draw = || {
            in_frame_box(has_box(self.title.as_deref()), || {
                let (rows, head) = frame_spec();
                render_with_title(
                    &mut terminal,
                    self.title.as_deref(),
                    rows,
                    keys,
                    &head,
                    body,
                )
            })
        };
        match &self.overlay {
            Some(overlay) => in_overlay_frame(
                overlay.width,
                overlay.height,
                overlay.title.as_deref(),
                Some(&overlay.backdrop),
                draw,
            ),
            None => draw(),
        }
        .expect("the in-memory terminal accepts a frame");
        Snapshot(terminal.backend().buffer().clone())
    }

    /// Pins the row enter would take, so its selection style stays reviewable.
    pub fn picker(
        &self,
        question: &str,
        options: &[Choice],
        on: Option<&[usize]>,
        keys: &str,
        at: usize,
    ) -> Snapshot {
        let mut state = initial_state(options, at);
        self.render(
            keys,
            || (picker_height(options.len()), question.to_string()),
            |frame, area| draw_picker(frame, area, question, options, on, keys, &mut state),
        )
    }

    /// Pins the moving progress state and its log tail to one reviewable frame.
    pub fn progress(
        &self,
        pct: u16,
        turn: usize,
        step: &str,
        notes: &[String],
        foot: &str,
    ) -> Snapshot {
        self.render(
            foot,
            || (ROWS, progress_head(turn, step)),
            |frame, area| working(frame, area, pct, turn, step, notes, foot),
        )
    }

    /// Keeps the replacement of a default by a typed answer reviewable.
    pub fn line(
        &self,
        question: &str,
        prefix: &str,
        typed: &str,
        default: Option<&str>,
    ) -> Snapshot {
        self.render(
            LINE_KEYS,
            || (LINE_ROWS, question.to_string()),
            |frame, area| written(frame, area, question, prefix, typed, default),
        )
    }
}

/// A calling test names the form's control state directly, so the snapshot does
/// not depend on terminal key routing.
pub struct FormFrame<'a> {
    pub fields: &'a [Field],
    pub visible: &'a [usize],
    pub cursor: usize,
    pub button: usize,
    pub mode: &'a FormMode,
    pub actions: &'a [&'a str],
    pub blocked: Option<&'a str>,
    pub tried: bool,
    pub keys: &'a str,
    pub header: &'a [HeaderLine],
    pub sections: &'a [(usize, &'a str)],
}

impl FormFrame<'_> {
    pub fn render(&self, screen: &Screen) -> Snapshot {
        let (lines, focus, tail) = laid_out(
            self.fields,
            self.visible,
            self.cursor,
            self.button,
            self.mode,
            self.actions,
            self.blocked,
            self.tried,
            self.header,
            self.sections,
        );
        let keys = match self.mode {
            FormMode::Open(_) | FormMode::Table | FormMode::TableMenu { .. } => SUB_KEYS,
            _ => self.keys,
        };
        screen.render(
            keys,
            || (form_frame_rows(lines.len()), String::new()),
            |frame, area| {
                sheet_of(frame, area, &lines, keys, focus, tail);
                let Some(row) = self.visible.get(self.cursor).copied() else {
                    return;
                };
                let FormMode::TableMenu { open, cursor } = self.mode else {
                    return;
                };
                draw_table_menu(frame, area, self.fields, row, *open, *cursor);
            },
        )
    }
}

/// The destructive boundary keeps its cost and chosen answer reviewable as one
/// frame.
pub struct ConfirmationFrame<'a> {
    pub heading: &'a str,
    pub note: &'a str,
    pub warning: &'a [&'a str],
    pub rows: &'a [(String, String)],
    pub ready: &'a str,
    pub yes: &'a str,
    pub no: &'a str,
    pub button: usize,
}

impl ConfirmationFrame<'_> {
    pub fn render(&self, screen: &Screen) -> Snapshot {
        screen.render(
            SIDE,
            || {
                (
                    decide_content_height(self.heading, self.note, self.warning, self.rows) as u16,
                    self.heading.to_string(),
                )
            },
            |frame, area| {
                decide_draw(
                    frame,
                    area,
                    self.heading,
                    self.note,
                    self.warning,
                    self.rows,
                    self.ready,
                    self.yes,
                    self.no,
                    self.button,
                )
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Stylize;
    use ratatui::text::Line;
    use ratatui::widgets::Paragraph;

    #[test]
    fn snapshot_text_records_content_and_style_transitions() {
        let screen = Screen::inline(8, 1);
        let drawn = screen
            .render(
                "",
                || (1, String::new()),
                |frame, area| {
                    frame.render_widget(
                        Paragraph::new(Line::from(vec!["plain".into(), "hot".red().bold()])),
                        area,
                    )
                },
            )
            .to_string();

        assert!(drawn.contains("plainhot"), "{drawn}");
        assert!(drawn.contains("fg: Red"), "{drawn}");
        assert!(drawn.contains("modifier: BOLD"), "{drawn}");
    }

    #[test]
    fn full_screen_snapshots_use_the_production_chrome() {
        let screen = Screen::full_screen(80, 25, "Install Tectonic");
        let drawn = screen
            .picker(
                "Choose a disk",
                &[Choice::new("/dev/vda", "20 GB")],
                None,
                "enter selects",
                0,
            )
            .to_string();

        assert!(drawn.contains("Choose a disk"), "{drawn}");
        assert!(drawn.contains("enter selects"), "{drawn}");
        assert!(drawn.contains("╭─ "), "{drawn}");
        assert_eq!(drawn.matches("Choose a disk").count(), 1, "{drawn}");
    }

    #[test]
    fn screen_chrome_is_scoped_to_one_render() {
        let full_screen = Screen::full_screen(80, 25, "Install Tectonic");
        full_screen.picker(
            "Choose a disk",
            &[Choice::new("/dev/vda", "20 GB")],
            None,
            "enter selects",
            0,
        );

        let inline = Screen::inline(40, 8);
        let drawn = inline
            .picker(
                "Choose inline",
                &[Choice::new("answer", "")],
                None,
                "enter selects",
                0,
            )
            .to_string();

        assert!(drawn.contains("Choose inline"), "{drawn}");
        assert!(drawn.contains("enter selects"), "{drawn}");
        assert!(!drawn.contains('╭'), "{drawn}");
    }

    #[test]
    fn a_picker_normalizes_a_heading_to_the_first_answer() {
        let screen = Screen::inline(40, 8);
        let drawn = screen
            .picker(
                "Choose",
                &[
                    Choice::new("Group", "").heading(),
                    Choice::new("answer", ""),
                ],
                None,
                "enter selects",
                0,
            )
            .to_string();

        assert!(drawn.contains("> answer"), "{drawn}");
        assert!(!drawn.contains("> Group"), "{drawn}");
    }

    #[test]
    fn overlays_keep_their_window_and_the_form_behind_them() {
        let screen = Screen::full_screen(80, 25, "Install Tectonic");
        let base_fields = [Field::text("Computer name", "deb2")];
        let visible = [0];
        let backdrop = FormFrame {
            fields: &base_fields,
            visible: &visible,
            cursor: 0,
            button: 0,
            mode: &FormMode::Rows,
            actions: &["Install"],
            blocked: None,
            tried: false,
            keys: "enter edits",
            header: &[],
            sections: &[],
        }
        .render(&screen);
        let fields = [Field::secret("Passphrase", "")];
        let actions = ["Open"];
        for (title, expected) in [
            (Some("Enter passphrase"), "Enter passphrase"),
            (None, "Install Tectonic"),
        ] {
            let overlay = screen.overlay(40, 10, title, &backdrop);
            let drawn = FormFrame {
                fields: &fields,
                visible: &visible,
                cursor: 0,
                button: 0,
                mode: &FormMode::Typing,
                actions: &actions,
                blocked: None,
                tried: false,
                keys: "enter submits",
                header: &[],
                sections: &[],
            }
            .render(&overlay);

            assert_eq!(drawn.0[(0, 0)].symbol(), "╭");
            assert_eq!(drawn.0[(79, 24)].symbol(), "╯");
            assert_eq!(drawn.0[(20, 7)].symbol(), "╭");
            assert_eq!(drawn.0[(59, 16)].symbol(), "╯");
            let snapshot = drawn.to_string();
            assert_eq!(
                snapshot.matches("Install Tectonic").count(),
                usize::from(title.is_none()) + 1,
                "{snapshot}"
            );
            assert!(snapshot.contains(expected), "{snapshot}");
            assert!(snapshot.contains("Passphrase"), "{snapshot}");
        }
    }
}
