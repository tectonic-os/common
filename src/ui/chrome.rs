use crate::ui::progress::*;
use crate::ui::term::*;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, ListItem, Padding};
use ratatui::Frame;

/// Green marks a firmware state that holds. Amber marks one that does not. Both
/// are chosen to read on the installer's dark console.
pub(crate) const GOOD: Color = Color::Rgb(0x6c, 0xe0, 0x7b);
pub(crate) const AMBER: Color = Color::Rgb(0xff, 0xbf, 0x00);
/// Red marks a form value that contradicts another answer. The machine reports
/// no such state. The user's own two answers disagree.
pub(crate) const ALERT: Color = Color::Rgb(0xff, 0x5c, 0x57);

/// The width cap keeps prose readable and still fits the partition table
/// columns.
pub(crate) const WIDEST: u16 = 92;

/// The calling command wraps panel prose to the narrowest supported console,
/// even when the widget box is wider.
pub const PANEL_ROOM: usize = (if WIDEST < NARROWEST as u16 {
    WIDEST
} else {
    NARROWEST as u16
} - 2
    - 2 * PAD_X) as usize;

/// An option-list row carries a two-column cursor marker beside it, so its prose
/// wraps short of the panel room.
pub const ROW_ROOM: usize = PANEL_ROOM - 2;

/// Installer console cells are twice as tall as they are wide, so a square box
/// needs half as many rows as columns.
const CELL: (u16, u16) = (8, 16);

/// Every colour on the installer screen comes from the progress bar's gradient.
/// The widget box takes its cold end and the cursor row takes its hot end.
pub(crate) const ACCENT: Color = Color::Rgb(COLD.0, COLD.1, COLD.2);
pub(crate) const HIGHLIGHT: Color = Color::Rgb(HOT.0, HOT.1, HOT.2);

/// Holds the room between the box border and its content. A box drawn tight
/// around the content reads as a table.
pub(crate) const PAD_X: u16 = 3;
pub(crate) const PAD_Y: u16 = 1;

thread_local! {
    /// Holds the overlay size while `in_overlay`'s body runs. The widget then
    /// draws as a small fixed window over the form it belongs to.
    pub(crate) static OVERLAY: std::cell::Cell<Option<(u16, u16)>> = const {
        std::cell::Cell::new(None)
    };
    /// Holds the form screen drawn last. An overlay window opens over that
    /// buffer. A new terminal starts blank and would otherwise show through.
    pub(crate) static BACKDROP: std::cell::RefCell<Option<Buffer>> = const {
        std::cell::RefCell::new(None)
    };
    /// Holds an overlay title that says more than the command title does, such as
    /// `Enter passphrase` over a form titled by the installer.
    static OVERLAY_TITLE: std::cell::RefCell<Option<String>> = const {
        std::cell::RefCell::new(None)
    };
    /// Alt held shows a `Secret` field in the clear. A terminal that reports the
    /// modifier keys keeps the reveal momentary. A terminal that reports Alt only
    /// with the key it was held with cannot say when Alt was released.
    pub(crate) static REVEAL: std::cell::Cell<bool> = const {
        std::cell::Cell::new(false)
    };
    /// Holds the typing caret's blink step while the form draws. The colour
    /// walks the accent gradient with it.
    pub(crate) static CARET: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
    /// A render call owns whether its frame carries the widget box. The scoped
    /// value lets an in-memory backend compose the same frame without changing
    /// the process-wide terminal mode.
    static FRAME_BOX: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

struct FrameBox(Option<bool>);

impl Drop for FrameBox {
    fn drop(&mut self) {
        FRAME_BOX.with(|boxed| boxed.set(self.0));
    }
}

pub(crate) fn in_frame_box<T>(boxed: bool, body: impl FnOnce() -> T) -> T {
    let previous = FRAME_BOX.with(|held| held.replace(Some(boxed)));
    let _restore = FrameBox(previous);
    body()
}

pub(crate) fn boxed() -> bool {
    FRAME_BOX
        .with(std::cell::Cell::get)
        .unwrap_or_else(|| has_box(CHROME.get().map(String::as_str)))
}

pub(crate) fn has_box(title: Option<&str>) -> bool {
    title.is_some() || OVERLAY_TITLE.with(|held| held.borrow().is_some())
}

struct OverlayFrame {
    size: Option<(u16, u16)>,
    title: Option<String>,
    backdrop: Option<Option<Buffer>>,
}

impl Drop for OverlayFrame {
    fn drop(&mut self) {
        OVERLAY.with(|held| held.set(self.size));
        OVERLAY_TITLE.with(|held| {
            held.replace(self.title.take());
        });
        if let Some(backdrop) = self.backdrop.take() {
            BACKDROP.with(|held| {
                held.replace(backdrop);
            });
        }
    }
}

pub(crate) fn in_overlay_frame<T>(
    width: u16,
    height: u16,
    title: Option<&str>,
    backdrop: Option<&Buffer>,
    body: impl FnOnce() -> T,
) -> T {
    let _restore = OverlayFrame {
        size: OVERLAY.with(|held| held.replace(Some((width, height)))),
        title: OVERLAY_TITLE.with(|held| held.replace(title.map(str::to_string))),
        backdrop: backdrop
            .map(|backdrop| BACKDROP.with(|held| held.replace(Some(backdrop.clone())))),
    };
    body()
}

/// Runs a widget as a small fixed window, centred, instead of the main box. Use
/// it for a screen that overlaps the form behind it.
pub fn in_overlay<T>(
    width: u16,
    height: u16,
    body: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    in_overlay_frame(width, height, None, None, body)
}

/// Runs the same overlay window under a title of its own.
pub fn in_titled_overlay<T>(
    width: u16,
    height: u16,
    title: &str,
    body: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    in_overlay_frame(width, height, Some(title), None, body)
}

/// Paints the widget box where a command owns the console, then answers with the
/// room left inside it.
///
/// The title arrives as a parameter and is never read from `CHROME`. `CHROME` is
/// a `OnceLock` and `cargo test` is one process. A test that set it would put a
/// title bar on every other test drawing in parallel.
///
/// `head` is the widget's own question. If the box draws a border, the question
/// moves onto it. Otherwise the command title keeps the top row.
pub(crate) fn chrome(
    frame: &mut Frame,
    title: Option<&str>,
    head: &str,
    rows: u16,
    keys: &str,
) -> Rect {
    // A new terminal starts blank, so the form behind an overlay is painted in
    // again before the overlay window.
    if OVERLAY.with(std::cell::Cell::get).is_some() {
        if let Some(back) = BACKDROP.with(|saved| saved.borrow().clone()) {
            let area = frame.area();
            let width = area.width.min(back.area.width);
            let height = area.height.min(back.area.height);
            for y in 0..height {
                for x in 0..width {
                    frame.buffer_mut()[(x, y)] = back[(x, y)].clone();
                }
            }
        }
    }
    // An overlay names itself where it has the room. The form box behind it
    // already carries the command title.
    let held = OVERLAY_TITLE.with(|held| held.borrow().clone());
    let title = match OVERLAY.with(std::cell::Cell::get).is_some() {
        true => held.as_deref().or(title),
        false => title,
    };
    let Some(title) = title else {
        return frame.area();
    };
    let wording = match head.trim() {
        "" => title.trim(),
        head => head,
    };
    let full = frame.area();
    // The height matches the width in console pixels, so the box reads square. A
    // taller widget still gets its rows. A short console caps the height in
    // `centred`, so the box remains usable without staying square.
    let (width, tall) = match OVERLAY.with(std::cell::Cell::get) {
        Some((width, height)) => (width.min(full.width), height.min(full.height)),
        None => {
            let width = WIDEST.min(full.width);
            let square = width.saturating_mul(CELL.0) / CELL.1;
            (width, rows.saturating_add(2 + 2 * PAD_Y).max(square))
        }
    };
    let box_area = centred(full, width, tall);
    let block = Block::new()
        .borders(Borders::ALL)
        // The installer media draws through kmscon. Where kmscon cannot start,
        // systemd hands tty1 back to the kernel VT, whose bitmap font has no arc
        // glyphs and draws the box corners as `+`.
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .padding(Padding::symmetric(PAD_X, PAD_Y))
        // The box title starts one cell in, so the corner reads as a corner with a
        // line coming off it. Flush against the words it reads as a bracket.
        .title(Line::from(vec![
            Span::styled("\u{2500} ", Style::new().fg(ACCENT)),
            Span::styled(wording.to_string(), Style::new().bold()),
            Span::raw(" "),
        ]))
        // The key legend sits on the bottom border and the title sits on the top.
        // Inside the box the legend would cost a row and read as content.
        .title_bottom(
            Line::from(vec![
                Span::styled("\u{2500} ", Style::new().fg(ACCENT)),
                Span::styled(keys.to_string(), Style::new().fg(TRACK)),
                Span::raw(" "),
            ])
            .left_aligned(),
        );
    let inside = block.inner(box_area);
    // The overlay window is opaque. The form behind it is painted around it. A box
    // drawn over what the overlay does not cover reads as a hole.
    frame.render_widget(Clear, box_area);
    frame.render_widget(block, box_area);
    inside
}

/// Returns nothing where the widget box already draws the key legend on its
/// bottom border.
pub(crate) fn hint_row(keys: &str) -> &str {
    match boxed() {
        true => "",
        false => keys,
    }
}

/// Returns nothing where the widget box's top border already draws the question.
pub(crate) fn head_row(question: &str) -> &str {
    match boxed() {
        true => "",
        false => question,
    }
}

pub(crate) fn centred(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

/// Centres a read-only screen on both axes. It takes the widest row, which
/// already holds its cursor column, and its own rows where the widget box has
/// them to give.
pub(crate) fn set_in(body: Rect, items: &[ListItem], rows: usize) -> Rect {
    let width = items.iter().map(ListItem::width).max().unwrap_or(0) as u16;
    centred(body, width, rows.min(usize::from(body.height)) as u16)
}
