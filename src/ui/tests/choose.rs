use super::*;

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
