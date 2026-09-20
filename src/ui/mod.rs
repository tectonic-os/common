//! Draws the terminal widgets the `tect` CLI and the bootc installer share.
//!
//! ratatui appears in this crate only. A widget draws in a bounded region of the
//! normal terminal scroll. No widget enters an alternate screen. The calling
//! command must find a terminal before it draws.

pub mod table;
pub mod tree;

pub(crate) const PICK: &str = "up and down to move, enter to choose, esc cancels";
pub(crate) const TOGGLE: &str = "space toggles, enter confirms, esc cancels";
pub(crate) const EITHER: &str = "up and down to move, enter to answer";
/// A form draws its two action buttons side by side, so left and right move
/// between them.
pub const SIDE: &str = "left and right to move, enter to answer";
/// The nested-tree legend names no `j` or `k` key. Every printable key goes to
/// the filter string.
pub(crate) const NEST: &str = "filter, space toggles, ←/→ opens, enter confirms";
/// Shown while an option list or the inline table holds the keys. Esc closes
/// that list and leaves the form row unanswered.
pub(crate) const SUB_KEYS: &str =
    "\u{2191}\u{2193} navigate \u{2022} \u{23ce}  select \u{2022} Esc back";
/// The typed-line legend names no esc key. Enter takes the default answer. If
/// the question has no default, the calling command fails and names the missing
/// flag.
pub(crate) const LINE_KEYS: &str = "enter confirms";
pub(crate) const NO_CHOICES: &str = "No available choices";

mod cells;
mod choose;
mod chrome;
mod form;
mod layout;
mod menu;
mod nest;
mod progress;
mod review;
mod term;

pub use cells::Cell;
pub use choose::{
    choose, confirm, confirm_current, confirm_over, multi, select, select_current, Answer, Choice,
    WINDOW_WIDTH,
};
pub use chrome::{in_overlay, in_titled_overlay, PANEL_ROOM, ROW_ROOM};
pub use form::{form, opens_at, Field, Filled, HeaderLine};
pub use menu::MenuItem;
pub use progress::Progress;
pub use review::{decide, line, offer_over, review};
pub use term::{bold, own_screen, width, INTERRUPTED};

#[cfg(test)]
mod tests;
