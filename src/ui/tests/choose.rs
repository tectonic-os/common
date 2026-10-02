use super::*;
use ratatui::crossterm::event::KeyCode;

#[test]
fn columns_width_must_be_nonzero_and_fit_the_renderer() {
    assert_eq!(parse_width("0"), None);
    assert_eq!(parse_width("65535"), Some(u16::MAX));
    assert_eq!(parse_width("65536"), None);
}

#[test]
fn every_label_is_drawn_with_its_detail_and_the_selection_marked() {
    let options = [
        Choice::new("gaming", "steam and the rest"),
        Choice::new("starship", "requires shell-config"),
    ];
    let drawn = drawn(&options, None, PICK, 1);
    assert!(drawn.contains("which module"), "{drawn}");
    assert!(drawn.contains("  gaming  steam and the rest"), "{drawn}");
    assert!(
        drawn.contains("> starship  requires shell-config"),
        "{drawn}"
    );
    assert!(drawn.contains(PICK), "{drawn}");
}

#[test]
fn a_toggled_option_is_drawn_held_and_the_rest_are_not() {
    let options = [Choice::new("gaming", ""), Choice::new("starship", "")];
    let drawn = drawn(&options, Some(&[1]), TOGGLE, 0);
    assert!(drawn.contains("> [ ] gaming"), "{drawn}");
    assert!(drawn.contains("  [x] starship"), "{drawn}");
}

#[test]
fn a_question_taking_one_answer_marks_nothing() {
    let options = [Choice::new("Yes", ""), Choice::new("No", "")];
    let drawn = drawn(&options, None, EITHER, 0);
    assert!(drawn.contains("> Yes"), "{drawn}");
    assert!(!drawn.contains('['), "{drawn}");
}

#[test]
fn a_list_longer_than_the_window_says_how_far_down_it_is() {
    let short = ListState::default();
    assert_eq!(hint(PICK, &short, 8, 3), PICK);
    assert_eq!(hint(PICK, &short, 8, 21), format!("{PICK}  8 of 21"));
    let scrolled = ListState::default().with_offset(13);
    assert_eq!(hint(PICK, &scrolled, 8, 21), format!("{PICK}  21 of 21"));
}

#[test]
fn a_child_is_drawn_under_its_parent_and_the_last_one_closes_the_branch() {
    let drawn = drawn(&tree(), Some(&[]), TOGGLE, 0);
    assert!(drawn.contains("> [ ] linux-desktop"), "{drawn}");
    assert!(drawn.contains("  [ ] \u{251c}\u{2500} dx"), "{drawn}");
    assert!(drawn.contains("  [ ] \u{2514}\u{2500} gaming"), "{drawn}");
    assert!(drawn.contains("  [ ] linux-server"), "{drawn}");
}

#[test]
fn a_parent_and_a_child_cannot_both_be_on() {
    let options = tree();
    let mut on = vec![0];
    flip(&mut on, 1, &options);
    assert_eq!(on, vec![1]);
    flip(&mut on, 0, &options);
    assert_eq!(on, vec![0]);
}

#[test]
fn two_children_of_one_parent_can() {
    let options = tree();
    let mut on = Vec::new();
    flip(&mut on, 1, &options);
    flip(&mut on, 2, &options);
    flip(&mut on, 3, &options);
    assert_eq!(on, vec![1, 2, 3]);
}

#[test]
fn flipping_a_held_row_turns_it_off_and_touches_nothing_else() {
    let options = tree();
    let mut on = vec![1, 3];
    flip(&mut on, 1, &options);
    assert_eq!(on, vec![3]);
}

/// The fixture holds a heading row with two commands under it, then a second
/// heading with one command, as a grouped command picker draws them.
fn grouped() -> [Choice; 5] {
    [
        Choice::new("Repository", "").heading(),
        Choice::new("create repo", "start a repository"),
        Choice::new("create image", "add an image"),
        Choice::new("Modules", "").heading(),
        Choice::new("create module", "write a module"),
    ]
}

#[test]
fn a_heading_sits_flush_left_and_its_rows_keep_the_pointer_column() {
    let drawn = drawn(&grouped(), None, PICK, 1);
    assert!(drawn.contains("\"Repository "), "{drawn}");
    assert!(drawn.contains("\"Modules "), "{drawn}");
    assert!(
        drawn.contains("\"> create repo  start a repository"),
        "{drawn}"
    );
    assert!(drawn.contains("\"  create image  add an image"), "{drawn}");
}

#[test]
fn the_cursor_passes_over_a_heading_in_the_direction_it_was_sent() {
    let options = grouped();
    let mut state = ListState::default().with_selected(Some(2));
    move_by(KeyCode::Down, &mut state, options.len());
    skip_spacers(KeyCode::Down, &options, &mut state, Some(2));
    assert_eq!(state.selected(), Some(4));
    move_by(KeyCode::Up, &mut state, options.len());
    skip_spacers(KeyCode::Up, &options, &mut state, Some(4));
    assert_eq!(state.selected(), Some(2));
}

#[test]
fn a_heading_above_the_first_answer_holds_the_cursor_where_it_was() {
    let options = grouped();
    let mut state = ListState::default().with_selected(Some(1));
    move_by(KeyCode::Up, &mut state, options.len());
    skip_spacers(KeyCode::Up, &options, &mut state, Some(1));
    assert_eq!(state.selected(), Some(1));
}

#[test]
fn a_picker_that_opens_on_a_heading_moves_to_the_first_answer() {
    let options = grouped();
    let mut state = ListState::default().with_selected(Some(0));
    skip_spacers(KeyCode::Down, &options, &mut state, Some(0));
    assert_eq!(state.selected(), Some(1));
}

/// A trailing spacer row catches `End` if the cursor index runs past the last
/// row, because the skip then sees no row to pass over.
#[test]
fn home_and_end_land_on_the_first_and_the_last_answer() {
    let options = [
        Choice::new("Repository", "").heading(),
        Choice::new("create repo", ""),
        Choice::new("create image", ""),
        Choice::new("", ""),
    ];
    let mut state = ListState::default().with_selected(Some(1));
    move_by(KeyCode::End, &mut state, options.len());
    skip_spacers(KeyCode::End, &options, &mut state, Some(1));
    assert_eq!(state.selected(), Some(2));
    move_by(KeyCode::Down, &mut state, options.len());
    skip_spacers(KeyCode::Down, &options, &mut state, Some(2));
    assert_eq!(state.selected(), Some(2));
    move_by(KeyCode::Home, &mut state, options.len());
    skip_spacers(KeyCode::Home, &options, &mut state, Some(2));
    assert_eq!(state.selected(), Some(1));
}

/// The installer's completion screen opens on its first action, below the
/// recovery key, on a console too short for every row. Its heading is the row
/// that the cursor never lands on.
#[test]
fn stepping_up_to_the_first_answer_scrolls_the_heading_back_into_view() {
    let mut options = vec![
        Choice::new("Recovery key:", "").heading(),
        Choice::new("the recovery key", ""),
    ];
    options.extend((0..19).map(|line| Choice::new(format!("step {line}"), "")));
    options.push(Choice::new("Reboot", ""));
    options.push(Choice::new("Power off", ""));
    let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
    let mut state = ListState::default().with_selected(Some(options.len() - 2));
    assert!(!framed(&mut terminal, &options, &mut state).contains("Recovery key:"));
    for _ in 0..options.len() {
        let from = state.selected();
        move_by(KeyCode::Up, &mut state, options.len());
        skip_spacers(KeyCode::Up, &options, &mut state, from);
        framed(&mut terminal, &options, &mut state);
    }
    assert_eq!(state.selected(), Some(1));
    assert!(framed(&mut terminal, &options, &mut state).contains("Recovery key:"));
}

/// A grouped command picker on a console too short for every group. Each
/// heading sits above the first answer of its group, where scrolling up stops.
#[test]
fn stepping_up_to_a_group_shows_the_heading_of_that_group() {
    let mut options = Vec::new();
    for group in ["Repository", "Image", "Modules"] {
        options.push(Choice::new(group, "").heading());
        options.extend((0..4).map(|line| Choice::new(format!("command {line}"), "")));
    }
    let mut terminal = Terminal::new(TestBackend::new(60, 7)).unwrap();
    let mut state = ListState::default().with_selected(Some(options.len() - 1));
    framed(&mut terminal, &options, &mut state);
    for _ in 0..options.len() {
        let from = state.selected();
        move_by(KeyCode::Up, &mut state, options.len());
        skip_spacers(KeyCode::Up, &options, &mut state, from);
        let drawn = framed(&mut terminal, &options, &mut state);
        if let Some(at) = state.selected().filter(|at| options[at - 1].heading) {
            assert!(drawn.contains(&options[at - 1].label), "{drawn}");
        }
    }
}

/// The list state carries its scroll offset from one frame to the next, as it
/// does between keys in the picker.
fn framed(
    terminal: &mut Terminal<TestBackend>,
    options: &[Choice],
    state: &mut ListState,
) -> String {
    terminal
        .draw(|frame| draw(frame, frame.area(), "done", options, None, PICK, state))
        .unwrap();
    terminal.backend().to_string()
}
