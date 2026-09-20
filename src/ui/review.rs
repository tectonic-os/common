//! Draws back what a command collected, so the user can decide on it. It draws
//! answer summaries, destructive confirmations, and typed lines.

use crate::ui::choose::*;
use crate::ui::chrome::*;
use crate::ui::table;
use crate::ui::term::*;
use crate::ui::{LINE_KEYS, SIDE};
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Draws no question head. Form rows say what they are, and the widget box title
/// says what is being filled in.
pub(crate) fn sheet_of(
    frame: &mut Frame,
    area: Rect,
    lines: &[Line<'static>],
    keys: &str,
    focus: usize,
    tail: usize,
) {
    // This widget reserves the legend row only where it draws the key legend
    // itself. Inside a widget box the bottom border carries that legend, and a
    // blank row under the action buttons would hold nothing.
    let keys = hint_row(keys);
    let [body, foot] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(u16::from(!keys.is_empty())),
    ])
    .areas(area);
    // Overlay rows start at the top, one padding row inside the border. Centring
    // made the content walk down when an opened option list or a hidden question
    // changed its height, and an overlay that jumps under the cursor is
    // unreadable.
    let height = usize::from(body.height);
    // Action buttons close the form. Where the widget box has room to spare, the
    // answers fill the top and the buttons sit one line above the box's own
    // padding.
    let mut shown = lines.to_vec();
    let mut focus = focus;
    if shown.len() < height {
        let pad = height - shown.len();
        for _ in 0..pad {
            shown.insert(tail, Line::default());
        }
        if focus >= tail {
            focus += pad;
        }
    }
    let lines = shown;
    let scroll = focus
        .saturating_sub(height.saturating_sub(1))
        .min(lines.len().saturating_sub(height));
    frame.render_widget(
        Paragraph::new(lines.to_vec()).scroll((scroll as u16, 0)),
        body,
    );
    frame.render_widget(Line::from(keys.dim()), foot);
}

/// Draws the answers a command has collected, one row per piece of
/// configuration, with `action` under them. `Some(rows.len())` names `action`. A
/// smaller value names the summary row to ask again. `None` means cancel. The
/// screen is sized to its summary rows, which the command's questions bound, so
/// the action cannot scroll off. `blocked` gives the reason the action cannot be
/// taken yet, and the action then draws dim and unpickable with that reason
/// beside it.
pub fn review(
    question: &str,
    rows: &[(String, String)],
    action: &str,
    keys: &str,
    blocked: Option<&str>,
    at: usize,
) -> Result<Option<usize>, String> {
    let options = sheet(rows, action, blocked);
    let chosen = inline((options.len() + 2) as u16, |terminal| {
        pick(
            terminal,
            question,
            &options,
            keys,
            at.min(options.len() - 1),
        )
    })?;
    // The cursor never lands on the spacer row, so any index past the last summary
    // row names the action.
    Ok(chosen.map(|at| at.min(rows.len())))
}

/// Counts the lines `decide_draw` paints, which sizes `decide`'s widget box. It
/// counts the heading row where no box carries it, the ready line, the wrapped
/// cost line with a blank on each side, the warning rows with one trailing blank,
/// and the answer rows. Held out of `decide` so a later change to the cost block
/// or the warning block cannot mis-size the inline box without a test catching it.
pub(crate) fn decide_content_height(
    heading: &str,
    note: &str,
    warning: &[&str],
    rows: &[(String, String)],
) -> usize {
    // The heading takes a row of its own only where no widget box carries it. The
    // cost line keeps a blank on both sides, so it reads as a cost and never as
    // another configuration row.
    let cost = match note.is_empty() {
        true => 0,
        false => table::wrap(note, PANEL_ROOM).len() + 2,
    };
    // Warning rows are already short enough for the widget box. They need no wrap
    // of their own, only the blank line that sets them apart.
    let warned = match warning.is_empty() {
        true => 0,
        false => warning.len() + 1,
    };
    rows.len() + 3 + cost + warned + usize::from(!head_row(heading).is_empty())
}

/// Asks a question over a read-only summary of what answering it would do. The
/// two answers sit side by side under the summary rows. `true` answers `yes`.
/// Esc answers `no`.
pub fn decide(
    heading: &str,
    note: &str,
    warning: &[&str],
    rows: &[(String, String)],
    ready: &str,
    yes: &str,
    no: &str,
) -> Result<bool, String> {
    let content = decide_content_height(heading, note, warning, rows);
    let mut button = 0;
    inline((content + 2) as u16, |terminal| loop {
        if !decide_fits(
            content,
            terminal
                .size()
                .map_err(|err| err.to_string())?
                .height
                .into(),
        ) {
            return Err(
                "the confirmation is too tall to show every destructive change".to_string(),
            );
        }
        render(terminal, content as u16, SIDE, heading, |frame, area| {
            decide_draw(
                frame, area, heading, note, warning, rows, ready, yes, no, button,
            )
        })?;
        let Some(key) = read()? else { continue };
        match key {
            KeyCode::Left | KeyCode::Up | KeyCode::Char('h') | KeyCode::Char('k') => button = 0,
            KeyCode::Right | KeyCode::Down | KeyCode::Char('l') | KeyCode::Char('j') => {
                button = 1;
            }
            KeyCode::Enter => return Ok(button == 0),
            KeyCode::Esc | KeyCode::Char('q') => return Ok(false),
            _ => {}
        }
    })
}

/// A destructive confirmation never leaves a summary row below the terminal's
/// fold. It refuses the screen and never lets an unseen row become an accepted
/// action.
pub(crate) fn decide_fits(content: usize, height: usize) -> bool {
    content.saturating_add(2) <= height
}

/// Draws the ready line, the cost line, the summary rows, then the two actions
/// side by side. The action enter takes carries the palette's hot end, as a form
/// draws it.
pub(crate) fn decide_draw(
    frame: &mut Frame,
    area: Rect,
    heading: &str,
    note: &str,
    warning: &[&str],
    rows: &[(String, String)],
    ready: &str,
    yes: &str,
    no: &str,
    button: usize,
) {
    let head = head_row(heading);
    let mut lines: Vec<Line> = Vec::new();
    if !head.is_empty() {
        lines.push(Line::from(Span::styled(
            head.to_string(),
            Style::new().fg(ACCENT).bold(),
        )));
    }
    lines.push(Line::from(Span::styled(
        ready.to_string(),
        Style::new().fg(Color::White).bold(),
    )));
    if !note.is_empty() {
        // The longest cost sentence runs past the widget box on a real disk name,
        // so it wraps where panel prose wraps and is never cut. A blank line sets
        // it apart from the question and from the summary rows.
        lines.push(Line::default());
        for line in table::wrap(note, PANEL_ROOM) {
            lines.push(Line::from(Span::styled(line, Style::new())));
        }
        lines.push(Line::default());
    }
    // The must-read fact takes the same warning colour the firmware panel uses for
    // a state that does not hold. It sits with the question, ahead of the dim
    // summary rows, so the user cannot mistake it for one more of them.
    for line in warning {
        lines.push(Line::from(Span::styled(
            (*line).to_string(),
            Style::new().fg(AMBER).bold(),
        )));
    }
    if !warning.is_empty() {
        lines.push(Line::default());
    }
    let width = rows
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);
    for (label, value) in rows {
        lines.push(Line::from(vec![
            Span::styled(format!("{label:<width$}"), Style::new()),
            Span::raw("  "),
            Span::styled(value.clone(), Style::new().dim()),
        ]));
    }
    let mut buttons: Vec<Span<'static>> = vec![Span::raw("  ")];
    for (at, action) in [yes, no].into_iter().enumerate() {
        let style = match at == button {
            true => Style::new().fg(HIGHLIGHT).bold().reversed(),
            false => Style::new(),
        };
        buttons.push(Span::styled(format!("  {action}  "), style));
        buttons.push(Span::raw(" "));
    }
    let footer = vec![Line::default(), Line::from(buttons)];

    // Inside a widget box this draws as a block in the middle of it. Outside one
    // it fills the region. A summary longer than the box keeps its action buttons
    // and gives up its narrowest rows. The answers are the last thing to lose.
    let widest = lines
        .iter()
        .chain(&footer)
        .map(Line::width)
        .max()
        .unwrap_or(0) as u16;
    let placed = match CHROME.get().is_some() {
        true => {
            let all = lines.len() as u16 + footer.len() as u16;
            centred(area, widest, all)
        }
        false => area,
    };
    let [body, act] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(footer.len() as u16)])
            .areas(placed);
    frame.render_widget(Paragraph::new(lines), body);
    frame.render_widget(Paragraph::new(footer), act);
}

/// Offers a single action over read-only rows. Esc is the way out, and `keys`
/// says so. A command that sets `CHROME` opens full screen and paints over
/// the output the command printed before it.
pub fn offer_over(
    question: &str,
    rows: Vec<Choice>,
    action: &str,
    keys: &str,
) -> Result<bool, String> {
    let mut options = rows;
    options.push(Choice::new("", ""));
    options.push(Choice::new(action, ""));
    let at = options.len() - 1;
    let chosen = inline((options.len() + 2) as u16, |terminal| {
        pick(terminal, question, &options, keys, at)
    })?;
    Ok(chosen == Some(at))
}

/// Pads the row labels to one column, adds a spacer row, then the action. The
/// action draws dim and unpickable while any answer it needs is missing.
fn sheet(rows: &[(String, String)], action: &str, blocked: Option<&str>) -> Vec<Choice> {
    let width = rows
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);
    let mut options: Vec<Choice> = rows
        .iter()
        .map(|(label, value)| Choice::new(format!("{label:<width$}"), value))
        .collect();
    options.push(Choice::new("", ""));
    options.push(match blocked {
        None => Choice::new(action, ""),
        Some(why) => Choice::new(action, why).unavailable(),
    });
    options
}

/// Shows a line as it is typed, with a default standing in until the user types
/// over it. Enter answers with what is there. Esc answers with nothing, and the
/// calling command then takes its default or fails and names its flag. `prefix`
/// stands before the answer and is not part of it.
pub fn line(question: &str, prefix: &str, default: Option<&str>) -> Result<String, String> {
    inline(3, |terminal| {
        let mut typed = String::new();
        loop {
            render(terminal, 3, LINE_KEYS, question, |frame, area| {
                written(frame, area, question, prefix, &typed, default)
            })?;
            let Some(key) = read()? else { continue };
            match key {
                KeyCode::Enter => return Ok(typed),
                KeyCode::Esc => return Ok(String::new()),
                KeyCode::Backspace => {
                    typed.pop();
                }
                KeyCode::Char(letter) => typed.push(letter),
                _ => {}
            }
        }
    })
}

/// Draws the default dim where the typed answer will be, because enter takes it.
pub(crate) fn written(
    frame: &mut Frame,
    area: Rect,
    question: &str,
    prefix: &str,
    typed: &str,
    default: Option<&str>,
) {
    // Inside the widget box the question sits on the border, so it takes no row
    // here.
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
    let answer = match (typed.is_empty(), default) {
        (true, Some(default)) => Span::styled(default, Style::new().dim()),
        _ => Span::raw(typed),
    };
    frame.render_widget(Line::from(vec![Span::raw(prefix), answer]), body);
    frame.render_widget(Line::from(hint_row(LINE_KEYS).dim()), foot);
}
