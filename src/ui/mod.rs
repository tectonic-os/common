//! Keeps ratatui inside the shared crate. Widgets use bounded regions of the
//! normal terminal scroll and never enter an alternate screen. The calling
//! command must find a terminal before drawing.

pub mod table;
#[cfg(feature = "test-support")]
pub mod testing;
pub mod tree;

pub(crate) const PICK: &str =
    "\u{2191}\u{2193} navigate \u{2022} \u{23ce}  select \u{2022} Esc cancel";
pub(crate) const TOGGLE: &str =
    "\u{2191}\u{2193} navigate \u{2022} \u{2423} toggle \u{2022} \u{23ce}  confirm \u{2022} Esc cancel";
pub(crate) const EITHER: &str = "\u{2191}\u{2193} navigate \u{2022} \u{23ce}  confirm";
pub(crate) const EITHER_CANCEL: &str =
    "\u{2191}\u{2193} navigate \u{2022} \u{23ce}  confirm \u{2022} Esc cancel";
/// A form draws its two action buttons side by side, so left and right move
/// between them.
pub const SIDE: &str = "\u{2190}/\u{2192} select \u{2022} \u{23ce}  confirm \u{2022} Esc back";
/// The nested-tree legend names no `j` or `k` key. Every printable key goes to
/// the filter string.
pub(crate) const NEST: &str = "\u{2191}\u{2193} navigate \u{2022} \u{2190}/\u{2192} open \u{2022} \u{2423} toggle \u{2022} \u{23ce}  confirm \u{2022} Esc cancel";
/// Shown while an option list or the inline table holds the keys. Esc closes
/// that list and leaves the form row unanswered.
pub(crate) const SUB_KEYS: &str =
    "\u{2191}\u{2193} navigate \u{2022} \u{23ce}  select \u{2022} Esc back";
/// Enter takes the typed line, and an empty one takes the default answer. If
/// the question has no default, the calling command fails and names the missing
/// flag.
pub(crate) const LINE_KEYS: &str = "\u{23ce}  confirm \u{2022} Esc cancel";
pub(crate) const NO_CHOICES: &str = "No available choices";

mod cells;
mod choose;
mod chrome;
mod form;
mod layout;
mod menu;
mod nest;
mod partition_bar;
mod progress;
mod review;
mod term;

pub use cells::Cell;
pub use choose::{
    choose, confirm, confirm_current, confirm_current_or_no, confirm_or_no, confirm_over, multi,
    select, select_current, Answer, Choice, WINDOW_WIDTH,
};
pub use chrome::{in_overlay, in_titled_overlay, PANEL_ROOM, ROW_ROOM};
pub use form::{form, opens_at, Field, Filled, HeaderLine};
pub use menu::MenuItem;
pub use partition_bar::Hole;
pub use progress::Progress;
pub use review::{decide, line, line_or_empty, offer_over, review};
pub use term::{bold, own_screen, width, INTERRUPTED};

#[cfg(test)]
mod tests;
