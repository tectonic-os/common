use super::*;

/// Draws the installer's screen. One widget box centres the form inside it. Two
/// fields carry answers, two stand unanswered, and an action sits under them.
#[test]
fn a_command_that_owns_the_screen_draws_one_box_around_the_form() {
    let fields = [
        Field::pick(
            "disk",
            vec![Choice::new("/dev/vda", "64G  QEMU HARDDISK")],
            None,
        ),
        Field::text("computer name", "deb2"),
        Field::secret("password", ""),
    ];
    let visible: Vec<usize> = (0..fields.len()).collect();
    let (shown, _, _) = laid_out(
        &fields,
        &visible,
        0,
        0,
        &Mode::Rows,
        &[copy::INSTALL, copy::SHUT_DOWN],
        Some("still needs a disk, a password"),
        true,
        &[],
        &[],
    );
    let mut terminal = Terminal::new(TestBackend::new(64, 16)).unwrap();
    terminal
        .draw(|frame| {
            let area = chrome(
                frame,
                Some("Tectonic installer"),
                "",
                shown.len() as u16,
                copy::INSTALL_KEYS,
            );
            // What the widget is given inside a box. The box carries the legend,
            // so passing the keys here would draw a second one.
            sheet_of(frame, area, &shown, "", 0, 0)
        })
        .unwrap();
    let drawn = terminal.backend().to_string();
    let rows: Vec<&str> = drawn.lines().collect();
    // Sixteen rows stands shorter than the square box, so this terminal's box is
    // capped and fills the screen. The top border takes the first row. On the
    // media's 50-row console the box centres, which the square test covers.
    assert!(rows[0].contains('\u{256d}'), "{drawn}");
    let top = rows
        .iter()
        .position(|row| row.contains("Tectonic installer"))
        .unwrap_or_else(|| panic!("{drawn}"));
    // Rounded corners, which the media's console draws.
    assert!(rows[top].contains('\u{256d}'), "{drawn}");
    // The first field draws with its status beside the label. The box's own
    // padding puts it under the border.
    assert!(drawn.contains("?  disk"), "{drawn}");
    assert!(drawn.contains(copy::INSTALL), "{drawn}");
    assert!(drawn.contains(copy::SHUT_DOWN), "{drawn}");
    assert!(drawn.contains("still needs a disk, a password"), "{drawn}");
    // Exactly one legend, on the bottom border. A widget that drew its own foot
    // inside a box would give two.
    assert_eq!(drawn.matches(copy::INSTALL_KEYS).count(), 1, "{drawn}");
    let last = rows
        .iter()
        .rposition(|row| row.contains('\u{2570}'))
        .unwrap();
    assert!(rows[last].contains(copy::INSTALL_KEYS), "{drawn}");
}

/// A panel stands above the form rows with one blank under it. Section titles
/// draw white and their rows grey, one blank separates two sections, and the form
/// rows keep their order afterwards. The line the widget scrolls to is the
/// cursor's own, with the panel and the headings counted.
#[test]
fn sections_are_drawn_above_the_rows() {
    let fields = [
        Field::text("computer name", "deb2"),
        Field::text("disk", ""),
    ];
    let header = vec![
        HeaderLine::Section("OS Image".to_string()),
        HeaderLine::Row("name: next44".to_string()),
    ];
    let sections = [(0usize, "OS setup"), (1usize, "Disk setup")];
    let (lines, focus, _) = laid_out(
        &fields,
        &[0, 1],
        1,
        0,
        &Mode::Rows,
        &[copy::INSTALL],
        None,
        false,
        &header,
        &sections,
    );
    let text = |line: &Line<'static>| {
        line.spans
            .iter()
            .map(|span| span.content.to_string())
            .collect::<String>()
    };
    assert_eq!(text(&lines[0]), "  OS Image");
    assert_eq!(text(&lines[1]), "     name: next44");
    assert_eq!(text(&lines[2]), "", "one blank under the panel");
    assert_eq!(text(&lines[3]), "  OS setup");
    assert!(
        text(&lines[4]).contains("computer name"),
        "{:?}",
        text(&lines[4])
    );
    assert_eq!(text(&lines[5]), "", "a blank separates the sections");
    assert_eq!(text(&lines[6]), "  Disk setup");
    assert!(text(&lines[7]).contains("disk"), "{:?}", text(&lines[7]));
    assert_eq!(text(&lines[8]), "");
    assert!(text(&lines[9]).contains(copy::INSTALL));
    // The cursor sits on the second field, which is the seventh line.
    assert_eq!(focus, 7);
    // A title draws white and bold. What it says draws grey.
    let title = lines[0].spans[0].style;
    assert_eq!(title.fg, Some(Color::White), "{title:?}");
    assert!(title.add_modifier.contains(Modifier::BOLD), "{title:?}");
    let row = lines[1].spans[0].style;
    assert!(row.add_modifier.contains(Modifier::DIM), "{row:?}");
}

/// The panel's statements hold prose. Each wraps to the widget box's own room,
/// so a long warning reads as sentences and never as one clipped line. The blank
/// row between two statements stays a blank.
#[test]
fn the_required_statements_wrap_and_keep_the_blank_between_them() {
    let statement = copy::required_uki_db();
    let header: Vec<HeaderLine> = statement
        .iter()
        .map(|row| HeaderLine::Row(row.to_string()))
        .collect();
    let fields = [Field::text("computer name", "deb2")];
    let (lines, _, _) = laid_out(
        &fields,
        &[0],
        0,
        0,
        &Mode::Rows,
        &[],
        None,
        false,
        &header,
        &[],
    );
    let text = |line: &Line<'static>| {
        line.spans
            .iter()
            .map(|span| span.content.to_string())
            .collect::<String>()
    };
    let drawn: Vec<String> = lines.iter().map(text).collect();
    let blank = drawn
        .iter()
        .position(|line| line.trim().is_empty())
        .expect("the blank between the statements");
    assert!(blank > 1, "the first statement wrapped: {drawn:?}");
    let joined = |part: &[String]| {
        part.iter()
            .map(|line| line.trim_start())
            .collect::<Vec<_>>()
            .join(" ")
    };
    assert_eq!(joined(&drawn[..blank]), statement[0]);
    let after = blank + 1;
    let end = drawn[after..]
        .iter()
        .position(|line| line.trim().is_empty())
        .expect("the blank under the panel");
    assert_eq!(joined(&drawn[after..after + end]), statement[2]);
    assert!(
        drawn.iter().all(|line| line.chars().count() <= PANEL_ROOM),
        "{drawn:?}"
    );
    // Every row of the statement draws grey, as the panel's rows do.
    let dim = lines[0].spans[0].style;
    assert!(dim.add_modifier.contains(Modifier::DIM), "{dim:?}");
}

/// The kernel-VT fallback gives 25 rows. A panel longer than the installer draws,
/// with every section and more rows than any of them carry, still fits above the
/// fold from the first question. Nothing in it goes unreachable on a console
/// without kmscon.
#[test]
fn the_longest_panel_fits_the_fallback_console() {
    let mut header = vec![HeaderLine::Section("OS Image".to_string())];
    header.extend((0..3).map(|n| HeaderLine::Row(format!("image row {n}"))));
    header.push(HeaderLine::Section("Detected System Firmware".to_string()));
    header.extend((0..2).map(|n| HeaderLine::Row(format!("firmware row {n}"))));
    header.push(HeaderLine::Section("Required".to_string()));
    header.extend((0..8).map(|n| HeaderLine::Row(format!("required row {n}"))));
    let fields = [
        Field::text("computer name", "next44"),
        Field::text("username", ""),
        Field::secret("password", ""),
        Field::secret("confirm", ""),
        Field::pick("disk", vec![Choice::new("/dev/vda", "")], None),
    ];
    let visible = [0, 1, 2, 3, 4];
    let (lines, focus, _) = laid_out(
        &fields,
        &visible,
        0,
        0,
        &Mode::Rows,
        &["Install"],
        None,
        false,
        &header,
        &[(0, "OS setup")],
    );
    let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
    terminal
        .draw(|frame| {
            let area = chrome(
                frame,
                Some("Tectonic installer"),
                "",
                lines.len() as u16,
                "",
            );
            sheet_of(frame, area, &lines, "", focus, 0);
        })
        .unwrap();
    let drawn = terminal.backend().to_string();
    assert!(drawn.contains("OS Image"), "{drawn}");
    assert!(drawn.contains("required row 7"), "{drawn}");
    assert!(drawn.contains("computer name"), "{drawn}");
}

/// The action under the cursor carries the palette's highlight colour. A blocked
/// action stays dim under the cursor, because the user must not take what cannot
/// be taken.
#[test]
fn the_chosen_action_is_the_highlight_colour() {
    let fields = [Field::text("disk", "")];
    let (chosen, _, _) = laid_out(
        &fields,
        &[0],
        1,
        0,
        &Mode::Rows,
        &["Install", "Shut down"],
        None,
        false,
        &[],
        &[],
    );
    let span = |line: &Line<'static>| {
        line.spans
            .iter()
            .find(|span| span.content.contains("Install"))
            .unwrap()
            .style
    };
    let style = span(chosen.last().expect("the actions row"));
    assert_eq!(style.fg, Some(HIGHLIGHT), "{style:?}");
    assert!(style.add_modifier.contains(Modifier::REVERSED), "{style:?}");

    let (blocked, _, _) = laid_out(
        &fields,
        &[0],
        1,
        0,
        &Mode::Rows,
        &["Install"],
        Some("still needs a disk"),
        true,
        &[],
        &[],
    );
    // The reason draws under a blank row, so the buttons are the line holding
    // `Install` and never the line before the reason.
    let buttons = blocked
        .iter()
        .find(|line| {
            line.spans
                .iter()
                .any(|span| span.content.contains("Install"))
        })
        .expect("the actions row");
    let style = span(buttons);
    assert!(style.add_modifier.contains(Modifier::DIM), "{style:?}");
    // Trying the action asks for the reason, and a blank line sets it apart.
    assert_eq!(
        blocked.iter().rev().nth(1).map(|line| line.width()),
        Some(0),
        "{blocked:?}"
    );
    assert!(
        blocked.last().is_some_and(|line| line
            .spans
            .iter()
            .any(|span| span.content.contains("still needs a disk"))),
        "{blocked:?}"
    );
}

/// Trying the action asks for the blocked reason. Before that the screen says
/// nothing and the buttons hold the last line.
#[test]
fn the_blocked_reason_is_drawn_only_once_the_action_is_tried() {
    let fields = [Field::text("computer name", "")];
    let quiet = laid_out(
        &fields,
        &[0],
        1,
        0,
        &Mode::Rows,
        &["Install"],
        Some("still needs a computer name"),
        false,
        &[],
        &[],
    )
    .0;
    assert!(
        !quiet.iter().any(|line| line
            .spans
            .iter()
            .any(|span| span.content.contains("still needs"))),
        "{quiet:?}"
    );
    assert!(
        quiet.last().is_some_and(|line| line
            .spans
            .iter()
            .any(|span| span.content.contains("Install"))),
        "{quiet:?}"
    );
}

/// A window with no actions has no button to try. The enter on its last field is
/// the decision, and a refusal draws its reason one blank under the fields,
/// wrapped to the room the window has, with the box's spare rows below it.
#[test]
fn an_actionless_window_draws_the_reason_it_refused_with() {
    const WHY: &str = "the two answers do not agree";
    let fields = [
        Field::secret("passphrase", "a"),
        Field::secret("confirm", "b"),
    ];
    let text = |lines: &[Line<'static>]| -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect()
    };
    let quiet = laid_out(
        &fields,
        &[0, 1],
        0,
        0,
        &Mode::Rows,
        &[],
        Some(WHY),
        false,
        &[],
        &[],
    );
    assert!(
        !text(&quiet.0).iter().any(|line| line.contains(WHY)),
        "{:?}",
        quiet.0
    );
    assert_eq!(
        quiet.2,
        quiet.0.len(),
        "a form without actions pads under its reason"
    );
    // `form` clamps the cursor to the last field where a form has no actions, so
    // the reason draws in that state.
    let said = laid_out(
        &fields,
        &[0, 1],
        1,
        0,
        &Mode::Rows,
        &[],
        Some(WHY),
        true,
        &[],
        &[],
    );
    let drawn = text(&said.0);
    assert_eq!(
        drawn.len(),
        4,
        "two fields, the blank, and the reason: {:?}",
        drawn
    );
    assert!(drawn[2].is_empty(), "one blank under the field: {drawn:?}");
    assert!(drawn[3].contains(WHY), "{drawn:?}");
    assert_eq!(said.1, 3, "the reason is what stays in view");
    assert_eq!(
        said.2,
        said.0.len(),
        "the padding goes below the reason, not between it and the field"
    );
    // The indent draws. The reason sits under the property names and never under
    // the cursor marker.
    assert!(drawn[3].starts_with("     "), "{:?}", drawn[3]);
}

/// A measure holds a number and the screen supplies its unit. It takes digits
/// only, and the row reads with one decimal place for every typed digit.
#[test]
fn a_measure_takes_only_digits_and_reads_with_one_decimal() {
    let mut fields = [Field::measure("size", "20", "GB")];
    fields[0].push('x');
    fields[0].push('.');
    fields[0].push('5');
    assert_eq!(fields[0].value(), "205", "a letter is not a size");
    fields[0].pop();
    assert_eq!(fields[0].shown(), "20.0");
    let (lines, _, _) = laid_out(&fields, &[0], 0, 0, &Mode::Rows, &[], None, false, &[], &[]);
    let text: String = lines[0]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect();
    assert!(text.contains("20.0 GB"), "{text}");
    assert!(
        lines[0].spans[4].style.add_modifier.contains(Modifier::DIM),
        "the unit is drawn apart from the value: {:?}",
        lines[0].spans[4].style
    );
}

/// A radio group draws whether or not it holds the keys. Its dot marks the
/// answer it holds, and its own row's marker goes while the group carries the
/// cursor.
#[test]
fn a_radio_group_is_drawn_and_its_dot_is_the_answer() {
    let fields = [
        Field::radio(
            "type",
            vec![Choice::new("none", ""), Choice::new("passphrase", "")],
            Some(1),
        ),
        Field::text("name", ""),
    ];
    let text = |lines: &[Line<'static>]| -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect()
    };
    // The cursor sits on the row below. The group still draws, its answer keeps
    // the dot, and no option carries the cursor.
    let away = laid_out(
        &fields,
        &[0, 1],
        1,
        0,
        &Mode::Rows,
        &[],
        None,
        false,
        &[],
        &[],
    );
    let away = text(&away.0);
    let answer = away
        .iter()
        .find(|line| line.contains("\u{25c9} passphrase"))
        .expect("the dotted answer");
    assert!(!answer.contains("> "), "{answer}");
    assert!(away.iter().any(|line| line.contains("\u{25cb} none")));
    // The group holds the keys. The row's own marker goes, the dot stays on the
    // answer, and the cursor marks the option it sits on.
    let drawn = laid_out(
        &fields,
        &[0, 1],
        0,
        0,
        &Mode::Open(0),
        &[],
        None,
        false,
        &[],
        &[],
    );
    // The dots carry the answer, so the row states no value of its own.
    let on_it = text(&drawn.0);
    assert!(!on_it[0].contains("passphrase"), "{}", on_it[0]);
    assert!(!on_it[0].contains("none"), "{}", on_it[0]);
    // The label draws plain while the list is open. The option under the cursor
    // is the only row drawn hot.
    let label = drawn.0[0]
        .spans
        .iter()
        .find(|span| span.content.contains("type"))
        .expect("the row's label");
    assert_ne!(label.style.fg, Some(HIGHLIGHT), "{:?}", label.style);
    assert!(!on_it[0].contains("> "), "{}", on_it[0]);
    let answer = on_it
        .iter()
        .find(|line| line.contains("\u{25c9} passphrase"))
        .expect("the dotted answer");
    assert!(!answer.contains("> "), "{answer}");
    let cursor = on_it
        .iter()
        .find(|line| line.contains("\u{25cb} none"))
        .expect("the cursor's option");
    assert!(cursor.contains("> "), "{cursor}");
    // A list open on one row touches no other row's label. A fixed row two above
    // stays as dim as it was, and a list belonging to a different question leaves
    // it dim.
    let elsewhere = [
        Field::fixed("shown", "value"),
        Field::radio(
            "type",
            vec![Choice::new("none", ""), Choice::new("passphrase", "")],
            Some(1),
        ),
    ];
    let open = laid_out(
        &elsewhere,
        &[0, 1],
        1,
        0,
        &Mode::Open(0),
        &[],
        None,
        false,
        &[],
        &[],
    );
    let dim = open.0[0]
        .spans
        .iter()
        .find(|span| span.content.contains("shown"))
        .expect("the fixed row's label");
    assert!(dim.style.add_modifier.contains(Modifier::DIM), "{dim:?}");
}

/// A refused menu option draws nowhere and never becomes a dim cursor stop. Its
/// original position stays the held-answer position. An all-refused radio says
/// why the menu has no rows.
#[test]
fn a_hidden_option_is_not_drawn_or_landed_on() {
    let options = vec![
        Choice::new("no TPM", "No TPM available")
            .unavailable()
            .hidden(),
        Choice::new("passphrase", ""),
        Choice::new("no initramfs", "No LUKS support")
            .unavailable()
            .hidden(),
    ];
    let fields = [Field::radio("encryption", options, Some(0))];
    assert_eq!(
        fields[0].value(),
        "no TPM",
        "the held answer keeps its index"
    );
    assert!(matches!(opened(&fields[0]), Mode::Open(1)));
    let (lines, _, _) = laid_out(
        &fields,
        &[0],
        0,
        0,
        &Mode::Open(1),
        &[],
        None,
        false,
        &[],
        &[],
    );
    let drawn: String = lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref())
        .collect();
    assert!(drawn.contains("passphrase"), "{drawn}");
    assert!(!drawn.contains("no TPM"), "{drawn}");
    assert!(!drawn.contains("no initramfs"), "{drawn}");
    assert_eq!(
        option_step(
            match &fields[0] {
                Field::Pick { options, .. } => options,
                _ => unreachable!(),
            },
            1,
            false
        ),
        None
    );
    assert_eq!(
        option_step(
            match &fields[0] {
                Field::Pick { options, .. } => options,
                _ => unreachable!(),
            },
            1,
            true
        ),
        None
    );

    let none = [Field::radio(
        "encryption",
        vec![Choice::new("no TPM", "No TPM available")
            .unavailable()
            .hidden()],
        Some(0),
    )];
    let (lines, _, _) = laid_out(&none, &[0], 0, 0, &Mode::Rows, &[], None, false, &[], &[]);
    assert!(lines.iter().any(|line| line
        .spans
        .iter()
        .any(|span| span.content.contains(NO_CHOICES))));
}

/// A flagged pair draws red on its values, under the labels' own ink. The red
/// says the user's two halves disagree.
#[test]
fn a_flagged_pair_is_drawn_red_on_its_values() {
    let fields = [
        Field::Secret {
            label: "password".to_string(),
            value: "hunter2".to_string(),
            alert: true,
        },
        Field::Secret {
            label: "confirm".to_string(),
            value: "hunter3".to_string(),
            alert: true,
        },
        Field::text("computer name", "deb2"),
    ];
    let (lines, _, _) = laid_out(
        &fields,
        &[0, 1, 2],
        2,
        0,
        &Mode::Rows,
        &[],
        None,
        false,
        &[],
        &[],
    );
    for at in 0..2 {
        let value = lines[at].spans.last().expect("a value span");
        assert_eq!(value.style.fg, Some(ALERT), "{:?}", lines[at]);
        let label = lines[at]
            .spans
            .iter()
            .find(|span| span.content.contains("password") || span.content.contains("confirm"))
            .expect("the row's label");
        assert_ne!(label.style.fg, Some(ALERT), "{:?}", label.style);
    }
    let plain = lines[2].spans.last().expect("a value span");
    assert_ne!(plain.style.fg, Some(ALERT), "a row nobody flagged");
}

/// An option whose detail cannot fit beside its label draws that detail on its
/// own wrapped line. A clipped reason is no reason.
#[test]
fn a_long_option_detail_wraps_to_its_own_line() {
    let reason = copy::OPENED_KEYFILE_PLAIN;
    let fields = [Field::pick(
        copy::ROW_ENCRYPTION,
        vec![Choice::new(copy::OPENED_ADD_KEY, reason).unavailable()],
        Some(0),
    )];
    let (lines, _, _) = laid_out(
        &fields,
        &[0],
        0,
        0,
        &Mode::Open(0),
        &[],
        None,
        false,
        &[],
        &[],
    );
    let text: Vec<String> = lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect();
    assert!(
        text.iter().any(|line| line.contains(reason)),
        "the reason is not on a line of its own: {text:?}"
    );
}

/// Answering a field lands on the next one highlighted. Enter opens a pick, so
/// nothing opens on the way. A gap is no stop. Typing opens a text field.
#[test]
fn a_step_lands_on_the_next_question_without_opening_it() {
    let fields = [
        Field::pick(
            "layout",
            vec![Choice::new("whole", ""), Choice::new("manual", "")],
            Some(0),
        ),
        Field::gap(),
        Field::text("name", ""),
    ];
    let visible = [0, 1, 2];
    let mut cursor = 0;
    assert!(matches!(onward(&fields, &visible, &mut cursor), Mode::Rows));
    assert_eq!(cursor, 2, "the gap is not a stop");
    let mut cursor = 2;
    assert!(matches!(
        backward(&fields, &visible, &mut cursor),
        Mode::Rows
    ));
    assert_eq!(cursor, 0);
    let (lines, _, _) = laid_out(
        &fields,
        &visible,
        0,
        0,
        &Mode::Rows,
        &[],
        None,
        false,
        &[],
        &[],
    );
    let text = |line: &Line<'static>| {
        line.spans
            .iter()
            .map(|span| span.content.to_string())
            .collect::<String>()
    };
    assert_eq!(text(&lines[1]), "", "{:?}", lines);
    assert!(text(&lines[2]).contains("name"), "{:?}", lines);
}

/// A table field draws its headings over its rows. While the table holds the
/// keys, the row its own cursor sits on draws hot in its first cell, and a
/// preview row draws dim. Resting on `disk selection` opens nothing, so no cell
/// draws hot until enter.
#[test]
fn a_table_field_draws_its_headings_its_rows_and_the_hot_one() {
    let rows = vec![
        vec![
            Cell::set("/dev/vda"),
            Cell::new("68 GB"),
            Cell::new(""),
            Cell::new(""),
            Cell::new(""),
            Cell::new(""),
        ],
        vec![
            Cell::new("\u{2514}\u{2500} root"),
            Cell::new("the rest"),
            Cell::new("ext4"),
            Cell::new(""),
            Cell::new(""),
            Cell::new("/"),
        ],
    ];
    let fields = [
        Field::text("computer name", "deb2"),
        Field::table(
            "disk",
            "disk",
            &["size", "filesystem", "format", "type", "mount"],
            &[],
            rows.clone(),
            vec![true, false],
            vec![Vec::new(), Vec::new()],
            0,
            false,
            false,
        ),
    ];
    // The cursor rests on the row and has entered nothing. The table's own cursor
    // stays cold.
    let resting = laid_out(
        &fields,
        &[0, 1],
        1,
        0,
        &Mode::Rows,
        &[copy::INSTALL],
        None,
        false,
        &[],
        &[],
    )
    .0;
    let cold = resting[3]
        .spans
        .iter()
        .find(|span| span.content.contains("/dev/vda"))
        .expect("the disk row");
    assert_ne!(cold.style.fg, Some(HIGHLIGHT), "{:?}", cold.style);
    // The cursor sits on the field above, so the table's label draws cold. It
    // carried the accent wherever the cursor was.
    let away = laid_out(
        &fields,
        &[0, 1],
        0,
        0,
        &Mode::Rows,
        &[copy::INSTALL],
        None,
        false,
        &[],
        &[],
    )
    .0;
    let cold_label = away[1]
        .spans
        .iter()
        .find(|span| span.content.contains("disk"))
        .expect("the table's label");
    assert_ne!(
        cold_label.style.fg,
        Some(HIGHLIGHT),
        "{:?}",
        cold_label.style
    );
    let hot_label = away[0]
        .spans
        .iter()
        .find(|span| span.content.contains("computer name"))
        .expect("the field above's label");
    assert_eq!(hot_label.style.fg, Some(HIGHLIGHT), "{:?}", hot_label.style);
    let (lines, focus, _) = laid_out(
        &fields,
        &[0, 1],
        1,
        0,
        &Mode::Table,
        &[copy::INSTALL],
        None,
        false,
        &[],
        &[],
    );
    let text = |line: &Line<'static>| {
        line.spans
            .iter()
            .map(|span| span.content.to_string())
            .collect::<String>()
    };
    // Field 0 takes one line. Then come the table's label, its headings, and its
    // two rows.
    assert!(text(&lines[1]).contains("disk"), "{:?}", text(&lines[1]));
    assert!(text(&lines[2]).contains("size"), "{:?}", text(&lines[2]));
    assert!(
        text(&lines[3]).contains("/dev/vda"),
        "{:?}",
        text(&lines[3])
    );
    assert!(text(&lines[4]).contains("root"), "{:?}", text(&lines[4]));
    // The cursor landed on the row the table holds.
    assert_eq!(focus, 3);
    let preview = lines[4]
        .spans
        .iter()
        .find(|span| span.content.contains("root"))
        .expect("the preview row");
    assert!(
        preview.style.add_modifier.contains(Modifier::DIM),
        "{:?}",
        preview.style
    );
    let hot = lines[3]
        .spans
        .iter()
        .find(|span| span.content.contains("/dev/vda"))
        .expect("the hot row");
    assert_eq!(hot.style.fg, Some(HIGHLIGHT), "{:?}", hot.style);
}

/// The table's first column and the row prefix come off the widget's room before
/// the value columns are budgeted, so a table with a wide first column still
/// draws its last columns at the narrowest width.
#[test]
fn a_table_with_long_node_names_still_draws_its_last_columns() {
    let names = [
        "\u{25c9} QEMU HARDDISK (/dev/vda)",
        "\u{251c}\u{2500} esp (/boot/efi)",
        "\u{2514}\u{2500} home (/var/home)",
    ];
    let rows: Vec<Vec<Cell>> = names
        .iter()
        .map(|name| {
            vec![
                Cell::new(name),
                Cell::new("68.7 GB"),
                Cell::new("ext4"),
                Cell::new(""),
                Cell::new(""),
                Cell::new("/var/home"),
            ]
        })
        .collect();
    let fields = [Field::table(
        "disk selection",
        "disk",
        &copy::layout_headings(),
        &[],
        rows,
        vec![true; names.len()],
        vec![Vec::new(); names.len()],
        0,
        false,
        true,
    )];
    let (lines, _, _) = laid_out(&fields, &[0], 0, 0, &Mode::Rows, &[], None, false, &[], &[]);
    let text = |line: &Line<'static>| -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    };
    // The headings row carries the label column and then the value columns. The
    // last of them draws and is never clipped away.
    let head = text(&lines[1]);
    assert!(head.contains("type"), "{head:?}");
    assert!(head.contains("mount"), "{head:?}");
    // The whole table stays inside the widget's room on the narrowest console the
    // tool draws for.
    assert!(head.chars().count() <= PANEL_ROOM, "{head:?}");
    for line in &lines[2..] {
        let row = text(line);
        assert!(row.chars().count() <= PANEL_ROOM, "{row:?}");
    }
}

/// The table's own cursor carries the marker once the keys are inside it, so
/// the label above stops reading as the selection.
#[test]
fn a_focused_table_moves_the_marker_onto_its_row() {
    let rows = vec![
        vec![Cell::set("\u{25c9} /dev/vda"), Cell::new("68.7 GB")],
        vec![
            Cell::new("\u{2514}\u{2500} root (/dev/vda1)"),
            Cell::new("68.7 GB"),
        ],
    ];
    let fields = [Field::table(
        "disk selection",
        "disk",
        &["size"],
        &[],
        rows,
        vec![true, true],
        vec![Vec::new(); 2],
        1,
        false,
        false,
    )];
    let text = |line: &Line<'static>| -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    };
    let (resting, _, _) = laid_out(&fields, &[0], 0, 0, &Mode::Rows, &[], None, false, &[], &[]);
    assert!(
        text(&resting[0]).starts_with("> "),
        "{:?}",
        text(&resting[0])
    );
    let (focused, _, _) = laid_out(
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
    // The label gives the marker up, and the row the table's cursor sits on
    // takes it.
    assert!(
        !text(&focused[0]).starts_with('>'),
        "{:?}",
        text(&focused[0])
    );
    assert!(
        text(&focused[3]).starts_with("   > "),
        "{:?}",
        text(&focused[3])
    );
}

/// Draws the installer's screen as a form. Every question stands on it at once,
/// the disk list opens in place under its own row, and the first of the two
/// actions draws dim because it cannot be taken yet.
#[test]
fn a_form_shows_every_field_and_opens_a_list_where_it_stands() {
    let disks = vec![
        Choice::new("/dev/sda", "512G  Samsung SSD"),
        Choice::new("/dev/sdb", "1T  WD  removable"),
    ];
    let fields = [
        Field::pick("disk", disks, None),
        Field::text("computer name", "debian-bootc"),
        Field::text("username", ""),
        Field::secret("password", "hunter2"),
        Field::secret("confirm", "hunter2"),
        Field::pick(
            "encryption",
            vec![Choice::new("none", "not encrypted")],
            Some(0),
        ),
    ];
    let visible: Vec<usize> = (0..fields.len()).collect();
    let (shown, _, _) = laid_out(
        &fields,
        &visible,
        0,
        0,
        &Mode::Open(1),
        &["Install", "Shut down"],
        Some("still needs a disk, a username"),
        false,
        &[],
        &[],
    );
    let mut terminal = Terminal::new(TestBackend::new(64, shown.len() as u16 + 1)).unwrap();
    terminal
        .draw(|frame| sheet_of(frame, frame.area(), &shown, copy::INSTALL_KEYS, 0, 0))
        .unwrap();
    let drawn = terminal.backend().to_string();
    eprintln!("SHOT\n{drawn}SHOT");
    // The list opens under its own row and takes no screen of its own. The row's
    // own marker goes while the list carries the cursor.
    let rows: Vec<&str> = drawn.lines().collect();
    assert!(rows[0].contains("?  disk"), "{drawn}");
    assert!(!rows[0].contains("> ?  disk"), "{drawn}");
    assert!(rows[1].contains("/dev/sda"), "{drawn}");
    assert!(rows[2].contains("> "), "{drawn}");
    assert!(rows[3].contains("computer name"), "{drawn}");
    // A secret reads as its length and never as itself.
    assert!(
        drawn.contains("*******") && !drawn.contains("hunter2"),
        "{drawn}"
    );
    // An unanswered field carries the amber question. The blocked action draws
    // dim and says nothing until the user tries it.
    assert!(drawn.contains('?'), "{drawn}");
    assert!(
        drawn.contains("Install") && drawn.contains("Shut down"),
        "{drawn}"
    );
    assert!(!drawn.contains("still needs a disk, a username"), "{drawn}");
}
