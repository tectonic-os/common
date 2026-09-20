//! Fills a form of rows in together. It routes each key to the form row under
//! the cursor.

use crate::ui::cells::*;
use crate::ui::choose::*;
use crate::ui::chrome::*;
use crate::ui::layout::*;
use crate::ui::menu::*;
use crate::ui::review::*;
use crate::ui::term::*;
use crate::ui::SUB_KEYS;
use ratatui::crossterm::event::KeyCode;

pub enum Field {
    /// Takes a typed answer in place.
    Text { label: String, value: String },
    /// Takes a typed answer in place and draws it as its length. The form sets
    /// `alert`, never the calling command. It stays true while this answer
    /// contradicts another one and both rows have been left, and the value then
    /// draws in red.
    Secret {
        label: String,
        value: String,
        alert: bool,
    },
    /// Opens its options in place, under the form row, until one is taken.
    /// `change` hands the form back once an option is taken, because the calling
    /// command owns what comes after that row and must rebuild it. `always` marks
    /// a radio group. A radio group draws its options whether or not the row has
    /// the keys, and landing on the row moves the cursor into them.
    Pick {
        label: String,
        options: Vec<Choice>,
        at: Option<usize>,
        change: bool,
        always: bool,
    },
    /// Takes a typed number in place and draws a unit grey after the value. The
    /// field line states the measure and the user types no unit. It takes digits
    /// only.
    Measure {
        label: String,
        value: String,
        unit: String,
    },
    /// Draws a value the user cannot answer. It stays on the form because the user,
    /// about to erase a disk, must see what is going onto it.
    Fixed { label: String, value: String },
    /// Opens a command-owned screen and keeps its summary on this form.
    Action { label: String, value: String },
    /// Draws a blank line between two groups of questions. The cursor passes over
    /// the gap and it answers nothing.
    Gap,
    /// Draws an inline table in place. The table row label stands in the form's
    /// label column and the headings stand over the value columns. The arrows move
    /// a table cursor over the rows the user can rest on. Enter focuses the field.
    /// If a table row carries `menus`, enter opens them in an overlay window over
    /// the form. Otherwise enter takes the row outright.
    Table {
        label: String,
        /// Heads the first table column, whose cells are the table row names the
        /// form draws in its label column.
        column: String,
        headings: Vec<String>,
        rows: Vec<Vec<Cell>>,
        /// Allows the table cursor to rest on the row. A grey preview row refuses
        /// it.
        selectable: Vec<bool>,
        /// Lists the menu items taking the table row offers. An empty list answers
        /// the row outright.
        menus: Vec<Vec<MenuItem>>,
        /// Marks the table row the table's own cursor sits on. The form cursor and
        /// the option cursor are counted apart from this one.
        cursor: usize,
        /// Records that a disk has been chosen. It drives the table's status glyph
        /// beside the field label.
        chosen: bool,
    },
}

impl Field {
    pub fn text(label: &str, value: &str) -> Self {
        Self::Text {
            label: label.to_string(),
            value: value.to_string(),
        }
    }

    pub fn secret(label: &str, value: &str) -> Self {
        Self::Secret {
            label: label.to_string(),
            value: value.to_string(),
            alert: false,
        }
    }

    pub fn pick(label: &str, options: Vec<Choice>, at: Option<usize>) -> Self {
        Self::Pick {
            label: label.to_string(),
            options,
            at,
            change: false,
            always: false,
        }
    }

    /// Builds a pick whose answer the calling command must see before the form
    /// continues. Taking an option gives the form back instead of opening the next
    /// row, because what follows that row is the command's to rebuild.
    pub fn pick_change(label: &str, options: Vec<Choice>, at: Option<usize>) -> Self {
        Self::Pick {
            label: label.to_string(),
            options,
            at,
            change: true,
            always: false,
        }
    }

    /// Builds a radio group. It draws its options under the form row whether or
    /// not the row has the keys, and the cursor moves straight into them. Taking
    /// an option answers the row, so the calling command gets the form back as in
    /// `pick_change`.
    pub fn radio(label: &str, options: Vec<Choice>, at: Option<usize>) -> Self {
        Self::Pick {
            label: label.to_string(),
            options,
            at,
            change: true,
            always: true,
        }
    }

    /// Builds a measure, typed in place with its unit drawn grey after the value.
    /// Only digits are taken. A size is a number and the unit belongs to the
    /// screen.
    pub fn measure(label: &str, value: &str, unit: &str) -> Self {
        Self::Measure {
            label: label.to_string(),
            value: value.to_string(),
            unit: unit.to_string(),
        }
    }

    pub fn fixed(label: &str, value: &str) -> Self {
        Self::Fixed {
            label: label.to_string(),
            value: value.to_string(),
        }
    }

    pub fn action(label: &str, value: &str) -> Self {
        Self::Action {
            label: label.to_string(),
            value: value.to_string(),
        }
    }

    pub fn gap() -> Self {
        Self::Gap
    }

    pub fn table(
        label: &str,
        column: &str,
        headings: &[&str],
        rows: Vec<Vec<Cell>>,
        selectable: Vec<bool>,
        menus: Vec<Vec<MenuItem>>,
        cursor: usize,
        chosen: bool,
    ) -> Self {
        Self::Table {
            label: label.to_string(),
            column: column.to_string(),
            headings: headings.iter().map(|heading| heading.to_string()).collect(),
            cursor: cursor.min(rows.len().saturating_sub(1)),
            rows,
            selectable,
            menus,
            chosen,
        }
    }

    /// Reports a typed field that holds a value, or an inline table with a disk
    /// chosen. A pick always answers true, because its option list is its answer.
    fn answered(&self) -> bool {
        match self {
            Self::Table { chosen, .. } => *chosen,
            Self::Pick { at, .. } => at.is_some(),
            _ => !self.value().is_empty(),
        }
    }

    /// Gives the status glyph beside a form row, which is the whole of what an
    /// empty field says. Green marks an answer that is there. Amber marks one that
    /// is missing.
    pub(crate) fn status(&self) -> (bool, &'static str) {
        match self.answered() {
            true => (true, "\u{2713}"),
            false => (false, "?"),
        }
    }

    /// Reports whether the user can answer the row. The form cursor skips every
    /// other row when a field is answered and the next one opens.
    pub(crate) fn answerable(&self) -> bool {
        !matches!(self, Self::Fixed { .. } | Self::Gap)
    }

    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Text { label, .. }
            | Self::Secret { label, .. }
            | Self::Pick { label, .. }
            | Self::Measure { label, .. }
            | Self::Fixed { label, .. }
            | Self::Action { label, .. }
            | Self::Table { label, .. } => label,
            Self::Gap => "",
        }
    }

    pub fn value(&self) -> String {
        match self {
            Self::Text { value, .. }
            | Self::Secret { value, .. }
            | Self::Measure { value, .. }
            | Self::Fixed { value, .. }
            | Self::Action { value, .. } => value.clone(),
            Self::Pick { options, at, .. } => at
                .and_then(|at| options.get(at))
                .map(|choice| choice.label.clone())
                .unwrap_or_default(),
            // Inline table answers are read through `Filled::Table`.
            Self::Table { .. } | Self::Gap => String::new(),
        }
    }

    /// Gives what the form row reads as. An unanswered field reads as nothing,
    /// and the amber status glyph beside its label says the answer is missing. A
    /// measure reads with its decimal place, so the size fills one column.
    pub(crate) fn shown(&self) -> String {
        match self {
            Self::Secret { value, .. } if !revealed() && !value.is_empty() => {
                "*".repeat(value.chars().count())
            }
            Self::Measure { value, .. } => match value.parse::<f64>() {
                Ok(number) => format!("{number:.1}"),
                Err(_) => value.clone(),
            },
            Self::Table { .. } => String::new(),
            _ => self.value(),
        }
    }

    /// Gives what draws while the form row is typed into, where that differs from
    /// the resting state. A measure shows its raw digits under the caret.
    pub(crate) fn typing(&self) -> String {
        match self {
            Self::Secret { value, .. } if !revealed() => "*".repeat(value.chars().count()),
            Self::Measure { value, .. } => value.clone(),
            _ => self.value(),
        }
    }

    pub(crate) fn push(&mut self, letter: char) {
        match self {
            Self::Text { value, .. } | Self::Secret { value, .. } => value.push(letter),
            // A measure holds a number. A letter is not one of its characters, so
            // this arm drops the letter.
            Self::Measure { value, .. } if letter.is_ascii_digit() => value.push(letter),
            Self::Measure { .. }
            | Self::Pick { .. }
            | Self::Fixed { .. }
            | Self::Action { .. }
            | Self::Table { .. }
            | Self::Gap => {}
        }
    }

    pub(crate) fn pop(&mut self) {
        match self {
            Self::Text { value, .. } | Self::Secret { value, .. } | Self::Measure { value, .. } => {
                value.pop();
            }
            Self::Pick { .. }
            | Self::Fixed { .. }
            | Self::Action { .. }
            | Self::Table { .. }
            | Self::Gap => {}
        }
    }
}

/// Says what the form came back with. The calling command reads a leave key
/// itself. The installer treats it as a question. Every other command treats it
/// as an exit.
pub enum Filled {
    /// Names one action button by index. The form fields hold the answers.
    Took(usize),
    /// Names the field whose command-owned screen the user asked for.
    Opened(usize),
    /// Names by field index a pick whose answer changed. The calling command
    /// rebuilds what comes after that form row and opens the form again.
    Changed(usize),
    /// Names the inline-table row its own cursor is on. If that row carries a
    /// popup menu, this also names the menu item taken and the child under it.
    /// `Filled::Table` with no item means enter took the row itself.
    Table {
        row: usize,
        item: Option<usize>,
        child: Option<usize>,
    },
    Left,
}

/// Holds one line of the panel above the form rows. A line carries a section
/// title, or one row of what that section says. The widget chooses the ink, so a
/// calling command writes structure and almost never a colour.
pub enum HeaderLine {
    Section(String),
    Row(String),
    /// Draws a panel row whose label and answer sit in the form's own columns, so
    /// the panel reads down the same edge as the form fields.
    Pair(String, String),
    /// Draws the same panel pair for a firmware state. The label keeps the
    /// panel's own ink and only the answer takes the state colour. Green marks a
    /// state that holds. Amber marks one that does not.
    Good(String, String),
    Warn(String, String),
}

/// States what a key means. With the form cursor, it is the whole of the form's
/// state.
pub(crate) enum Mode {
    Rows,
    Typing,
    Open(usize),
    /// Gives the inline table the keys, so the arrows move its own row cursor.
    Table,
    /// Holds a table row's popup menu open inside the inline table.
    TableMenu {
        open: Option<usize>,
        cursor: usize,
    },
}

/// Maps a field index to the row it draws on. `shown` gives the drawn rows and
/// can hide fields, renumbering from zero.
pub fn opens_at(shown: &[usize], field: usize) -> usize {
    shown.iter().position(|at| *at == field).unwrap_or(0)
}

/// Draws a form holding every question at once. `actions` names the buttons under
/// the form rows. `blocked` gives the reason the first button cannot be taken,
/// and that button then draws dim and refuses the key, with the reason under it.
///
/// `blocked` is asked on every draw, because it judges the fields edited on this
/// form. A reason computed before the loop would leave the button dim however
/// complete the answers become.
pub fn form(
    fields: &mut [Field],
    actions: &[&str],
    blocked: impl Fn(&[Field]) -> Option<String>,
    invalid: impl Fn(&[Field]) -> Vec<usize>,
    shown: impl Fn(&[Field]) -> Vec<usize>,
    keys: &str,
    header: &[HeaderLine],
    sections: &[(usize, &str)],
    // Form field the cursor opens on, by its index in `fields`. If `shown` hides
    // that field, the form opens at the top.
    start: usize,
    // Form rows that have had the keys and been left again, owned by the calling
    // command. A command that rebuilds `fields` and calls `form` again after an
    // unrelated answer, such as a table row taken or a radio changed, still knows
    // which rows were left. A contradicting pair then keeps its red when another
    // answer on the form changes.
    left: &mut Vec<usize>,
) -> Result<Filled, String> {
    let rows = fields
        .iter()
        .map(|field| match field {
            Field::Table { rows, .. } => rows.len() + 1,
            _ => 1,
        })
        .sum::<usize>()
        + 3;
    // An overlay window keeps its size even where its content is shorter, so the
    // viewport must stand tall enough to hold the whole overlay.
    let view = match OVERLAY.with(std::cell::Cell::get) {
        Some((_, height)) => (rows as u16).max(height),
        None => rows as u16,
    };
    inline(view, |terminal| {
        // Places the form cursor. The calling command sends it back to the row it
        // just changed and names that row by its field index. A form whose `shown`
        // hides fields numbers its rows differently from its fields, and a command
        // holding a field index cannot know the difference.
        let mut cursor = opens_at(&shown(fields), start);
        let mut button = 0usize;
        let mut mode = Mode::Rows;
        // Records whether the blocked first action has been tried. Trying it is
        // what asks for the reason, and the form says nothing before then.
        let mut tried = false;
        loop {
            // One field can decide whether another is a question at all, so this
            // is asked on every draw.
            let visible = shown(fields);
            // A form with no actions has no row under its fields. The last field
            // ends the form, and a step past it would park the cursor on nothing.
            cursor = cursor.min(match actions.is_empty() {
                true => visible.len().saturating_sub(1),
                false => visible.len(),
            });
            let at_row = visible.get(cursor).copied();
            // A radio group draws always, so landing on its row opens it. It has
            // no closed state for the arrows to move past.
            if matches!(mode, Mode::Rows) {
                if let Some(row) = at_row {
                    if let Field::Pick { always: true, .. } = &fields[row] {
                        mode = opened(&fields[row]);
                    }
                }
            }
            // The fields `invalid` names draw red once every one of them has been
            // left.
            let flagged = alerted(fields, left, &invalid);
            for (at, field) in fields.iter_mut().enumerate() {
                if let Field::Secret { alert, .. } = field {
                    *alert = flagged.get(at).copied().unwrap_or(false);
                }
            }
            let blocked = blocked(fields);
            let (lines, focus, tail) = laid_out(
                fields,
                &visible,
                cursor,
                button,
                &mode,
                actions,
                blocked.as_deref(),
                tried,
                header,
                sections,
            );
            // While an option list or the inline table holds the keys, esc closes
            // it and leaves the row unanswered. The open-list legend says so, and
            // the form-row legend does not.
            let keys = match mode {
                Mode::Open(_) | Mode::Table | Mode::TableMenu { .. } => SUB_KEYS,
                _ => keys,
            };
            render(terminal, lines.len() as u16 + 1, keys, "", |frame, area| {
                sheet_of(frame, area, &lines, keys, focus, tail);
                // A table row's popup menu draws as an overlay window over the
                // form, the same one the table's own screen draws.
                if let (Mode::TableMenu { open, cursor }, Some(row)) = (&mode, at_row) {
                    if let Field::Table {
                        rows,
                        menus,
                        cursor: table_at,
                        ..
                    } = &fields[row]
                    {
                        let items = menus.get(*table_at).map(Vec::as_slice).unwrap_or(&[]);
                        let title = rows
                            .get(*table_at)
                            .and_then(|cells| cells.first())
                            .map(Cell::text)
                            .unwrap_or("")
                            .trim_start_matches([
                                '\u{251c}', '\u{2514}', '\u{2500}', '\u{2502}', ' ',
                            ]);
                        menu_draw(frame, area, title, items, *open, *cursor);
                    }
                }
                // Kept so an overlay opened over this form can paint the form
                // behind itself. The form outlives the terminal it was drawn on.
                BACKDROP.with(|saved| *saved.borrow_mut() = Some(frame.buffer_mut().clone()));
            })?;
            let Some(key) = read()? else { continue };
            let Some(row) = at_row else {
                // Actions row, the one form row that holds no field.
                match key {
                    KeyCode::Esc | KeyCode::Char('q') => return Ok(Filled::Left),
                    KeyCode::Up | KeyCode::Char('k') => cursor = cursor.saturating_sub(1),
                    KeyCode::Left => button = button.saturating_sub(1),
                    KeyCode::Right => button = (button + 1).min(actions.len().saturating_sub(1)),
                    // `blocked` speaks for the first action only. The other
                    // actions stay available, because a screen the user cannot leave is
                    // worse than one the user cannot finish.
                    KeyCode::Enter if button > 0 || blocked.is_none() => {
                        return Ok(Filled::Took(button))
                    }
                    // The first action draws dim. Taking it asks why.
                    KeyCode::Enter => tried = true,
                    _ => {}
                }
                continue;
            };
            // Records whether the option list or the inline table let go of the
            // keys on the way out. An arrow at either end leaves the open row and
            // never holds inside it. Applied after the match, where `fields` is
            // free.
            let mut leave: Option<bool> = None;
            match &mut mode {
                Mode::Typing => match key {
                    // Answering a field lands on the next question highlighted and
                    // never inside it. Enter opens a drop-down, and typing opens a
                    // text field. An overlay's last field submits outright where
                    // there is no action button to take instead. Esc answers
                    // nothing and holds the cursor where it is.
                    KeyCode::Enter => {
                        left_row(left, row);
                        // A measure answers a plan the calling command draws, so
                        // the form goes back and the rows it sizes redraw at once.
                        if matches!(fields[row], Field::Measure { .. }) {
                            return Ok(Filled::Changed(row));
                        }
                        let next = onward(fields, &visible, &mut cursor);
                        if cursor == visible.len() && actions.is_empty() {
                            if blocked.is_none() {
                                return Ok(Filled::Took(0));
                            }
                            tried = true;
                        }
                        mode = next;
                    }
                    KeyCode::Esc => {
                        left_row(left, row);
                        mode = Mode::Rows;
                    }
                    // Up and down move between form fields while one is typed
                    // into. Every field opens for typing as the cursor reaches it,
                    // so without this the arrow keys stop working as soon as the
                    // form is filled in.
                    KeyCode::Up => {
                        left_row(left, row);
                        mode = backward(fields, &visible, &mut cursor);
                    }
                    KeyCode::Down => {
                        left_row(left, row);
                        mode = onward(fields, &visible, &mut cursor);
                    }
                    KeyCode::Backspace => fields[row].pop(),
                    KeyCode::Char(letter) => fields[row].push(letter),
                    _ => {}
                },
                Mode::Open(at) => {
                    let Field::Pick {
                        options,
                        change,
                        always,
                        ..
                    } = &fields[row]
                    else {
                        mode = Mode::Rows;
                        continue;
                    };
                    let (change, always) = (*change, *always);
                    match key {
                        KeyCode::Enter => {
                            let taken = *at;
                            if available(options, taken) {
                                if let Field::Pick { at: held, .. } = &mut fields[row] {
                                    *held = Some(taken);
                                }
                                // What follows the form row belongs to the calling
                                // command, so a changed answer hands the form back
                                // and opens no stale row under it.
                                if change {
                                    return Ok(Filled::Changed(row));
                                }
                                mode = onward(fields, &shown(fields), &mut cursor);
                            }
                        }
                        // A radio group has no closed state to go back to, so esc
                        // becomes the overlay's own way out.
                        KeyCode::Esc | KeyCode::Char('q') => match always {
                            true => return Ok(Filled::Left),
                            false => mode = Mode::Rows,
                        },
                        // The ends of the option list are the way out of it. The
                        // form row above or below takes the cursor, so an open list
                        // the user is done with never traps them.
                        KeyCode::Up | KeyCode::Char('k') => {
                            match option_step(options, *at, false) {
                                Some(next) => *at = next,
                                None => leave = Some(false),
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => match option_step(options, *at, true)
                        {
                            Some(next) => *at = next,
                            None => leave = Some(true),
                        },
                        _ => {}
                    }
                }
                Mode::Table => {
                    let Field::Table {
                        selectable,
                        menus,
                        cursor: table_cursor,
                        ..
                    } = &mut fields[row]
                    else {
                        mode = Mode::Rows;
                        continue;
                    };
                    match key {
                        KeyCode::Esc | KeyCode::Char('q') => mode = Mode::Rows,
                        // The inline table ends the same way. Past the first or
                        // last row the user can rest on, the cursor moves to the
                        // form field above or below, which keeps a one-disk table
                        // usable.
                        KeyCode::Up | KeyCode::Char('k') => {
                            match table_step(selectable.as_slice(), *table_cursor, false) {
                                Some(next) => *table_cursor = next,
                                None => leave = Some(false),
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            match table_step(selectable.as_slice(), *table_cursor, true) {
                                Some(next) => *table_cursor = next,
                                None => leave = Some(true),
                            }
                        }
                        // If a table row carries menu items, enter opens them.
                        // Otherwise enter takes the row.
                        KeyCode::Enter => {
                            match menus.get(*table_cursor).map(Vec::as_slice).unwrap_or(&[]) {
                                [] => {
                                    return Ok(Filled::Table {
                                        row: *table_cursor,
                                        item: None,
                                        child: None,
                                    })
                                }
                                _ => {
                                    mode = Mode::TableMenu {
                                        open: None,
                                        cursor: 0,
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Mode::TableMenu { open, cursor } => {
                    let Field::Table {
                        menus,
                        cursor: table_at,
                        ..
                    } = &fields[row]
                    else {
                        mode = Mode::Rows;
                        continue;
                    };
                    let items = menus.get(*table_at).map(Vec::as_slice).unwrap_or(&[]);
                    let visible = menu_rows(items, *open);
                    match key {
                        KeyCode::Esc | KeyCode::Char('q') => match open {
                            Some(was) => {
                                let was = *was;
                                *open = None;
                                *cursor = menu_rows(items, *open)
                                    .iter()
                                    .position(|(at, _)| *at == was)
                                    .unwrap_or(0);
                            }
                            None => mode = Mode::Table,
                        },
                        KeyCode::Enter => {
                            let Some(&(item, child)) = visible.get(*cursor) else {
                                continue;
                            };
                            if child.is_some() || items[item].children.is_empty() {
                                return Ok(Filled::Table {
                                    row: *table_at,
                                    item: Some(item),
                                    child,
                                });
                            }
                            match *open {
                                Some(was) if was == item => {
                                    *open = None;
                                    *cursor = menu_rows(items, *open)
                                        .iter()
                                        .position(|(at, _)| *at == item)
                                        .unwrap_or(0);
                                }
                                _ => {
                                    *open = Some(item);
                                    *cursor = menu_rows(items, *open)
                                        .iter()
                                        .position(|(at, child)| *at == item && child.is_none())
                                        .unwrap_or(0);
                                }
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => *cursor = cursor.saturating_sub(1),
                        KeyCode::Down | KeyCode::Char('j') => {
                            *cursor = (*cursor + 1).min(visible.len().saturating_sub(1))
                        }
                        _ => {}
                    }
                }
                Mode::Rows => match key {
                    // Typing opens a text field. The user types into the form row
                    // the cursor is on and presses no enter first. `q` counts as a
                    // letter here and as a leave key everywhere else.
                    KeyCode::Char(letter)
                        if matches!(
                            fields[row],
                            Field::Text { .. } | Field::Secret { .. } | Field::Measure { .. }
                        ) =>
                    {
                        // A measure takes digits only, and a character it refuses
                        // is no edit. `q` still leaves from a size field the way it
                        // leaves from every other row.
                        let takes = !matches!(fields[row], Field::Measure { .. })
                            || letter.is_ascii_digit();
                        match takes {
                            true => {
                                fields[row].push(letter);
                                mode = Mode::Typing;
                            }
                            false if letter == 'q' => return Ok(Filled::Left),
                            false => {}
                        }
                    }
                    // Backspace edits too, and after a refused enter the form
                    // cursor sits back on the row it belongs to.
                    KeyCode::Backspace
                        if matches!(
                            fields[row],
                            Field::Text { .. } | Field::Secret { .. } | Field::Measure { .. }
                        ) =>
                    {
                        fields[row].pop();
                        mode = Mode::Typing;
                    }
                    KeyCode::Esc | KeyCode::Char('q') => return Ok(Filled::Left),
                    // The arrows skip every row the user cannot answer, a gap included,
                    // so the form cursor always rests on a question or an action.
                    KeyCode::Up | KeyCode::Char('k') => {
                        mode = backward(fields, &visible, &mut cursor)
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        mode = onward(fields, &visible, &mut cursor)
                    }
                    KeyCode::Enter if matches!(fields[row], Field::Action { .. }) => {
                        return Ok(Filled::Opened(row))
                    }
                    KeyCode::Enter => mode = opened(&fields[row]),
                    _ => {}
                },
            }
            // An arrow at the end of an open option list or inline table left it,
            // so the form cursor moves to the field above or below in the row
            // state.
            if let Some(down) = leave {
                mode = match down {
                    true => onward(fields, &shown(fields), &mut cursor),
                    false => backward(fields, &shown(fields), &mut cursor),
                };
            }
        }
    })
}

/// Moves the cursor to the next form row it can rest on. A form is a list of
/// questions, and answering one moves to the next. Nothing opens under the cursor
/// on the way. Enter opens a drop-down, typing opens a text field, and landing on
/// a radio group opens that group. Moving down stops at the actions row, because
/// taking an action is a decision and not an answer.
pub(crate) fn onward(fields: &[Field], visible: &[usize], cursor: &mut usize) -> Mode {
    loop {
        *cursor = (*cursor + 1).min(visible.len());
        match visible.get(*cursor) {
            // A form row the user cannot answer is no stop on the way down. The
            // actions row ends the walk, and the calling command draws it last.
            Some(row) if !fields[*row].answerable() => continue,
            _ => return landed(fields, visible, *cursor),
        }
    }
}

/// Moves the cursor to the form row before this one, for the arrow that goes the
/// other way.
pub(crate) fn backward(fields: &[Field], visible: &[usize], cursor: &mut usize) -> Mode {
    loop {
        let above = cursor.saturating_sub(1);
        let stuck = above == *cursor;
        *cursor = above;
        match visible.get(*cursor) {
            Some(row) if !fields[*row].answerable() && !stuck => continue,
            _ => return landed(fields, visible, *cursor),
        }
    }
}

/// States what landing on a form row means. A radio group takes the keys at
/// once, because its option list draws always. Every other field only takes the
/// highlight.
pub(crate) fn landed(fields: &[Field], visible: &[usize], cursor: usize) -> Mode {
    match visible.get(cursor) {
        Some(row) => match &fields[*row] {
            Field::Pick { always: true, .. } => opened(&fields[*row]),
            _ => Mode::Rows,
        },
        None => Mode::Rows,
    }
}

/// States what editing a form field means. It is the only difference between a
/// field the user types and one the user chooses.
pub(crate) fn opened(field: &Field) -> Mode {
    match field {
        Field::Pick { options, at, .. } => options
            .iter()
            .position(|choice| !choice.hidden)
            .map(|first| {
                Mode::Open(
                    at.filter(|at| options.get(*at).is_some_and(|choice| !choice.hidden))
                        .unwrap_or(first),
                )
            })
            .unwrap_or(Mode::Rows),
        Field::Table { .. } => Mode::Table,
        Field::Fixed { .. } | Field::Action { .. } | Field::Gap => Mode::Rows,
        _ => Mode::Typing,
    }
}

/// Moves to the next option still drawn in this popup menu. Hidden choices keep
/// their indices for held answers and never take the cursor.
pub(crate) fn option_step(options: &[Choice], at: usize, down: bool) -> Option<usize> {
    let mut range: Box<dyn Iterator<Item = usize>> = match down {
        true => Box::new(at.saturating_add(1)..options.len()),
        false => Box::new((0..at).rev()),
    };
    range.find(|next| !options[*next].hidden)
}

/// Moves the inline table's cursor one row in `down`'s direction. It passes over
/// a grey preview row and stops at both table ends. If there is nowhere inside
/// the table to go, it answers `None`, which lets the cursor leave to the form
/// field above or below.
pub(crate) fn table_step(selectable: &[bool], at: usize, down: bool) -> Option<usize> {
    let at = at.min(selectable.len().saturating_sub(1));
    let mut next = at;
    loop {
        let ahead = match down {
            true => next + 1,
            false => next.saturating_sub(1),
        };
        // Both table ends stop the cursor. A run of grey preview rows with nothing
        // answerable at the far end holds the cursor where it was.
        if ahead == next || ahead >= selectable.len() {
            return None;
        }
        next = ahead;
        if selectable.get(next).copied().unwrap_or(false) {
            return Some(next);
        }
    }
}

/// Gives the room inside the widget box the form draws in, whether that box is
/// an overlay window or the box the console allows. A form row or an option wraps
/// where it is drawn, and inline-table columns are placed against the room the
/// box actually has.
pub(crate) fn form_room() -> usize {
    match OVERLAY.with(std::cell::Cell::get) {
        Some((width, _)) => (width as usize).saturating_sub(2 + 2 * PAD_X as usize),
        None => (WIDEST.min(width() as u16) as usize).saturating_sub(2 + 2 * PAD_X as usize),
    }
}

/// Marks a form row as having had the keys and been left again. It records a row
/// once, however many times the user visits it.
fn left_row(left: &mut Vec<usize>, row: usize) {
    if !left.contains(&row) {
        left.push(row);
    }
}
