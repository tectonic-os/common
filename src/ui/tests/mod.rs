//! Draws the widgets and asserts on them, against copies of the strings the
//! installer and the `tect` CLI pass in.

mod cells;
mod choose;
mod chrome;
mod form;
mod layout;
mod menu;
mod nest;
mod progress;
mod review;

use crate::ui::cells::*;
use crate::ui::choose::*;
use crate::ui::chrome::*;
use crate::ui::form::*;
use crate::ui::layout::*;
use crate::ui::menu::*;
use crate::ui::nest::*;
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
    pub const DONE_KEYS: &str = "enter to restart, esc to quit the installer";
    pub const EFI_CELL: &str = "efi";
    pub const ERASE_WARNING_SECURE_BOOT: &str = "secure boot: the platform key must be cleared";
    pub const ERASE_WARNING_UEFI_SETUP: &str = "afterwards, in UEFI setup: Custom mode";
    pub const FORMAT_TICK: &str = "\u{2713}";
    pub const GO_BACK: &str = "Go back";
    pub const INSTALL: &str = "Install";
    pub const INSTALLATION_SUMMARY: &str = "Installation Summary";
    pub const INSTALL_DONE: &str = "Installation Complete!";
    pub const INSTALL_KEYS: &str = "\u{2191}\u{2193} navigate \u{2022} \u{23ce}  select";
    pub const NEXT_STEPS_HEADING: &str = "Next Steps:";
    pub const OPENED_ADD_KEY: &str = "add a key file for boot";
    pub const OPENED_KEYFILE_PLAIN: &str =
        "the root is not encrypted, so the key would be readable";
    pub const PASSWORD_SET: &str = "set";
    pub const READY: &str = "Ready to install?";
    pub const RECOVERY_HEADING: &str = "LUKS disk encryption recovery key:";
    pub const RESTART: &str = "Restart now";
    pub const ROW_ENCRYPTION: &str = "encryption";
    pub const SHUT_DOWN: &str = "Shut down";
    pub const START_INSTALLATION: &str = "Start Installation";

    pub fn erasing(disk: &str) -> String {
        format!("Everything on {disk} will be erased. Are you sure?")
    }

    pub fn changing_partitions(disk: &str) -> String {
        format!("Partitions marked format on {disk} will be erased. Are you sure?")
    }

    pub fn layout_headings() -> [&'static str; 5] {
        ["size", "filesystem", "format", "type", "mount"]
    }

    pub fn logging(log: Option<&std::path::Path>) -> String {
        match log {
            Some(at) => format!("the install log is at {}", at.display()),
            None => "nothing here is writable, so this screen is the only copy".to_string(),
        }
    }

    pub fn write_down() -> &'static str {
        "Write it down and save it somewhere safe."
    }

    pub fn required_uki_db() -> [&'static str; 3] {
        [
                "Secure-boot with the systemd-boot bootloader requires the image's platform key to be set in this system's UEFI.",
                "",
                "The existing platform key will be deleted, so any existing OS on this system will no longer boot with secure-boot on.",
            ]
    }
}

fn drawn(options: &[Choice], on: Option<&[usize]>, hint: &str, at: usize) -> String {
    let mut state = ListState::default().with_selected(Some(at));
    let height = options.len() as u16 + 3;
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
