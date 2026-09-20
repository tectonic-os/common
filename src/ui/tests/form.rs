use super::*;

/// `form` opens on a form field and never on a drawn row number. The two agree
/// only while every field is drawn. A form that hides fields 2 and 3 draws field
/// 4 third, so opening on row 4 there lands on the last row, or past the end of a
/// shorter form. That is how the encryption overlay opened on `confirm PIN`
/// instead of `PIN`.
#[test]
fn the_form_opens_on_the_field_it_was_given_wherever_that_row_is() {
    // Every field drawn, so field index and row number agree. A calling command
    // that passed a row number was right until a form started hiding fields.
    assert_eq!(opens_at(&[0, 1, 2, 3], 2), 2);
    // Encryption overlay, PIN kind. Fields 4 and 5 hold its pair, and field 4 is
    // the third row drawn.
    assert_eq!(opens_at(&[0, 1, 4, 5], 4), 2);
    assert_eq!(opens_at(&[0, 1, 4, 5], 5), 3);
    // A field this form does not draw has no row of its own. The form opens at
    // the top, which the user can see. A clamp to the end cannot be seen.
    assert_eq!(opens_at(&[0, 1, 4, 5], 2), 0);
    assert_eq!(opens_at(&[0], 4), 0);
}

/// A contradicting pair is called wrong only once both halves are finished being
/// typed. The red flags wait for every named form row to have been left.
#[test]
fn a_pair_is_flagged_only_once_both_halves_have_been_left() {
    let fields = [
        Field::secret("password", "hunter2"),
        Field::secret("confirm", "hunter3"),
    ];
    let pair = |fields: &[Field]| match fields[0].value() == fields[1].value() {
        true => Vec::new(),
        false => vec![0, 1],
    };
    assert_eq!(alerted(&fields, &[], &pair), [false, false]);
    assert_eq!(alerted(&fields, &[0], &pair), [false, false]);
    assert_eq!(alerted(&fields, &[0, 1], &pair), [true, true]);
    // A pair that agrees names no form row, however much of it was left.
    let same = [
        Field::secret("password", "x"),
        Field::secret("confirm", "x"),
    ];
    assert_eq!(alerted(&same, &[0, 1], &pair), [false, false]);
}

/// The inline table's cursor walks over the rows the user may rest on, passes
/// the grey preview rows by, and holds at both ends.
#[test]
fn the_table_cursor_skips_what_cannot_be_taken() {
    let selectable = [true, false, true];
    assert_eq!(table_step(&selectable, 0, true), Some(2));
    assert_eq!(table_step(&selectable, 2, false), Some(0));
    // A table end holds the cursor. `None` lets the cursor leave to the form
    // field above or below, which is all a one-row table has.
    assert_eq!(table_step(&selectable, 0, false), None);
    assert_eq!(table_step(&selectable, 2, true), None);
    assert_eq!(table_step(&[true], 0, true), None);
    assert_eq!(table_step(&[true], 0, false), None);
    assert_eq!(table_step(&[], 0, true), None);
}
