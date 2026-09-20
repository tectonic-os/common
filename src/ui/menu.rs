//! Opens a popup menu for one inline-table row. Taking a menu item answers the
//! row. If the item carries children, it opens them in the same overlay window
//! instead.

use crate::ui::chrome::*;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

/// Holds one item of the popup menu.
#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub children: Vec<String>,
}

impl MenuItem {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            children: Vec::new(),
        }
    }

    pub fn under(label: &str, children: &[&str]) -> Self {
        Self {
            label: label.to_string(),
            children: children.iter().map(|child| child.to_string()).collect(),
        }
    }
}

pub(crate) fn menu_rows(items: &[MenuItem], open: Option<usize>) -> Vec<(usize, Option<usize>)> {
    let mut rows = Vec::new();
    for (at, item) in items.iter().enumerate() {
        rows.push((at, None));
        if open == Some(at) {
            rows.extend((0..item.children.len()).map(|child| (at, Some(child))));
        }
    }
    rows
}

/// Fixes the overlay-window size. The longest menu the installer draws sets it,
/// and that menu is the Format list with all of its children. The window then
/// keeps its shape as answers change.
const MENU_WIDTH: u16 = 36;
const MENU_HEIGHT: u16 = 16;

pub(crate) fn menu_draw(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    items: &[MenuItem],
    open: Option<usize>,
    cursor: usize,
) {
    let visible = menu_rows(items, open);
    let text: Vec<String> = visible
        .iter()
        .map(|(item, child)| match child {
            None => items[*item].label.clone(),
            Some(child) => format!("  {}", items[*item].children[*child]),
        })
        .collect();
    let popup = centred(
        area,
        MENU_WIDTH.min(area.width),
        MENU_HEIGHT.min(area.height),
    );
    frame.render_widget(Clear, popup);
    let mut shown = title.chars().collect::<String>();
    shown.truncate((popup.width as usize).saturating_sub(4));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .title(Line::from(vec![
            Span::raw("─ "),
            Span::styled(shown, Style::new().bold()),
            Span::raw(" "),
        ]));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let lines: Vec<Line> = visible
        .iter()
        .zip(text)
        .enumerate()
        .map(|(row, (_, label))| {
            let marker = match row == cursor {
                true => "> ",
                false => "  ",
            };
            let style = match row == cursor {
                true => Style::new().fg(HIGHLIGHT).bold(),
                false => Style::new(),
            };
            Line::from(vec![Span::raw(marker), Span::styled(label, style)])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}
