//! Draws the cells of a form's inline table and sizes their columns.

use crate::ui::chrome::*;
use ratatui::style::{Color, Style};
use ratatui::text::Span;

/// An answered table cell draws white. An unanswered table cell draws dim.
#[derive(Clone)]
pub struct Cell {
    pub(crate) text: String,
    pub(crate) set: bool,
}

impl Cell {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            set: false,
        }
    }

    /// Marks the cell answered by the user, which draws it white.
    pub fn set(text: &str) -> Self {
        Self {
            text: text.to_string(),
            set: true,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn answered(&self) -> bool {
        self.set
    }
}

/// Pads each table column to its widest cell or heading. Shaves those widths
/// until the row, its separators and the two-column cursor marker fit the form
/// panel, so one long answer cannot push the other columns off the console.
/// `room` is what is left after the row prefix and the label column, which the
/// calling widget takes off first.
pub(crate) fn column_widths(headings: &[&str], rows: &[Vec<Cell>], room: usize) -> Vec<usize> {
    let columns = rows
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(0)
        .max(headings.len());
    let mut widths: Vec<usize> = (0..columns)
        .map(|column| {
            let cells = rows
                .iter()
                .filter_map(|row| row.get(column))
                .map(|cell| cell.text.chars().count())
                .max()
                .unwrap_or(0);
            // A heading wider than its own cells would push the table columns
            // right.
            cells.max(
                headings
                    .get(column)
                    .map_or(0, |heading| heading.chars().count()),
            )
        })
        .collect();
    let room = room.saturating_sub(2 * columns.saturating_sub(1));
    while widths.iter().sum::<usize>() > room {
        let Some(widest) = widths
            .iter()
            .enumerate()
            .max_by_key(|(_, width)| **width)
            .map(|(at, _)| at)
        else {
            break;
        };
        if widths[widest] == 0 {
            break;
        }
        widths[widest] -= 1;
    }
    widths
}

/// Drops the first cell of each table row. The form draws that cell as the row
/// name in its label column.
pub(crate) fn value_cells(rows: &[Vec<Cell>]) -> Vec<Vec<Cell>> {
    rows.iter()
        .map(|row| row.iter().skip(1).cloned().collect())
        .collect()
}

/// Heads the value columns. The form's label column carries the field label, so
/// its heading stays out of this row.
pub(crate) fn table_headings(headings: &[&str], widths: &[usize]) -> String {
    headings
        .iter()
        .enumerate()
        .map(|(column, heading)| {
            let width = widths.get(column).copied().unwrap_or(0);
            let text: String = clipped(heading, width);
            format!("{text:<width$}")
        })
        .collect::<Vec<String>>()
        .join("  ")
}

/// Ends an over-long cell with `...`. A hard cut reads as a different word. A
/// disk model name is the usual cause. A cell ending in a parenthetical keeps
/// that whole, because a partition row's node is the half the user reads.
pub(crate) fn clipped(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    if let Some(at) = text.rfind(" (") {
        let tail = &text[at + 1..];
        let tail_len = tail.chars().count();
        // The head needs one character and the ellipsis before the tail.
        if tail.ends_with(')') && tail_len + 4 <= width {
            let head: String = text.chars().take(width - tail_len - 3).collect();
            return format!("{head}...{tail}");
        }
    }
    match width < 3 {
        true => text.chars().take(width).collect(),
        false => format!("{}...", text.chars().take(width - 3).collect::<String>()),
    }
}

/// Inks one table row. The first cell is the row name and draws in the form's
/// label column. Branch glyphs keep dim ink at every row state.
pub(crate) fn table_row_spans(
    row: &[Cell],
    first_width: usize,
    widths: &[usize],
    hot: bool,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let Some(first) = row.first() else {
        return spans;
    };
    let text = clipped(&first.text, first_width);
    // A child device inside a container carries the vertical line of the table
    // row above it.
    let branch: String = text
        .chars()
        .take_while(|c| matches!(c, '\u{251c}' | '\u{2514}' | '\u{2500}' | '\u{2502}' | ' '))
        .collect();
    let device: String = text.chars().skip(branch.chars().count()).collect();
    spans.push(Span::styled(branch.clone(), Style::new().dim()));
    let ink = match (hot, first.set) {
        (true, _) => Style::new().fg(HIGHLIGHT).bold(),
        (false, true) => Style::new().fg(Color::White),
        (false, false) => Style::new().dim(),
    };
    spans.push(Span::styled(
        format!(
            "{device:<rest$}",
            rest = first_width.saturating_sub(branch.chars().count())
        ),
        ink,
    ));
    for (column, cell) in row.iter().enumerate().skip(1) {
        let width = widths.get(column - 1).copied().unwrap_or(0);
        let text: String = clipped(&cell.text, width);
        let style = match cell.set {
            true => Style::new().fg(Color::White),
            false => Style::new().dim(),
        };
        spans.push(Span::raw("  "));
        spans.push(Span::styled(format!("{text:<width$}"), style));
    }
    spans
}
