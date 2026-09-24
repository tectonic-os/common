//! Draws where a new partition lands inside the free space of a disk.

use crate::ui::chrome::ALERT;
use crate::ui::progress::{blend, TRACK};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use std::ops::Range;

/// Holds one free region of a disk and the partitions on either side of it.
/// If no partition bounds a side, then that side is `None` and the region
/// runs to the start or the end of the disk. `gb` is the whole GB a partition
/// can take from the region.
#[derive(Debug, PartialEq)]
pub struct Hole {
    pub before: Option<String>,
    pub after: Option<String>,
    pub gb: u64,
}

/// Colours the boxes of the neighbour partitions. The mid grey sits between
/// the dark grey of the free region and the black of the labels, so both read
/// against it.
pub(crate) const NEIGHBOUR: Color = Color::Rgb(0xa0, 0xa0, 0xa0);

/// Draws the free region a new partition goes in as one bar, with the new
/// partition at its offset inside it. The bar takes the lowest region with
/// room for `offset` and `size`, because the calling command places the
/// partition by that rule. If no region has room, then the bar takes the
/// largest region and the partition draws red to the region's end.
pub(crate) fn placed(holes: &[Hole], offset: u64, size: u64, width: usize) -> Vec<Span<'static>> {
    let end = offset.saturating_add(size);
    let Some((hole, fits)) = chosen(holes, offset, size) else {
        return Vec::new();
    };
    // The neighbour boxes do not scale with the neighbour partitions. Each box
    // takes at most a quarter of the bar, so the free region keeps half the
    // bar and the new partition inside it stays readable.
    let before = hole.before.as_deref().map(|label| boxed(label, width / 4));
    let after = hole.after.as_deref().map(|label| boxed(label, width / 4));
    let taken = [&before, &after]
        .iter()
        .flat_map(|side| side.iter())
        .map(Vec::len)
        .sum::<usize>();
    let room = width.saturating_sub(taken);
    let cell = |gb: u64| {
        let gb = u128::from(gb.min(hole.gb));
        (gb * room as u128 / u128::from(hole.gb.max(1))) as usize
    };
    let mut spans = before.unwrap_or_default();
    if size == 0 {
        spans.extend(painted(0..room, &said(hole.gb), |_| TRACK));
    } else {
        // A partition smaller than one cell still takes one, so a typed size
        // always draws.
        let from = cell(offset).min(room.saturating_sub(1));
        let to = cell(end).max(from + 1).min(room);
        let partition = |at| match fits {
            true => blend(at, room),
            false => ALERT,
        };
        spans.extend(painted(0..from, &said(offset), |_| TRACK));
        spans.extend(painted(from..to, &said(size), partition));
        spans.extend(painted(
            to..room,
            &said(hole.gb.saturating_sub(end)),
            |_| TRACK,
        ));
    }
    spans.extend(after.unwrap_or_default());
    spans
}

/// Picks the free region the bar draws and reports whether the partition fits
/// in it. The bar and the room the window states both read this answer, so
/// the stated room is always the room of the region drawn.
pub(crate) fn chosen(holes: &[Hole], offset: u64, size: u64) -> Option<(&Hole, bool)> {
    let end = offset.saturating_add(size);
    match holes.iter().find(|hole| end <= hole.gb) {
        Some(hole) => Some((hole, true)),
        None => holes
            .iter()
            .max_by_key(|hole| hole.gb)
            .map(|hole| (hole, false)),
    }
}

/// Draws one neighbour partition's label in a mid grey box of at most `most`
/// cells. The label is cut to leave one cell of box on each side of it.
fn boxed(label: &str, most: usize) -> Vec<Span<'static>> {
    let width = (label.chars().count() + 2).min(most);
    let label: String = label.chars().take(width.saturating_sub(2)).collect();
    painted(0..width, &label, |_| NEIGHBOUR)
}

/// Formats a size the way the form's measure fields show it.
pub(crate) fn said(gb: u64) -> String {
    format!("{gb}.0 GB")
}

/// Paints the bar cells in `cells` with `text` centred on them in black.
/// `colour` takes each cell's place in the bar, which is what the gradient of
/// the new partition runs along.
///
/// A cell without text draws a solid block in the fill colour, as the
/// progress bar does. The kernel VT has no dark grey background, so only the
/// text cells depend on one. If the text does not fit with one cell of fill on
/// each side, then the segment draws no text, because a label touching the
/// next segment reads as part of it.
fn painted(cells: Range<usize>, text: &str, colour: impl Fn(usize) -> Color) -> Vec<Span<'static>> {
    let letters: Vec<char> = match text.chars().count() + 2 <= cells.len() {
        true => text.chars().collect(),
        false => Vec::new(),
    };
    let lead = cells.start + (cells.len() - letters.len()) / 2;
    cells
        .map(
            |at| match at.checked_sub(lead).and_then(|index| letters.get(index)) {
                Some(letter) => Span::styled(
                    letter.to_string(),
                    Style::new().fg(Color::Black).bg(colour(at)),
                ),
                None => Span::styled("\u{2588}", Style::new().fg(colour(at))),
            },
        )
        .collect()
}
