use super::*;

fn rules() -> Vec<Choice> {
    vec![
        Choice::new("1.1.1 tmp", "a separate partition").within("1.1"),
        Choice::new("1.1.2 nodev", "no device files there").within("1.1"),
        Choice::new("1.2.1 gpgcheck", "signatures are checked").within("1.2"),
        Choice::new("RHEL-09-232010", "numbered by nothing"),
    ]
}

fn shape(nodes: &[Node]) -> Vec<(&str, usize, Option<usize>)> {
    nodes
        .iter()
        .map(|node| (node.label.as_str(), node.depth, node.at))
        .collect()
}

fn nest_drawn(options: &[Choice], open: &[bool], on: &[usize], filter: &str, at: usize) -> String {
    nest_drawn_at(60, options, open, on, filter, at)
}

fn nest_drawn_at(
    width: u16,
    options: &[Choice],
    open: &[bool],
    on: &[usize],
    filter: &str,
    at: usize,
) -> String {
    let nodes = nodes(options);
    let rows = shown(&nodes, options, open, filter, false);
    let mut state = ListState::default().with_selected(Some(at));
    let height = u16::try_from(rows.len() + 5).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| {
            nested(
                frame,
                frame.area(),
                "which rules",
                options,
                &Tree {
                    nodes: &nodes,
                    rows: &rows,
                    open,
                    on,
                    filter,
                    legend: NEST,
                },
                &mut state,
            )
        })
        .unwrap();
    terminal.backend().to_string()
}

#[test]
fn a_branch_is_drawn_for_every_dotted_part_and_a_row_with_no_group_stays_at_the_top() {
    assert_eq!(
        shape(&nodes(&rules())),
        vec![
            ("1", 0, None),
            ("1.1", 1, None),
            ("1.1.1 tmp", 2, Some(0)),
            ("1.1.2 nodev", 2, Some(1)),
            ("1.2", 1, None),
            ("1.2.1 gpgcheck", 2, Some(2)),
            ("RHEL-09-232010", 0, Some(3)),
        ]
    );
}

#[test]
fn a_branch_turns_on_everything_under_it_and_off_again() {
    let options = rules();
    let nodes = nodes(&options);
    let mut on = Vec::new();
    check(&mut on, &nodes, &options, 0);
    assert_eq!(on, vec![0, 1, 2]);
    check(&mut on, &nodes, &options, 0);
    assert!(on.is_empty());
}

#[test]
fn a_branch_holding_some_of_what_is_on_is_neither_on_nor_off() {
    let options = rules();
    let nodes = nodes(&options);
    let mut on = Vec::new();
    check(&mut on, &nodes, &options, 2);
    assert_eq!(checkbox(&on, &nodes, &options, 1), "[-] ");
    check(&mut on, &nodes, &options, 3);
    assert_eq!(checkbox(&on, &nodes, &options, 1), "[x] ");
    assert_eq!(checkbox(&on, &nodes, &options, 4), "[ ] ");
}

#[test]
fn a_closed_branch_hides_what_it_contains_and_an_open_one_shows_it() {
    let options = rules();
    let nodes = nodes(&options);
    let mut open = vec![false; nodes.len()];
    assert_eq!(shown(&nodes, &options, &open, "", false), vec![0, 6]);
    open[0] = true;
    assert_eq!(shown(&nodes, &options, &open, "", false), vec![0, 1, 4, 6]);
    open[1] = true;
    assert_eq!(
        shown(&nodes, &options, &open, "", false),
        vec![0, 1, 2, 3, 4, 6]
    );
}

#[test]
fn a_filtered_answer_names_the_option_chosen_and_not_the_row_it_was_on() {
    let options = rules();
    let nodes = nodes(&options);
    let open = vec![false; nodes.len()];
    let rows = shown(&nodes, &options, &open, "signatures", false);
    assert_eq!(rows, vec![5]);
    let mut on = Vec::new();
    check(&mut on, &nodes, &options, rows[0]);
    assert_eq!(on, vec![2]);
    assert_eq!(options[on[0]].label, "1.2.1 gpgcheck");
}

#[test]
fn the_detail_of_the_highlighted_row_is_drawn_under_the_tree() {
    let options = rules();
    let open = vec![true; nodes(&options).len()];
    let drawn = nest_drawn(&options, &open, &[1], "", 3);
    assert!(drawn.contains("> [x]     1.1.2 nodev"), "{drawn}");
    assert!(drawn.contains("[-]   \u{25be} 1.1"), "{drawn}");
    assert!(drawn.contains("no device files there"), "{drawn}");
    assert!(drawn.contains(NEST), "{drawn}");
}

/// Two rules a target cannot take, one of them the whole of its branch.
fn partly_unavailable() -> Vec<Choice> {
    let mut options = rules();
    options[1] = options[1].clone().unavailable();
    options[2] = options[2].clone().unavailable();
    options
}

#[test]
fn an_unavailable_option_is_never_checked_and_a_branch_passes_over_it() {
    let options = partly_unavailable();
    let nodes = nodes(&options);
    let mut on = Vec::new();
    check(&mut on, &nodes, &options, 3);
    assert!(on.is_empty());
    check(&mut on, &nodes, &options, 0);
    assert_eq!(on, vec![0]);
    assert_eq!(checkbox(&on, &nodes, &options, 1), "[x] ");
}

#[test]
fn hiding_drops_each_unavailable_option_and_a_branch_left_empty() {
    let options = partly_unavailable();
    let nodes = nodes(&options);
    let open = vec![true; nodes.len()];
    assert_eq!(shown(&nodes, &options, &open, "", true), vec![0, 1, 2, 6]);
    assert_eq!(shown(&nodes, &options, &open, "", false).len(), 7);
}

/// The narrowest terminal the tool draws for. A key legend cut in half repeats
/// the defect the questions once had.
#[test]
fn every_hint_fits_the_narrowest_terminal() {
    let mut state = ListState::default().with_selected(Some(0));
    for hint in [PICK, TOGGLE, TOGGLE_HIDE, TOGGLE_SHOW, EITHER] {
        let mut terminal = Terminal::new(TestBackend::new(60, 4)).unwrap();
        terminal
            .draw(|frame| {
                draw(
                    frame,
                    frame.area(),
                    "which module",
                    &[Choice::new("gaming", "")],
                    None,
                    hint,
                    &mut state,
                )
            })
            .unwrap();
        let drawn = terminal.backend().to_string();
        assert!(drawn.contains(hint), "{drawn}");
    }
    for legend in [NEST, NEST_HIDE, NEST_SHOW] {
        let options = rules();
        let nodes = nodes(&options);
        let rows = shown(&nodes, &options, &[true; 7], "", false);
        let mut state = ListState::default().with_selected(Some(0));
        let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
        terminal
            .draw(|frame| {
                nested(
                    frame,
                    frame.area(),
                    "which rules",
                    &options,
                    &Tree {
                        nodes: &nodes,
                        rows: &rows,
                        open: &[true; 7],
                        on: &[],
                        filter: "",
                        legend,
                    },
                    &mut state,
                )
            })
            .unwrap();
        let drawn = terminal.backend().to_string();
        assert!(drawn.contains(legend), "{drawn}");
    }
}
