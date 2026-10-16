//! Draws the widgets and asserts on them, against copies of the strings the
//! installer and the `tect` CLI pass in.

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

use crate::ui::cells::*;
use crate::ui::choose::*;
use crate::ui::chrome::*;
use crate::ui::form::*;
use crate::ui::layout::*;
use crate::ui::menu::*;
use crate::ui::nest::*;
use crate::ui::partition_bar::*;
use crate::ui::progress::*;
use crate::ui::review::*;
use crate::ui::term::*;
use crate::ui::*;
use copy::INSTALL_KEYS;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::text::Line;
use ratatui::widgets::{ListItem, ListState};
use ratatui::Terminal;

/// The screens these tests draw belong to the installer, which owns its own
/// strings. The copies here let a test assert a widget's geometry against text of
/// the real shape while this crate depends on no calling command.
mod copy {
    pub(super) const DONE_KEYS: &str = "enter to restart, esc to quit the installer";
    pub(super) const EFI_CELL: &str = "efi";
    pub(super) const ERASE_WARNING_SECURE_BOOT: &str =
        "secure boot: the platform key must be cleared";
    pub(super) const ERASE_WARNING_UEFI_SETUP: &str = "afterwards, in UEFI setup: Custom mode";
    pub(super) const FORMAT_TICK: &str = "\u{2713}";
    pub(super) const GO_BACK: &str = "Go back";
    pub(super) const INSTALL: &str = "Install";
    pub(super) const INSTALLATION_SUMMARY: &str = "Installation Summary";
    pub(super) const INSTALL_DONE: &str = "Installation Complete!";
    pub(super) const INSTALL_KEYS: &str = "\u{2191}\u{2193} navigate \u{2022} \u{23ce}  select";
    pub(super) const NEXT_STEPS_HEADING: &str = "Next Steps:";
    pub(super) const OPENED_ADD_KEY: &str = "add a key file for boot";
    pub(super) const OPENED_KEYFILE_PLAIN: &str =
        "the root is not encrypted, so the key would be readable";
    pub(super) const PASSWORD_SET: &str = "set";
    pub(super) const READY: &str = "Ready to install?";
    pub(super) const RECOVERY_HEADING: &str = "LUKS disk encryption recovery key:";
    pub(super) const RESTART_AUTO: &str = "Restart and finalize (automatic)";
    pub(super) const RESTART_MANUAL: &str = "Restart and finalize (manual)";
    pub(super) const AUTO_WINDOW: &str = "until then this key opens the disk without a passphrase";
    pub(super) const ROW_ENCRYPTION: &str = "encryption";
    pub(super) const SHUT_DOWN: &str = "Shut down";
    pub(super) const START_INSTALLATION: &str = "Start Installation";

    pub(super) fn erasing(disk: &str) -> String {
        format!("Everything on {disk} will be erased. Are you sure?")
    }

    pub(super) fn changing_partitions(disk: &str) -> String {
        format!("Partitions marked format on {disk} will be erased. Are you sure?")
    }

    pub(super) fn layout_headings() -> [&'static str; 5] {
        ["size", "filesystem", "format", "type", "mount"]
    }

    pub(super) fn logging(log: Option<&std::path::Path>) -> String {
        match log {
            Some(at) => format!("the install log is at {}", at.display()),
            None => "nothing here is writable, so this screen is the only copy".to_string(),
        }
    }

    pub(super) fn write_down() -> &'static str {
        "Write it down and save it somewhere safe."
    }

    pub(super) fn required_uki_db() -> [&'static str; 3] {
        [
                "Secure-boot with the systemd-boot bootloader requires the image's platform key to be set in this system's UEFI.",
                "",
                "The existing platform key will be deleted, so any existing OS on this system will no longer boot with secure-boot on.",
            ]
    }
}

fn drawn(options: &[Choice], on: Option<&[usize]>, hint: &str, at: usize) -> String {
    let mut state = ListState::default().with_selected(Some(at));
    let height = u16::try_from(options.len() + 3).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(60, height)).unwrap();
    terminal
        .draw(|frame| {
            draw(
                frame,
                frame.area(),
                "which module",
                options,
                on,
                hint,
                &mut state,
            )
        })
        .unwrap();
    terminal.backend().to_string()
}

fn tree() -> Vec<Choice> {
    vec![
        Choice::new("linux-desktop", ""),
        Choice::new("dx", "").under(0),
        Choice::new("gaming", "").under(0),
        Choice::new("linux-server", ""),
    ]
}
