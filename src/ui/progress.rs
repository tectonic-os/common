//! Draws the progress region for an install. It shows a gauge, the running
//! step, and the tail of the install log.

use crate::ui::chrome::*;
use crate::ui::term::*;
use ratatui::crossterm::terminal;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{DefaultTerminal, Frame};

/// Keeps this many log messages under the gauge. The message pane works as a
/// console and the install log file keeps the record, so the pane holds the last
/// few screens of log rather than the whole install.
const LOGGED: usize = 256;

/// Counts the rows the region takes. They are the progress bar, the step line,
/// the message pane under its rule, and the fixed foot line.
const ROWS: u16 = 9;

/// Holds a bounded region over work that takes a while. It stays open across
/// fisherman's event stream, which is why it is a handle. Every other widget in
/// this crate is a closure.
pub struct Progress {
    terminal: DefaultTerminal,
    /// Holds the install percentage finished before the step now running.
    pct: u16,
    /// Holds the share of the whole install the running step carries.
    flight: u16,
    /// Counts the log messages that arrived during the running step. They are the
    /// only signal for how far into that step the machine has got.
    within: u32,
    /// Names the spinner frame now drawn.
    turn: usize,
    step: String,
    notes: Vec<String>,
    foot: String,
}

impl Progress {
    /// `foot` is the one line that stays put under the message pane. It names
    /// where the install transcript is written, and what cancel means now.
    ///
    /// Raw mode is turned off again. Nothing reads the keyboard while the progress
    /// region is open, and raw mode would make Ctrl+C a key nothing reads. A
    /// region held over an hour of work that can hang must stay interruptible.
    pub fn open(foot: &str) -> Result<Self, String> {
        let terminal = open(ROWS)?;
        let _ = terminal::disable_raw_mode();
        Ok(Self {
            terminal,
            pct: 0,
            flight: 0,
            within: 0,
            turn: 0,
            step: String::new(),
            notes: Vec::new(),
            foot: foot.to_string(),
        })
    }

    /// Moves the gauge to the next step. Log messages stay on screen. The message
    /// pane works as a console, and the install is watched from the tail of it.
    pub fn step(&mut self, pct: u16, flight: u16, name: &str) -> Result<(), String> {
        self.pct = pct.min(100);
        self.flight = flight.min(100);
        self.within = 0;
        self.step = name.to_string();
        self.show()
    }

    /// Gives how far along the whole install the progress bar is drawn.
    ///
    /// fisherman reports what a step weighs and never how far into it the machine
    /// has got, and one step carries most of the weight. Each log message during a
    /// step takes a fixed share of what is left of that step, so the percentage
    /// climbs and never reaches where the next step begins. fisherman emits no
    /// sub-step progress, so this counts the messages instead.
    fn at(&self) -> u16 {
        crept(self.pct, self.flight, self.within)
    }

    /// Redraws for time passing and nothing else. A fisherman step can hold the
    /// machine for minutes between two log messages. A screen unchanged for that
    /// long reads as a stopped one, so the spinner moves.
    pub fn tick(&mut self) -> Result<(), String> {
        self.turn = self.turn.wrapping_add(1);
        self.show()
    }

    /// Adds one log message under the gauge and drops the oldest.
    pub fn note(&mut self, text: &str) -> Result<(), String> {
        self.within += 1;
        if self.notes.len() == LOGGED {
            self.notes.remove(0);
        }
        self.notes.push(text.to_string());
        self.show()
    }

    fn show(&mut self) -> Result<(), String> {
        let (pct, turn, step, notes, foot) = (
            self.at(),
            self.turn,
            self.step.as_str(),
            self.notes.as_slice(),
            self.foot.as_str(),
        );
        // Inside the widget box the top border carries the step and the spinner.
        // Inline there is no border, and `working` draws the step line itself.
        let head = match CHROME.get().is_some() && !step.is_empty() {
            true => step_line(turn, step),
            false => String::new(),
        };
        render(&mut self.terminal, ROWS, foot, &head, |frame, area| {
            working(frame, area, pct, turn, step, notes, foot)
        })
    }

    pub fn close(self) {
        close(self.terminal);
    }
}

pub(crate) fn working(
    frame: &mut Frame,
    area: Rect,
    pct: u16,
    turn: usize,
    step: &str,
    notes: &[String],
    foot: &str,
) {
    // A blank row above the progress bar gives the widget box a margin, and one
    // more sets the step line apart from the step it names. The message pane takes
    // everything left over, which inside the installer's box is most of the screen.
    let keys = hint_row(foot);
    let [_, meter, name, _, body, tail] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(u16::from(!keys.is_empty())),
    ])
    .areas(area);
    frame.render_widget(bar(pct, meter.width), meter);
    // Inline there is no border, so the spinner and the step draw here. Inside the
    // widget box the top border carries them, and this row stays the one blank
    // line between the progress bar and the message pane.
    if CHROME.get().is_none() {
        frame.render_widget(
            Line::from(vec![
                Span::styled(TURNING[turn % TURNING.len()], Style::new().fg(HIGHLIGHT)),
                Span::raw(" "),
                Span::styled(step.to_string(), Style::new().fg(HIGHLIGHT).bold()),
            ]),
            name,
        );
    }
    // The message pane holds the tail of the install log, so it shows what just
    // happened rather than the oldest lines the region still has.
    let room = usize::from(body.height);
    let first = notes.len().saturating_sub(room);
    frame.render_widget(Paragraph::new(notes[first..].join("\n")).dim(), body);
    frame.render_widget(Line::from(keys.dim()), tail);
}

/// Fixes the two ends of the progress gradient, which are also the two ends of
/// the installer palette. `ACCENT` and `HIGHLIGHT` take the same two values, so nothing
/// on the screen drifts away from the bar.
pub(crate) const COLD: (u8, u8, u8) = (0x5a, 0x56, 0xe0);
pub(crate) const HOT: (u8, u8, u8) = (0xee, 0x6f, 0xf8);

/// Adds the install percentage finished before this step to a share of the step
/// weight for each log message that arrived during it. Held out of `Progress` so
/// a test can read it without a terminal.
pub(crate) fn crept(pct: u16, flight: u16, within: u32) -> u16 {
    // The `install OS` step copies a blob per image layer and there are dozens,
    // so each message's share must stay small enough that a hundred of them do
    // not run out of bar.
    let left = HELD.powi(within.min(400) as i32);
    pct + (f64::from(flight) * (1.0 - left)) as u16
}

/// Leaves this share of a step after one more log message during it.
const HELD: f64 = 0.97;

/// Draws one fill that only grows, with the percentage after it. `Progress::at`
/// advances it. The fill runs solid the whole way across on a grey track, and
/// uses no partial blocks. A serial console has none, and the kernel VT the
/// installer media falls back to has grey as its own colour 8.
pub(crate) fn bar<'a>(pct: u16, width: u16) -> Line<'a> {
    let label = format!(" {pct:>3}%");
    let room = usize::from(width).saturating_sub(label.chars().count());
    let done = room * usize::from(pct.min(100)) / 100;
    let mut spans: Vec<Span> = (0..room)
        .map(|at| match at < done {
            true => Span::styled("\u{2588}", Style::new().fg(blend(at, room))),
            false => Span::styled("\u{2588}", Style::new().fg(TRACK)),
        })
        .collect();
    spans.push(Span::styled(label, Style::new().bold()));
    Line::from(spans)
}

/// Colours the unfilled length of the progress bar and the key legend on the box
/// border.
pub(crate) const TRACK: Color = Color::DarkGray;

/// Turns a spinner beside the running step. It draws braille, which the
/// installer media's kmscon draws and a kernel VT has no glyphs for.
pub(crate) const TURNING: [&str; 10] = [
    "\u{280b}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283c}", "\u{2834}", "\u{2826}", "\u{2827}",
    "\u{2807}", "\u{280f}",
];

/// Formats the step and spinner as the widget box's top border reads them,
/// `⠹ 6/10 Installing OS`.
pub(crate) fn step_line(turn: usize, step: &str) -> String {
    format!("{} {step}", TURNING[turn % TURNING.len()])
}

fn blend(at: usize, room: usize) -> Color {
    let step = |cold: u8, hot: u8| {
        let (cold, hot) = (i32::from(cold), i32::from(hot));
        (cold + (hot - cold) * at as i32 / room.max(1) as i32) as u8
    };
    Color::Rgb(
        step(COLD.0, HOT.0),
        step(COLD.1, HOT.1),
        step(COLD.2, HOT.2),
    )
}

/// Draws a label one character at a time. Digits take white, letters take the
/// palette's hot pink, and both draw bold. Three classes, because the character
/// pairs the user confuses (`0` and `O`, `1` and `l`, `5` and `S`) hold one
/// character from each class. Colour separates what shape does not.
pub(crate) fn tinted(label: &str) -> Vec<Span<'static>> {
    label
        .chars()
        .map(|letter| {
            let colour = match letter {
                _ if letter.is_ascii_digit() => Color::White,
                _ => HIGHLIGHT,
            };
            Span::styled(letter.to_string(), Style::new().fg(colour).bold())
        })
        .collect()
}
