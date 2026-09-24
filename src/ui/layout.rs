//! Turns form fields into the lines drawn, and flags the form rows that
//! contradict each other.

use crate::ui::cells::*;
use crate::ui::chrome::*;
use crate::ui::form::*;
use crate::ui::partition_bar::{chosen, placed, said};
use crate::ui::progress::blend;
use crate::ui::table;
use crate::ui::NO_CHOICES;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// Counts the steps the typing caret walks through the accent gradient. The
/// blink is the step changing, and the modulo brings it back to the start.
const CARET_STEPS: usize = 8;

/// Reports which form rows named by `invalid` are finished being answered. A row
/// counts as finished once it has been left. If one half of a pair is still
/// being typed, neither half is called wrong. A row named on its own is never
/// flagged.
pub(crate) fn alerted(
    fields: &[Field],
    left: &[usize],
    invalid: impl Fn(&[Field]) -> Vec<usize>,
) -> Vec<bool> {
    let mut flags = vec![false; fields.len()];
    let named = invalid(fields);
    if !named.is_empty() && named.iter().all(|row| left.contains(row)) {
        for row in named {
            if let Some(flag) = flags.get_mut(row) {
                *flag = true;
            }
        }
    }
    flags
}

/// Lays out every form field, the options under whichever field is open, a
/// blank, the action buttons, and the reason the first button is stopped. `open`
/// is the option the cursor is on inside `fields[cursor]`. No other field can be
/// open, because a form shows one option list at a time.
#[allow(clippy::too_many_arguments)]
pub(crate) fn laid_out(
    fields: &[Field],
    visible: &[usize],
    cursor: usize,
    button: usize,
    mode: &Mode,
    actions: &[&str],
    blocked: Option<&str>,
    tried: bool,
    header: &[HeaderLine],
    sections: &[(usize, &str)],
) -> (Vec<Line<'static>>, usize, usize) {
    // `mode` states what the form is doing. It names which option a pick has
    // open, whether a field is being typed into, and whether the inline table's
    // cursor is hot. Three booleans passed in would be three chances to disagree
    // with the keys.
    let typing = matches!(mode, Mode::Typing);
    let table_focused = matches!(mode, Mode::Table | Mode::TableMenu { .. });
    let open = match mode {
        Mode::Open(at) => Some(*at),
        _ => None,
    };
    // The label column takes the width of the widest field label or table row
    // name. An inline table draws its first cell there, so its columns line up
    // with the form's own.
    let mut width = visible
        .iter()
        .map(|row| fields[*row].label().chars().count())
        .max()
        .unwrap_or(0);
    let labels = width;
    for row in visible {
        if let Field::Table { rows, .. } = &fields[*row] {
            for cells in rows {
                width = width.max(cells.first().map_or(0, |cell| cell.text.chars().count()));
            }
        }
    }
    // A table given its own column widths keeps the value columns still, and the
    // label column takes the room the widget has left. Long row names then give
    // way to the columns rather than moving them. The form's own labels never
    // give way, because a clipped question is a question the user cannot read.
    for row in visible {
        if let Field::Table { widths, .. } = &fields[*row] {
            if !widths.is_empty() {
                let fixed: usize =
                    widths.iter().sum::<usize>() + 2 * widths.len().saturating_sub(1);
                width = form_room().saturating_sub(7 + fixed).max(1).max(labels);
            }
        }
    }
    let mut lines: Vec<Line<'static>> = Vec::new();
    for (at, line) in header.iter().enumerate() {
        match line {
            // A panel section opens with a blank line, which sets it apart from
            // the section above it. The calling command spaces no rows itself.
            HeaderLine::Section(title) => {
                if at > 0 {
                    lines.push(Line::default());
                }
                // The title indents with the panel rows under it, so it reads as
                // the head of its own group.
                lines.push(Line::from(Span::styled(
                    format!("  {title}"),
                    Style::new().fg(Color::White).bold(),
                )));
            }
            // A panel row holds prose and wraps where the widget box runs out of
            // columns, so a long statement reads as a sentence and is never
            // clipped. One statement fills one panel row, and the blank between
            // two statements wraps to a blank line.
            HeaderLine::Row(text) => {
                // The row indents with the panel pairs beside it and the form
                // fields below, so the panel body reads down one edge at every
                // row shape. A panel inside an overlay wraps where that
                // overlay ends.
                for line in table::wrap(text, form_room().saturating_sub(5)) {
                    lines.push(Line::from(Span::styled(
                        format!("     {line}"),
                        Style::new().dim(),
                    )))
                }
            }
            // A panel pair holds no prose. Its answer reads in the form's own
            // value column, so the panel and the fields under it line up.
            HeaderLine::Pair(label, value) => {
                lines.push(Line::from(vec![
                    Span::styled(format!("     {label:<width$}  "), Style::new().dim()),
                    Span::styled(value.clone(), Style::new()),
                ]));
            }
            // The same panel pair. The answer takes the colour its firmware state
            // reads in.
            HeaderLine::Good(label, value) | HeaderLine::Warn(label, value) => {
                let colour = match line {
                    HeaderLine::Good(..) => GOOD,
                    _ => AMBER,
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("     {label:<width$}  "), Style::new().dim()),
                    Span::styled(value.clone(), Style::new().fg(colour)),
                ]));
            }
            // A measure field holds digits only. An empty field reads as zero,
            // which draws no partition until the user types a size. A number
            // too long for a `u64` reads as the largest one, so the bar draws
            // it red where the window refuses it.
            HeaderLine::Placement {
                holes,
                offset,
                size,
                available,
            } => {
                let gb = |at: &usize| match fields.get(*at).map(Field::value) {
                    Some(typed) if !typed.is_empty() => typed.parse().unwrap_or(u64::MAX),
                    _ => 0,
                };
                // The bar spans the whole room between the paddings of the
                // window, so it stands centred and ignores the form columns.
                lines.push(Line::from(placed(holes, gb(offset), gb(size), form_room())));
                if let Some((hole, _)) = chosen(holes, gb(offset), gb(size)) {
                    lines.push(Line::default());
                    lines.push(Line::from(vec![
                        Span::styled(format!("     {available:<width$}  "), Style::new().dim()),
                        Span::raw(said(hole.gb)),
                    ]));
                }
                // A panel line under the bar belongs to the form rows, so a
                // blank sets the bar apart from it.
                if at + 1 < header.len() {
                    lines.push(Line::default());
                }
            }
        }
    }
    if !header.is_empty() {
        lines.push(Line::default());
    }
    // Records the line the widget keeps in view. A section heading is a line like
    // any other, so it is counted here and never by the calling command.
    let mut focus = 0usize;
    let mut rows_began = false;
    for (at, row) in visible.iter().enumerate() {
        // A section heading stands over the first field of its section, with a
        // blank line above it. No field takes a row that would then be counted.
        // The first heading follows the panel's own blank and adds none.
        if let Some((_, title)) = sections.iter().find(|(section, _)| section == row) {
            if rows_began {
                lines.push(Line::default());
            }
            lines.push(Line::from(Span::styled(
                format!("  {title}"),
                Style::new().fg(Color::White).bold(),
            )));
        }
        let field = &fields[*row];
        // A gap draws a blank line and holds no form row. It separates two groups
        // of questions and answers nothing. The cursor never lands on it.
        if matches!(field, Field::Gap) {
            lines.push(Line::default());
            rows_began = true;
            continue;
        }
        let here = at == cursor;
        let (done, glyph) = field.status();
        let status = match done {
            true => Style::new().fg(GOOD),
            false => Style::new().fg(AMBER),
        };
        // An overlay draws no status mark beside its rows, as the installer's
        // overlays have none. The two marker columns stay, so labels and values
        // still line up. The main form keeps its green and amber marks.
        let glyph = match OVERLAY.with(std::cell::Cell::get) {
            Some(_) => " ",
            None => glyph,
        };
        // The cursor row takes the accent over the whole row, because a form is
        // read down its field labels. While this row's own option list is open,
        // the label goes plain, because the open option is then the only
        // row drawn hot. A list open under another row changes no label but its own.
        let label_style = match here {
            true if open.is_some() || table_focused => Style::new(),
            true => Style::new().fg(HIGHLIGHT).bold(),
            // A field the user cannot answer draws dim, so the form says which of its
            // rows are questions without being told.
            false if field.answerable() => Style::new(),
            false => Style::new().dim(),
        };
        // The inline table is a field like the others. Its label stands in the
        // label column with its status, and its headings over the value columns.
        if let Field::Table {
            label,
            column,
            headings,
            widths: fixed,
            rows,
            cursor: table_cursor,
            ..
        } = field
        {
            let heads: Vec<&str> = headings.iter().map(String::as_str).collect();
            // The row prefix and the label column draw before the value columns,
            // so the value columns take what is left of the widget's room. A
            // table that names its own widths keeps them, because the row names
            // then give way instead of the columns moving.
            let widths = match fixed.is_empty() {
                true => column_widths(
                    &heads,
                    &value_cells(rows),
                    form_room().saturating_sub(7 + width),
                ),
                false => fixed.clone(),
            };
            let field_at = lines.len();
            lines.push(Line::from(vec![
                Span::styled(
                    match (here, table_focused) {
                        // The table's own cursor carries the marker once the keys
                        // are inside it, the same way an open option list takes
                        // the marker from its row.
                        (true, false) => "> ",
                        _ => "  ",
                    },
                    label_style,
                ),
                Span::styled(format!("{glyph}  "), status),
                Span::styled(format!("{label:<width$}"), label_style),
            ]));
            // Table columns head under the field label. The first column's own
            // heading stands over the row names, and the later headings over their own
            // values.
            lines.push(Line::from(vec![
                Span::raw("     "),
                Span::styled(format!("{column:<width$}  "), Style::new().dim()),
                Span::styled(table_headings(&heads, &widths), Style::new().dim()),
            ]));
            let first = lines.len();
            for (n, cells) in rows.iter().enumerate() {
                // Two columns for the panel section body and three for the status
                // column, the same prefix the label and heading rows carry. The
                // row the table's cursor is on takes the marker when the table
                // holds the keys, so the pointer reads on the row itself.
                let hot = table_focused && here && n == *table_cursor;
                let mut spans = vec![match hot {
                    true => Span::styled("   > ", Style::new().fg(HIGHLIGHT).bold()),
                    false => Span::raw("     "),
                }];
                spans.extend(table_row_spans(cells, width, &widths, hot));
                lines.push(Line::from(spans));
            }
            if here {
                focus = match rows.is_empty() {
                    true => field_at,
                    false => first + (*table_cursor).min(rows.len() - 1),
                };
            }
            rows_began = true;
            continue;
        }
        let field_at = lines.len();
        // The caret draws as a quarter block after the text. This widget opens no
        // terminal cursor of its own, and a field being typed into must read
        // differently from one the user is not typing into. Its colour walks the
        // accent gradient, so the blink reads on a console drawing no cursor.
        let editing = here && typing;
        let value = match editing {
            true => field.typing(),
            false => field.shown(),
        };
        let mut row = vec![
            Span::styled(
                match (here, open.is_some()) {
                    // An open option list under the row carries the cursor, so
                    // the row's own marker goes with it.
                    (true, false) => "> ",
                    _ => "  ",
                },
                label_style,
            ),
            Span::styled(format!("{glyph}  "), status),
            Span::styled(format!("{:<width$}  ", field.label()), label_style),
        ];
        // A radio group's dots carry the answer. The field row states no value of
        // its own, because the chosen label repeated over the dots reads as two
        // answers.
        if !matches!(field, Field::Pick { always: true, .. }) {
            row.push(match field {
                Field::Secret { alert: true, .. } => Span::styled(value, Style::new().fg(ALERT)),
                _ => Span::raw(value),
            });
        }
        // A measure's unit belongs to the screen and never to the user. It draws
        // grey after the value, so the field line reads its own meaning.
        if let Field::Measure { unit, .. } = field {
            row.push(Span::styled(format!(" {unit}"), Style::new().dim()));
        }
        if editing {
            let caret = CARET.with(std::cell::Cell::get);
            row.push(Span::styled(
                "\u{258e}",
                Style::new().fg(blend(caret as usize % CARET_STEPS, CARET_STEPS)),
            ));
        }
        lines.push(Line::from(row));
        rows_began = true;
        if here {
            focus = field_at;
        }
        let Field::Pick {
            options,
            at,
            always,
            ..
        } = field
        else {
            continue;
        };
        let hot = open.filter(|_| here);
        // A drop-down shows its options only while it is open. A radio group draws
        // always, so its answer reads with the list closed.
        let any = options.iter().any(|choice| !choice.hidden);
        if hot.is_none() && !*always && any {
            continue;
        }
        let room = form_room();
        let mut option_line = field_at + 1;
        if !any {
            lines.push(Line::from(Span::styled(
                format!("      {}", NO_CHOICES),
                Style::new().dim(),
            )));
            continue;
        }
        for (n, choice) in options.iter().enumerate() {
            if choice.hidden {
                continue;
            }
            let on_it = hot == Some(n);
            let row = match (choice.available, on_it) {
                (false, _) => Style::new().dim(),
                (true, true) => Style::new().fg(HIGHLIGHT).bold(),
                (true, false) => Style::new(),
            };
            // A pick chooses among options, so its options draw as radio buttons.
            // A radio group's dot marks the answer it holds. A drop-down's dot
            // follows the cursor that is about to take it.
            let checked = match *always {
                true => *at == Some(n),
                false => on_it,
            };
            let radio = match checked {
                true => "\u{25c9} ",
                false => "\u{25cb} ",
            };
            let marker = match on_it {
                true => "    > ",
                false => "      ",
            };
            // A detail longer than the room left on the option row moves to its
            // own wrapped line under that option. A clipped reason is no reason.
            let alone =
                marker.chars().count() + radio.chars().count() + choice.label.chars().count();
            let wrapped =
                !choice.detail.is_empty() && alone + 2 + choice.detail.chars().count() > room;
            lines.push(Line::from(vec![
                Span::styled(marker, row),
                Span::styled(radio, row),
                Span::styled(choice.label.clone(), row),
                match wrapped {
                    true => Span::raw(""),
                    false => Span::raw("  "),
                },
                match wrapped {
                    true => Span::raw(""),
                    false => Span::styled(choice.detail.clone(), Style::new().dim()),
                },
            ]));
            if on_it {
                focus = option_line;
            }
            option_line += 1;
            if wrapped {
                for line in table::wrap(&choice.detail, room.saturating_sub(10)) {
                    lines.push(Line::from(Span::styled(
                        format!("          {line}"),
                        Style::new().dim(),
                    )));
                    option_line += 1;
                }
            }
        }
    }

    // Draws the blank row, the action buttons, and the reason the first button is
    // dim. The form bottom-aligns that group in the widget box. A form with no
    // actions has no bottom group, and a refused enter still owes its reason under
    // the fields.
    let mut tail = lines.len();
    if !actions.is_empty() {
        lines.push(Line::default());
        let on_actions = cursor == visible.len();
        // Buttons align left with the panel section titles. They are rows of the
        // form and never a footer of their own.
        let mut buttons: Vec<Span<'static>> = Vec::new();
        for (n, action) in actions.iter().enumerate() {
            // Only the first action is ever blocked. The other actions are the way
            // off this screen, and a screen the user cannot leave is worse than one the
            // user cannot finish.
            let style = match (n == 0 && blocked.is_some(), on_actions && n == button) {
                (true, true) => Style::new().dim().reversed(),
                (true, false) => Style::new().dim(),
                // The button under the cursor carries the palette's hot end, the
                // same colour the step line has while an install runs.
                (false, true) => Style::new().fg(HIGHLIGHT).bold().reversed(),
                (false, false) => Style::new(),
            };
            buttons.push(Span::styled(format!("  {action}  "), style));
            buttons.push(Span::raw(" "));
        }
        lines.push(Line::from(buttons));
        let actions_at = lines.len() - 1;
        // The calling command asked for a refusal reason. It draws once the action
        // has been tried, and a blank line sets it off from the buttons so it does
        // not read as a second action.
        let said = tried && blocked.is_some_and(|why| !why.is_empty());
        if said {
            if let Some(why) = blocked {
                refusing(&mut lines, why, "  ");
            }
        }
        // On a field, the loop already set `focus`. On the actions row, `focus`
        // takes the buttons, or the refusal reason where there is one.
        if cursor >= visible.len() {
            focus = match said {
                true => lines.len() - 1,
                false => actions_at,
            };
        }
    } else {
        // This form has no button to try, so the last field's enter is the
        // decision. The refusal reason draws under the fields once tried, aligned
        // with the property names the reason is about.
        let said = tried && blocked.is_some_and(|why| !why.is_empty());
        if said {
            if let Some(why) = blocked {
                refusing(&mut lines, why, "     ");
            }
        }
        // The refusal reason is what the user needs, so it stays in view wherever
        // the cursor is. The widget box's spare rows go under it. The reason belongs
        // one blank under the field it refused, and never at the bottom of the
        // overlay with the padding between them.
        if said || cursor >= visible.len() {
            focus = lines.len().saturating_sub(1);
        }
        tail = lines.len();
    }
    (lines, focus, tail)
}

/// Draws one sentence under a blank row, wrapped to the room the widget has and
/// indented to the rows it speaks for. Each line carries the indent itself,
/// because `table::wrap` drops leading whitespace.
fn refusing(lines: &mut Vec<Line<'static>>, why: &str, indent: &str) {
    let room = form_room().saturating_sub(indent.chars().count());
    lines.push(Line::default());
    for line in table::wrap(why, room) {
        lines.push(Line::from(Span::styled(
            format!("{indent}{line}"),
            Style::new().dim(),
        )));
    }
}
