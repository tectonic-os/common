use super::*;

/// Covers the serial console. It comes up 0x0 and stays there, and a ratatui
/// viewport laid out for that size draws nothing at all.
#[test]
fn a_terminal_that_reports_no_size_is_given_one() {
    assert!(unsized_tty(None));
    assert!(unsized_tty(Some((0, 0))));
    assert!(unsized_tty(Some((80, 0))));
    assert!(!unsized_tty(Some((80, 24))));
}

#[test]
fn a_form_scrolls_to_keep_its_focused_line_visible() {
    let lines: Vec<Line<'static>> = (0..12)
        .map(|row| Line::from(format!("row {row}")))
        .collect();
    let mut terminal = Terminal::new(TestBackend::new(20, 4)).unwrap();
    terminal
        .draw(|frame| sheet_of(frame, frame.area(), &lines, "", 11, 0))
        .unwrap();
    let drawn = terminal.backend().to_string();
    assert!(drawn.contains("row 11"), "{drawn}");
    assert!(!drawn.contains("row 0"), "{drawn}");
}

/// The completion screen draws the LUKS recovery key. fisherman generates that
/// key at install time, keeps it out of the install log on purpose, and writes it
/// to no file, so this screen holds the only copy.
#[test]
fn the_completion_screen_draws_the_recovery_key() {
    // 32 random bytes as hex, which fisherman's RandomPassphrase hands back for a
    // `tpm2-luks` install.
    const KEY: &str = "6f1b4c0d2a9e83f57b6c1d40e2578a93bb0e4f21c7d68a5039e1b74c2f8d605a";
    let options: Vec<Choice> = vec![
        Choice::new(copy::RECOVERY_HEADING, "").heading(),
        Choice::new(copy::write_down(), "").content(),
        Choice::new("", ""),
        Choice::new(KEY, "").content().tinted(),
        Choice::new(copy::NEXT_STEPS_HEADING, "").heading(),
        Choice::new("set secure boot to setup mode", "").content(),
        Choice::new(
            copy::logging(Some(std::path::Path::new("/var/log/tect-install.log"))),
            "",
        )
        .content(),
        Choice::new("", ""),
        Choice::new(copy::RESTART, ""),
    ];
    // Every option row under the headings reads and answers nothing. The spacer
    // is a gap and holds no row. The cursor lands on the action alone.
    for (at, choice) in options.iter().enumerate().take(options.len() - 2) {
        if choice.label.is_empty() {
            continue;
        }
        assert!(!choice.available, "row {at} is landable");
        assert!(!choice.dim, "row {at} is dim");
    }
    assert!(available(&options, options.len() - 1));

    let at = options.len() - 1;
    let mut state = ListState::default().with_selected(Some(at));
    let mut terminal = Terminal::new(TestBackend::new(72, options.len() as u16 + 2)).unwrap();
    terminal
        .draw(|frame| {
            draw(
                frame,
                frame.area(),
                copy::INSTALL_DONE,
                &options,
                None,
                copy::DONE_KEYS,
                &mut state,
            )
        })
        .unwrap();
    let drawn = terminal.backend().to_string();
    assert!(drawn.contains(KEY), "the key is not on the screen: {drawn}");
    assert!(drawn.contains("/var/log/tect-install.log"), "{drawn}");
    assert!(drawn.contains(copy::RESTART), "{drawn}");

    // Both headings read white and bold, as the panel sections do. A blank row
    // sets the recovery key apart from the line above it.
    let buffer = terminal.backend().buffer();
    for heading in [copy::RECOVERY_HEADING, copy::NEXT_STEPS_HEADING] {
        let row = drawn
            .lines()
            .position(|line| line.contains(heading))
            .unwrap_or_else(|| panic!("{heading} is not on the screen: {drawn}"));
        let cell = (0..buffer.area.width)
            .map(|column| &buffer[(column, row as u16)])
            .find(|cell| !cell.symbol().trim().is_empty())
            .unwrap_or_else(|| panic!("{heading} has no ink: {drawn}"));
        assert_eq!(cell.fg, Color::White, "{heading} is not white: {drawn}");
        assert!(
            cell.modifier.contains(Modifier::BOLD),
            "{heading} is not bold: {drawn}"
        );
    }

    // The recovery key draws at full contrast, one character at a time, bold. A
    // refused option draws dim, and this key is the one string on the screen that
    // leaves with the user.
    let row = KEY_ROW;
    let lit = (0..buffer.area.width)
        .map(|column| &buffer[(column, row)])
        .filter(|cell| !cell.symbol().trim().is_empty())
        .collect::<Vec<_>>();
    assert_eq!(
        lit.len(),
        KEY.len(),
        "the key's row is not the key: {drawn}"
    );
    for cell in &lit {
        assert!(
            !cell.modifier.contains(Modifier::DIM),
            "the recovery key is drawn dim: {drawn}"
        );
        assert!(
            cell.modifier.contains(Modifier::BOLD),
            "the recovery key is not bold: {drawn}"
        );
    }

    // Letters and digits take a tint each, so the user tells `0` from `O` on a
    // screen where nothing else can check them. Digits draw white and letters
    // draw the same pink the highlight uses.
    let tint = |class: fn(&char) -> bool| {
        KEY.chars()
            .zip(&lit)
            .filter(|(letter, _)| class(letter))
            .map(|(_, cell)| cell.fg)
            .collect::<std::collections::HashSet<_>>()
    };
    let digits = tint(|letter| letter.is_ascii_digit());
    let letters = tint(|letter| letter.is_alphabetic());
    assert_eq!(digits, std::collections::HashSet::from([Color::White]));
    assert_eq!(letters, std::collections::HashSet::from([HIGHLIGHT]));

    // `Restart now` is the one option row the cursor can sit on, and it carries
    // the palette's highlight colour.
    let row = 1 + at as u16;
    let lit = (0..buffer.area.width)
        .map(|column| &buffer[(column, row)])
        .filter(|cell| !cell.symbol().trim().is_empty())
        .collect::<Vec<_>>();
    assert!(!lit.is_empty(), "{drawn}");
    for cell in &lit {
        assert_eq!(cell.fg, HIGHLIGHT, "{drawn}");
    }
}

/// The question takes the first row. The recovery heading and `write_down` come
/// under it, then a blank, then the key.
const KEY_ROW: u16 = 4;

/// The last question before a disk is wiped. It carries what installing costs.
/// The disk, what happens to that disk, and every answer about to be acted on.
/// The two answers sit side by side, and the one enter would take lights the way
/// a form lights its action button.
#[test]
fn the_confirmation_shows_the_answers_and_lights_the_one_enter_takes() {
    let rows = [
        ("disk".to_string(), "/dev/vda".to_string()),
        ("computer name".to_string(), "deb2".to_string()),
        ("password".to_string(), copy::PASSWORD_SET.to_string()),
    ];
    let note = copy::erasing("/dev/vda");
    let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();

    for (button, lit) in [(0, copy::START_INSTALLATION), (1, copy::GO_BACK)] {
        terminal
            .draw(|frame| {
                decide_draw(
                    frame,
                    frame.area(),
                    copy::INSTALLATION_SUMMARY,
                    &note,
                    &[],
                    &rows,
                    copy::READY,
                    copy::START_INSTALLATION,
                    copy::GO_BACK,
                    button,
                )
            })
            .unwrap();
        let drawn = terminal.backend().to_string();
        assert!(drawn.contains(copy::INSTALLATION_SUMMARY), "{drawn}");
        assert!(drawn.contains(copy::READY), "{drawn}");
        assert!(drawn.contains("will be erased. Are you sure?"), "{drawn}");
        assert!(
            drawn.contains("/dev/vda") && drawn.contains("deb2"),
            "{drawn}"
        );
        assert!(drawn.contains(copy::START_INSTALLATION), "{drawn}");
        assert!(drawn.contains(copy::GO_BACK), "{drawn}");

        // Reversed cells arrive in one run here, and that run holds the answer
        // enter would take. The other answer stays plain.
        let buffer = terminal.backend().buffer().clone();
        let row = (0..buffer.area.height)
            .find(|row| {
                let text: String = (0..buffer.area.width)
                    .map(|column| buffer[(column, *row)].symbol().to_string())
                    .collect();
                text.contains(copy::START_INSTALLATION)
            })
            .expect("the answers row is not drawn");
        let taken: String = (0..buffer.area.width)
            .filter(|column| buffer[(*column, row)].modifier.contains(Modifier::REVERSED))
            .map(|column| buffer[(column, row)].symbol().to_string())
            .collect();
        assert_eq!(taken.trim(), lit, "{drawn}");
    }
}

/// Cost note and warning each add their own block of lines to the inline widget
/// box, and neither block depends on the other. A change that made one block eat
/// into the other's count, or dropped one when both arrive, fails this.
#[test]
fn the_decide_box_sizes_its_note_and_warning_blocks_independently() {
    let rows = [("computer name".to_string(), "deb2".to_string())];
    let note = copy::erasing("/dev/vda");
    let warning = [
        copy::ERASE_WARNING_SECURE_BOOT,
        copy::ERASE_WARNING_UEFI_SETUP,
    ];
    let heading = copy::INSTALLATION_SUMMARY;
    let bare = decide_content_height(heading, "", &[], &rows);
    let note_extra = decide_content_height(heading, &note, &[], &rows) - bare;
    let warning_extra = decide_content_height(heading, "", &warning, &rows) - bare;
    assert_eq!(
        decide_content_height(heading, &note, &warning, &rows),
        bare + note_extra + warning_extra,
        "note and warning must add up rather than interact"
    );
    // Both blocks come back non-zero, or the additive check above would pass
    // vacuously.
    assert!(note_extra > 0);
    assert!(warning_extra > 0);
}

#[test]
fn a_confirmation_taller_than_the_console_has_no_action() {
    assert!(decide_fits(23, 25));
    assert!(!decide_fits(24, 25));
}

/// A must-read fact ahead of the erase draws in the warning colour, above the dim
/// summary rows. An empty warning draws no line at all.
#[test]
fn the_confirmation_shows_a_warning_ahead_of_the_dim_summary() {
    // A summary row whose text appears nowhere in the erase question above it, so
    // the search below finds the summary row and never the cost note.
    let rows = [("computer name".to_string(), "deb2".to_string())];
    let warning = [copy::ERASE_WARNING_SECURE_BOOT];
    let mut terminal = Terminal::new(TestBackend::new(70, 12)).unwrap();
    terminal
        .draw(|frame| {
            decide_draw(
                frame,
                frame.area(),
                copy::INSTALLATION_SUMMARY,
                &copy::erasing("/dev/vda"),
                &warning,
                &rows,
                copy::READY,
                copy::START_INSTALLATION,
                copy::GO_BACK,
                0,
            )
        })
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    let warn_row = (0..buffer.area.height)
        .find(|row| {
            (0..buffer.area.width)
                .map(|column| buffer[(column, *row)].symbol().to_string())
                .collect::<String>()
                .contains(copy::ERASE_WARNING_SECURE_BOOT)
        })
        .expect("the warning is not drawn");
    let summary_row = (0..buffer.area.height)
        .find(|row| {
            (0..buffer.area.width)
                .map(|column| buffer[(column, *row)].symbol().to_string())
                .collect::<String>()
                .contains("deb2")
        })
        .expect("the summary row is not drawn");
    assert!(
        warn_row < summary_row,
        "warning must read before the summary"
    );
    assert_eq!(buffer[(0, warn_row)].fg, AMBER);

    let mut plain = Terminal::new(TestBackend::new(70, 12)).unwrap();
    plain
        .draw(|frame| {
            decide_draw(
                frame,
                frame.area(),
                copy::INSTALLATION_SUMMARY,
                &copy::erasing("/dev/vda"),
                &[],
                &rows,
                copy::READY,
                copy::START_INSTALLATION,
                copy::GO_BACK,
                0,
            )
        })
        .unwrap();
    let drawn = plain.backend().to_string();
    assert!(!drawn.contains(copy::ERASE_WARNING_SECURE_BOOT), "{drawn}");
}

/// The cost sentence can run past the widget box on a real disk name, and a
/// summary can stand taller than the room it draws in. Neither may cost the
/// user the answers, so the cost note wraps and the summary rows give way.
#[test]
fn a_confirmation_that_does_not_fit_still_wraps_its_note_and_keeps_its_answers() {
    let one = [("disk".to_string(), "/dev/nvme0n1".to_string())];
    let mut narrow = Terminal::new(TestBackend::new(70, 8)).unwrap();
    narrow
        .draw(|frame| {
            decide_draw(
                frame,
                frame.area(),
                "",
                &copy::changing_partitions("/dev/nvme0n1"),
                &[],
                &one,
                copy::READY,
                copy::START_INSTALLATION,
                copy::GO_BACK,
                0,
            )
        })
        .unwrap();
    let drawn = narrow.backend().to_string();
    assert!(drawn.contains("sure?"), "{drawn}");

    let many: Vec<(String, String)> = (0..20)
        .map(|at| (format!("row {at}"), "value".to_string()))
        .collect();
    let mut short = Terminal::new(TestBackend::new(40, 10)).unwrap();
    short
        .draw(|frame| {
            decide_draw(
                frame,
                frame.area(),
                "",
                "",
                &[],
                &many,
                copy::READY,
                copy::START_INSTALLATION,
                copy::GO_BACK,
                0,
            )
        })
        .unwrap();
    let drawn = short.backend().to_string();
    assert!(drawn.contains(copy::START_INSTALLATION), "{drawn}");
    assert!(drawn.contains(copy::GO_BACK), "{drawn}");
}

fn typed_line(prefix: &str, typed: &str, default: Option<&str>) -> String {
    let mut terminal = Terminal::new(TestBackend::new(60, 3)).unwrap();
    terminal
        .draw(|frame| written(frame, frame.area(), "who owns it", prefix, typed, default))
        .unwrap();
    terminal.backend().to_string()
}

/// The default answer stands where the typed answer will be until the user types
/// over it. The prefix stands before both.
#[test]
fn a_default_is_shown_until_something_is_typed_over_it() {
    let empty = typed_line("github.com/", "", Some("someone"));
    assert!(empty.contains("github.com/someone"), "{empty}");
    assert!(empty.contains(LINE_KEYS), "{empty}");

    let over = typed_line("github.com/", "else", Some("someone"));
    assert!(over.contains("github.com/else"), "{over}");
    assert!(!over.contains("someone"), "{over}");

    // A question with no default leaves the answer line empty.
    let bare = typed_line("", "", None);
    assert!(bare.contains("who owns it"), "{bare}");
    // The test backend quotes each row, so the quotes come off before reading it.
    let answer = bare.lines().nth(1).unwrap().replace('"', "");
    assert_eq!(answer.trim(), "", "{bare}");
}
