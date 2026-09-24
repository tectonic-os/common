//! Reports the terminal facts a widget draws against. It measures the console
//! width, says whether a terminal watches the output, and opens the viewport.

use crate::ui::choose::*;
use crate::ui::chrome::*;
use ratatui::backend::Backend;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::crossterm::terminal;
use ratatui::layout::Rect;
use ratatui::{DefaultTerminal, Frame, TerminalOptions, Viewport};
use std::io::IsTerminal;
use std::sync::OnceLock;

/// Assumes this console width when the terminal reports none.
pub(crate) const NARROWEST: usize = 80;

/// Reports whether a terminal watches the drawn output. The calling command uses
/// it to decide colour. They also use it to decide whether a read-out prints as a table
/// or as the markdown a file would hold.
pub(crate) fn colour() -> bool {
    std::io::stdout().is_terminal()
}

/// Asks the terminal only where stdout is one, so a redirected run and a piped
/// run draw the same width. `COLUMNS` supplies the width a terminal withholds.
pub fn width() -> usize {
    if !colour() {
        return NARROWEST;
    }
    std::env::var("COLUMNS")
        .ok()
        .and_then(|cols| parse_width(&cols).map(usize::from))
        .or_else(|| {
            terminal::size()
                .map(|(cols, _)| usize::from(cols))
                .ok()
                .filter(|cols| *cols > 0)
        })
        .unwrap_or(NARROWEST)
}

pub(crate) fn parse_width(width: &str) -> Option<u16> {
    width.parse().ok().filter(|width| *width > 0)
}

/// Emits the ANSI bold sequence for every kind of stdout. Only the drawn read-out path
/// calls it, and `Prompt::draws` has already gated that path.
pub fn bold(text: &str) -> String {
    ratatui::crossterm::style::Stylize::bold(text).to_string()
}

/// Counts the rows a question takes inside a bounded region. If a command owns
/// the screen, the viewport is the whole terminal and nothing calls this.
pub(crate) fn height(rows: usize) -> u16 {
    (rows.min(VISIBLE) + 2) as u16
}

/// A serial console starts at 0x0 and never sends `SIGWINCH`. A ratatui viewport
/// laid out for that size draws nothing, so the console size is set here.
fn give_size() {
    if !unsized_tty(terminal::size().ok()) {
        return;
    }
    let size = libc::winsize {
        ws_row: 24,
        ws_col: NARROWEST as u16,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCSWINSZ, &size) };
}

pub(crate) fn unsized_tty(size: Option<(u16, u16)>) -> bool {
    !matches!(size, Some((cols, rows)) if cols > 0 && rows > 0)
}

/// Holds the title bar of a command that owns the console. Setting it also puts
/// every widget in a full-screen viewport. If it stays unset, a widget draws in a
/// bounded region of the terminal scroll and paints no box.
pub(crate) static CHROME: OnceLock<String> = OnceLock::new();

/// Call once, at the entry of a command that owns the screen. Nothing reads the
/// title back and nothing can unset it.
pub fn own_screen(title: impl Into<String>) {
    let _ = CHROME.set(title.into());
}

/// Opens the viewport, turns raw mode on, and sets the size a serial console
/// withholds. Paired with `close`, which every path out of a widget goes through.
pub(crate) fn open(height: u16) -> Result<DefaultTerminal, String> {
    give_size();
    let viewport = match CHROME.get() {
        Some(_) => Viewport::Fullscreen,
        None => Viewport::Inline(height),
    };
    ratatui::try_init_with_options(TerminalOptions { viewport }).map_err(|err| err.to_string())
}

/// Clears the region and returns the cursor to its start, so what the calling
/// command prints next lands where the region was.
pub(crate) fn close(mut terminal: DefaultTerminal) {
    let origin = terminal.get_frame().area().as_position();
    let _ = terminal.clear();
    let _ = terminal.set_cursor_position(origin);
    let _ = terminal.show_cursor();
    // `ratatui::restore` would also leave an alternate screen this crate never
    // entered.
    let _ = terminal::disable_raw_mode();
}

/// Clears the region again before it returns, so what the calling command prints
/// next lands where the region was.
pub(crate) fn inline<T>(
    height: u16,
    body: impl FnOnce(&mut DefaultTerminal) -> Result<T, String>,
) -> Result<T, String> {
    let mut terminal = open(height)?;
    let out = body(&mut terminal);
    close(terminal);
    out
}

/// Gives a widget the frame it draws in and calls `terminal.draw` once. If a
/// command owns the console, the widget gets the inside of the box. Otherwise it
/// gets the whole frame.
///
/// `rows` is what the widget asks for. Inline, the viewport is already the
/// region and `rows` is ignored. Inside a box, `rows` sizes and centres that box.
pub(crate) fn render<B: Backend>(
    terminal: &mut ratatui::Terminal<B>,
    rows: u16,
    keys: &str,
    head: &str,
    body: impl FnOnce(&mut Frame, Rect),
) -> Result<(), String> {
    terminal
        .draw(|frame| {
            let area = chrome(frame, CHROME.get().map(String::as_str), head, rows, keys);
            body(frame, area);
        })
        .map(|_| ())
        .map_err(|err| err.to_string())
}

/// Reads one key, but gives up after `ms` and answers `None`. A widget that
/// draws a moving caret uses this to redraw without a key in between.
pub(crate) fn read_for(ms: u64) -> Result<Option<KeyCode>, String> {
    let ready = event::poll(std::time::Duration::from_millis(ms)).map_err(|err| err.to_string())?;
    match ready {
        true => read(),
        false => Ok(None),
    }
}

/// Ctrl+C in raw mode arrives as a key. A widget returns this error to say a
/// user wants out. The calling command must read it.
pub const INTERRUPTED: &str = "interrupted";

/// Reports whether Alt is held. Alt held shows a secret in the clear.
pub(crate) fn revealed() -> bool {
    REVEAL.with(std::cell::Cell::get)
}

/// Returns None for an event that is not a key. Returns an error only for the
/// interrupt.
pub(crate) fn read() -> Result<Option<KeyCode>, String> {
    let Event::Key(key) = event::read().map_err(|err| err.to_string())? else {
        return Ok(None);
    };
    // Alt marks a reveal and never a key binding, so every event it arrives on is
    // consumed. See `REVEAL` for what each terminal reports.
    let alt = key.modifiers.contains(KeyModifiers::ALT)
        || matches!(
            key.code,
            KeyCode::Modifier(event::ModifierKeyCode::LeftAlt | event::ModifierKeyCode::RightAlt)
        );
    match key.kind {
        KeyEventKind::Press | KeyEventKind::Repeat if alt => {
            REVEAL.with(|reveal| reveal.set(true));
            return Ok(None);
        }
        KeyEventKind::Release if alt => {
            REVEAL.with(|reveal| reveal.set(false));
            return Ok(None);
        }
        // The next key without Alt takes the reveal away, which is the only
        // release a terminal that never reports Alt alone will give.
        KeyEventKind::Press | KeyEventKind::Repeat => {
            REVEAL.with(|reveal| reveal.set(false));
        }
        _ => return Ok(None),
    }
    if key.kind != KeyEventKind::Press {
        return Ok(None);
    }
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Err(INTERRUPTED.to_string())
        }
        code => Ok(Some(code)),
    }
}
