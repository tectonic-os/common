//! Asks a question from an option list. A question takes one option, several
//! options, or one of two answers.

use crate::ui::chrome::*;
use crate::ui::nest::*;
use crate::ui::progress::*;
use crate::ui::term::*;
use crate::ui::{EITHER, PICK, TOGGLE};
use ratatui::backend::Backend;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::Frame;

#[derive(Clone)]
pub struct Choice {
    pub label: String,
    pub detail: String,
    /// Names the option row this one nests under. The parent comes first. A
    /// parent and its child contradict each other. Two children of one parent
    /// agree.
    pub parent: Option<usize>,
    /// Names the dotted group this option sits inside. A question that takes
    /// several answers draws a group as a collapsed tree. Never set `group` and
    /// `parent` together.
    pub group: String,
    /// An unavailable option still draws and refuses the key that picks it. Its
    /// `detail` says why.
    pub available: bool,
    /// Tells a popup menu to leave a refused option out. Read-only option rows
    /// are unavailable and still stay visible, so `available` cannot supply this.
    pub hidden: bool,
    /// Dim ink reads as absent from the answer. `unavailable` sets this.
    /// `content` leaves it clear.
    pub dim: bool,
    /// Draws the label one character at a time and tints each character by class.
    /// See `tinted`.
    pub tint: bool,
    /// Draws the label as a section heading. See `heading`.
    pub heading: bool,
}

impl Choice {
    pub fn new(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            parent: None,
            group: String::new(),
            available: true,
            hidden: false,
            dim: false,
            tint: false,
            heading: false,
        }
    }

    /// Draws the option and refuses the key that would pick it. The detail beside
    /// it says why.
    pub fn unavailable(mut self) -> Self {
        self.available = false;
        self.dim = true;
        self
    }

    /// Keeps the refusal with the popup menu and out of its option rows.
    pub fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    /// Puts the option on the screen to be read, at full contrast, and refuses
    /// the key that would pick it. Dim ink would read as absent from the answer,
    /// which a LUKS recovery key never is.
    pub fn content(mut self) -> Self {
        self.available = false;
        self
    }

    /// Draws the label one character at a time and tints each character by class.
    /// The LUKS recovery key needs it. That key is 64 characters of hex, held in
    /// no file, copied off the completion screen by eye.
    pub fn tinted(mut self) -> Self {
        self.tint = true;
        self
    }

    /// Draws the label as a section heading over the option rows under it. The
    /// cursor never answers it. The blank line between two sections belongs to the
    /// calling command, not to this widget.
    pub fn heading(mut self) -> Self {
        self.available = false;
        self.heading = true;
        self
    }

    pub fn under(mut self, parent: usize) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Give the options of one group together. A broken run draws the group
    /// heading once for each part of it.
    pub fn within(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }
}

/// Leaving without an answer and answering with nothing are different states.
/// Only the asking command knows whether they mean the same thing.
pub enum Answer {
    Cancelled,
    Chosen(Vec<usize>),
}

/// Limits how many option rows draw at once. Option rows past that count
/// scroll under them.
pub(crate) const VISIBLE: usize = 8;

pub fn select(question: &str, options: &[Choice]) -> Result<Option<usize>, String> {
    select_current(question, options, 0)
}

/// Opens on option `at`, so a question asked again is not navigated again.
pub fn select_current(
    question: &str,
    options: &[Choice],
    at: usize,
) -> Result<Option<usize>, String> {
    inline(height(options.len()), |terminal| {
        pick(terminal, question, options, PICK, at)
    })
}

/// Fixes one width for every overlay window, so the installer screens do not
/// jump as one follows another.
pub const WINDOW_WIDTH: u16 = 68;

/// Draws a plain option list in a fixed-width overlay window. Use it for a
/// selection the form does not keep. Esc answers `None`.
pub fn choose(
    title: &str,
    keys: &str,
    options: &[&str],
    at: usize,
) -> Result<Option<usize>, String> {
    // One padding row over and under the option list, between the two borders.
    let height = options.len() as u16 + 4;
    in_titled_overlay(WINDOW_WIDTH, height, title, || {
        inline(height, |terminal| {
            let mut cursor = at.min(options.len().saturating_sub(1));
            loop {
                // The form keeps no pick here, so the cursor draws as a marker
                // beside the option row and nothing else.
                render(terminal, height, keys, "", |frame, area| {
                    let lines: Vec<Line> = options
                        .iter()
                        .enumerate()
                        .map(|(n, option)| {
                            let (marker, style) = match n == cursor {
                                true => ("> ", Style::new().fg(HIGHLIGHT).bold()),
                                false => ("  ", Style::new()),
                            };
                            Line::from(vec![
                                Span::raw(marker),
                                Span::styled((*option).to_string(), style),
                            ])
                        })
                        .collect();
                    frame.render_widget(Paragraph::new(lines), area);
                })?;
                let Some(key) = read()? else { continue };
                match key {
                    KeyCode::Enter => return Ok(options.get(cursor).map(|_| cursor)),
                    KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
                    KeyCode::Up | KeyCode::Char('k') => cursor = cursor.saturating_sub(1),
                    KeyCode::Down | KeyCode::Char('j') => {
                        cursor = (cursor + 1).min(options.len().saturating_sub(1))
                    }
                    _ => {}
                }
            }
        })
    })
}

/// Draws read-only summary rows and two answers in a fixed overlay window over
/// the screen the decision belongs to. Choosing a disk shows this before that
/// disk is taken. `true` answers the first option. Esc answers the second.
pub fn confirm_over(
    title: &str,
    heading: &str,
    rows: &[(String, String)],
    yes: &str,
    no: &str,
    keys: &str,
) -> Result<bool, String> {
    let mut button = 0usize;
    // The overlay window stands as tall as what it holds. Heading and its blank,
    // the summary rows, a blank, and the two buttons, inside borders and padding.
    let head = usize::from(!heading.is_empty()) * 2;
    let height = (rows.len() + head + 2 + 4) as u16;
    in_titled_overlay(WINDOW_WIDTH, height, title, || {
        inline(height, |terminal| loop {
            let mut lines: Vec<Line<'static>> = Vec::new();
            if !heading.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("  {heading}"),
                    Style::new().fg(Color::White).bold(),
                )));
                lines.push(Line::default());
            }
            let width = rows
                .iter()
                .map(|(name, _)| name.chars().count())
                .max()
                .unwrap_or(0);
            for (name, detail) in rows {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {name:<width$}  "), Style::new()),
                    Span::styled(detail.clone(), Style::new().dim()),
                ]));
            }
            lines.push(Line::default());
            let mut buttons: Vec<Span<'static>> = Vec::new();
            for (at, action) in [yes, no].into_iter().enumerate() {
                let style = match at == button {
                    true => Style::new().fg(HIGHLIGHT).bold().reversed(),
                    false => Style::new(),
                };
                buttons.push(Span::styled(format!("  {action}  "), style));
                buttons.push(Span::raw(" "));
            }
            lines.push(Line::from(buttons));
            render(terminal, height, keys, "", |frame, area| {
                frame.render_widget(Paragraph::new(lines.clone()), area);
            })?;
            let Some(key) = read()? else { continue };
            match key {
                KeyCode::Left | KeyCode::Char('h') => button = button.saturating_sub(1),
                KeyCode::Right | KeyCode::Char('l') => button = (button + 1).min(1),
                KeyCode::Enter => return Ok(button == 0),
                KeyCode::Esc | KeyCode::Char('q') => return Ok(false),
                _ => {}
            }
        })
    })
}

pub fn confirm(question: &str, yes: &str, no: &str) -> Result<bool, String> {
    confirm_current(question, yes, no, true)
}

/// Opens on the existing answer.
pub fn confirm_current(question: &str, yes: &str, no: &str, current: bool) -> Result<bool, String> {
    let options = [Choice::new(yes, ""), Choice::new(no, "")];
    let chosen = inline(height(options.len()), |terminal| {
        pick(terminal, question, &options, EITHER, usize::from(!current))
    })?;
    Ok(chosen == Some(0))
}

/// Answers with any of `options`, or with none. `on` is what is already true,
/// which a question that edits a declaration opens with. Options that carry a
/// `group` draw as a collapsed tree with a filter.
pub fn multi(question: &str, options: &[Choice], on: &[usize]) -> Result<Answer, String> {
    if options.iter().any(|choice| !choice.group.is_empty()) {
        let rows = nodes(options).len();
        return inline(height(rows) + 2, |terminal| {
            nest(terminal, question, options, on)
        });
    }
    inline(height(options.len()), |terminal| {
        toggle(terminal, question, options, on)
    })
}

pub(crate) fn pick<B: Backend>(
    terminal: &mut ratatui::Terminal<B>,
    question: &str,
    options: &[Choice],
    hint: &str,
    selected: usize,
) -> Result<Option<usize>, String> {
    let mut state = ListState::default().with_selected(Some(selected));
    loop {
        render(
            terminal,
            height(options.len()),
            hint,
            question,
            |frame, area| draw(frame, area, question, options, None, hint, &mut state),
        )?;
        let Some(key) = read()? else { continue };
        match key {
            KeyCode::Enter => match state.selected() {
                Some(at) if !available(options, at) => {}
                chosen => return Ok(chosen),
            },
            KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
            code => {
                move_by(code, &mut state);
                skip_spacers(code, options, &mut state);
            }
        }
    }
}

/// An option row with no label is a spacer. The cursor passes over it in the
/// direction it was sent. Only the review screen carries one, above `Create`.
fn skip_spacers(code: KeyCode, options: &[Choice], state: &mut ListState) {
    for _ in 0..options.len() {
        match state.selected() {
            Some(at) if options.get(at).is_some_and(|row| row.label.is_empty()) => {
                move_by(code, state)
            }
            _ => return,
        }
    }
}

pub(crate) fn toggle<B: Backend>(
    terminal: &mut ratatui::Terminal<B>,
    question: &str,
    options: &[Choice],
    held: &[usize],
) -> Result<Answer, String> {
    let mut state = ListState::default().with_selected(Some(0));
    let mut on: Vec<usize> = held.to_vec();
    loop {
        render(
            terminal,
            height(options.len()),
            TOGGLE,
            question,
            |frame, area| {
                draw(
                    frame,
                    area,
                    question,
                    options,
                    Some(&on),
                    TOGGLE,
                    &mut state,
                )
            },
        )?;
        let Some(key) = read()? else { continue };
        match key {
            KeyCode::Enter => return Ok(Answer::Chosen(on)),
            KeyCode::Char(' ') => match state.selected() {
                Some(at) if available(options, at) => flip(&mut on, at, options),
                _ => {}
            },
            KeyCode::Esc | KeyCode::Char('q') => return Ok(Answer::Cancelled),
            code => move_by(code, &mut state),
        }
    }
}

pub(crate) fn available(options: &[Choice], at: usize) -> bool {
    options.get(at).is_some_and(|choice| choice.available)
}

/// Turning an option on clears the parent it contradicts and every child of it.
pub(crate) fn flip(on: &mut Vec<usize>, at: usize, options: &[Choice]) {
    if let Some(held) = on.iter().position(|held| *held == at) {
        on.remove(held);
        return;
    }
    let parent = options[at].parent;
    on.retain(|held| Some(*held) != parent && options[*held].parent != Some(at));
    on.push(at);
}

pub(crate) fn branch(options: &[Choice], at: usize) -> &'static str {
    let Some(parent) = options[at].parent else {
        return "";
    };
    match options[at + 1..]
        .iter()
        .any(|choice| choice.parent == Some(parent))
    {
        true => "\u{251c}\u{2500} ",
        false => "\u{2514}\u{2500} ",
    }
}

pub(crate) fn move_by(code: KeyCode, state: &mut ListState) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => state.select_previous(),
        KeyCode::Down | KeyCode::Char('j') => state.select_next(),
        KeyCode::Home => state.select_first(),
        KeyCode::End => state.select_last(),
        _ => {}
    }
}

/// `on` is None for a question that takes one answer, which marks nothing.
pub(crate) fn draw(
    frame: &mut Frame,
    area: Rect,
    question: &str,
    options: &[Choice],
    on: Option<&[usize]>,
    hint_text: &str,
    state: &mut ListState,
) {
    let heading = head_row(question);
    let [head, body, foot] = Layout::vertical([
        Constraint::Length(u16::from(!heading.is_empty())),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // The heading takes the same palette as the widget box, the progress bar and
    // the step line. A named colour here is one the palette cannot move.
    if !heading.is_empty() {
        frame.render_widget(
            Line::from(Span::styled(
                heading.to_string(),
                Style::new().fg(ACCENT).bold(),
            )),
            head,
        );
    }

    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(at, choice)| {
            let mark = match on {
                None => "",
                Some(on) if on.contains(&at) => "[x] ",
                Some(_) => "[ ] ",
            };
            // A section heading takes the panel section-title style, white and
            // bold.
            let row = match (choice.heading, choice.dim) {
                (true, _) => Style::new().fg(Color::White).bold(),
                (false, true) => Style::new().dim(),
                (false, false) => Style::new(),
            };
            let mut spans = vec![
                Span::styled(mark, row),
                Span::styled(branch(options, at), row),
            ];
            match choice.tint {
                true => spans.extend(tinted(&choice.label)),
                false => spans.push(Span::styled(choice.label.clone(), row)),
            }
            spans.push(Span::raw("  "));
            spans.push(Span::styled(choice.detail.clone(), Style::new().dim()));
            ListItem::new(Line::from(spans))
        })
        .collect();
    let rows = items.len();
    // Inside a widget box a read-only screen sits in the middle as a block.
    // Outside one the option list becomes the viewport and fills it.
    let placed = match CHROME.get().is_some() {
        true => set_in(body, &items, rows),
        false => body,
    };
    // The cursor row takes the palette's hot end, so the answer about to be taken
    // reads the same as the highlighted action button on a form.
    frame.render_stateful_widget(
        List::new(items)
            .highlight_symbol("> ")
            .highlight_style(Style::new().fg(HIGHLIGHT).bold()),
        placed,
        state,
    );
    frame.render_widget(
        Line::from(hint(hint_row(hint_text), state, placed.height, rows).dim()),
        foot,
    );
}

/// Also says how far down a list longer than the window the cursor has reached.
/// Nothing else says there are option rows under the last one on screen.
pub(crate) fn hint(hint: &str, state: &ListState, height: u16, rows: usize) -> String {
    let seen = (state.offset() + usize::from(height)).min(rows);
    match seen < rows || state.offset() > 0 {
        true => format!("{hint}  {seen} of {rows}"),
        false => hint.to_string(),
    }
}
