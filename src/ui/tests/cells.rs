use super::*;

/// The inline table draws three inks. The cursor row's disk cell takes the
/// highlight, an answered cell draws white, and an unanswered cell draws dim.
/// Table headings line up with the columns they name.
#[test]
fn the_inline_table_marks_the_cursor_and_which_answers_are_set() {
    let rows = vec![
        vec![
            Cell::set("/dev/vda"),
            Cell::new("64G"),
            Cell::new(""),
            Cell::new(""),
            Cell::new(""),
            Cell::new(""),
        ],
        vec![
            Cell::new("\u{2514}\u{2500} /dev/vda1"),
            Cell::new("512M"),
            Cell::set("vfat"),
            Cell::set(copy::EFI_CELL),
            Cell::new(""),
            Cell::set("/boot/efi"),
        ],
        vec![
            Cell::set("\u{2514}\u{2500} /dev/vda2"),
            Cell::new("60G"),
            Cell::set("ext4"),
            Cell::set(copy::FORMAT_TICK),
            Cell::new(""),
            Cell::set("/var"),
        ],
    ];
    let fields = [Field::table(
        "disk",
        "disk",
        &copy::layout_headings(),
        &[],
        rows,
        vec![true, true, true],
        vec![Vec::new(); 3],
        1,
        false,
        true,
    )];
    let (lines, _, _) = laid_out(
        &fields,
        &[0],
        0,
        0,
        &Mode::Table,
        &[],
        None,
        false,
        &[],
        &[],
    );
    let mut terminal = Terminal::new(TestBackend::new(80, 8)).unwrap();
    terminal
        .draw(|frame| sheet_of(frame, frame.area(), &lines, INSTALL_KEYS, 0, 0))
        .unwrap();
    let drawn = terminal.backend().to_string();
    let buffer = terminal.backend().buffer();
    let row_text = |row: u16| {
        (0..buffer.area.width)
            .map(|column| buffer[(column, row)].symbol().to_string())
            .collect::<String>()
    };
    let row_of = |text: &str| {
        (0..buffer.area.height)
            .find(|row| row_text(*row).contains(text))
            .unwrap_or_else(|| panic!("{text} is not drawn: {drawn}"))
    };
    let cell_at = |text: &str| {
        let row = row_of(text);
        let line = row_text(row);
        // Table columns count characters and never bytes. Branch glyphs run to
        // more than one byte each.
        let column = line[..line.find(text).expect("the column")].chars().count() as u16;
        &buffer[(column, row)]
    };
    let disk_row = row_of("64G");
    // Cursor row. Its disk cell carries the highlight, its unset cells draw dim,
    // and its answered cells draw white.
    let cursor_disk = cell_at("/dev/vda1");
    assert_eq!(cursor_disk.fg, HIGHLIGHT, "{cursor_disk:?}");
    assert!(
        cursor_disk.modifier.contains(Modifier::BOLD),
        "{cursor_disk:?}"
    );
    // Branch glyphs are furniture. Only the device name takes the hot end, and
    // the device tree keeps its dim ink.
    let branch = cell_at("\u{2514}\u{2500}");
    assert!(
        branch.modifier.contains(Modifier::DIM),
        "{branch:?} is not dim"
    );
    assert!(cell_at("512M").modifier.contains(Modifier::DIM));
    for answered in ["vfat", copy::EFI_CELL, "/boot/efi"] {
        assert_eq!(cell_at(answered).fg, Color::White, "{answered}");
    }
    // The table row without the cursor takes the highlight nowhere. An answered
    // row's device name draws white wherever the cursor sits.
    assert_ne!(cell_at("/dev/vda").fg, HIGHLIGHT);
    assert_eq!(cell_at("/dev/vda2").fg, Color::White);
    // Table headings sit over their own columns.
    let headings = row_of("filesystem");
    assert_eq!(
        row_text(headings).find("size"),
        row_text(disk_row).find("64G"),
    );
}

/// A partition row ends in its device node. When the label column cannot hold
/// the whole cell, the label gives way and the node stays whole, because the
/// node is what the user acts on.
#[test]
fn a_clipped_row_keeps_the_node_after_its_label() {
    assert_eq!(
        clipped("LONG-PARTITION-DESCRIPTION (/dev/sda1)", 20),
        "LONG-P...(/dev/sda1)"
    );
    // A cell with no node in brackets still cuts at its end.
    assert_eq!(clipped("unbreakable", 4), "u...");
}
