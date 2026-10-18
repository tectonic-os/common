//! Draws a filtered tree of grouped options. A branch turns on every option
//! under it.

use crate::ui::choose::*;
use crate::ui::chrome::*;
use crate::ui::table;
use crate::ui::term::*;
use crate::ui::{NEST, NEST_HIDE, NEST_SHOW};
use ratatui::backend::Backend;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::Frame;

/// Holds one tree row. A row carries an option, or a branch over the deeper rows
/// after it.
pub(crate) struct Node {
    pub(crate) label: String,
    pub(crate) depth: usize,
    pub(crate) at: Option<usize>,
}

/// Builds the tree in the order the options were given. Each dotted group part
/// not already open becomes a branch, and the option follows under it.
pub(crate) fn nodes(options: &[Choice]) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut open: Vec<&str> = Vec::new();
    for (at, choice) in options.iter().enumerate() {
        let path: Vec<&str> = match choice.group.is_empty() {
            true => Vec::new(),
            false => choice.group.split('.').collect(),
        };
        let same = open
            .iter()
            .zip(&path)
            .take_while(|(held, part)| held == part)
            .count();
        open.truncate(same);
        for part in &path[same..] {
            open.push(part);
            nodes.push(Node {
                label: open.join("."),
                depth: open.len() - 1,
                at: None,
            });
        }
        nodes.push(Node {
            label: choice.label.clone(),
            depth: open.len(),
            at: Some(at),
        });
    }
    nodes
}

/// Lists the options a tree row stands for. An option row stands for itself. A
/// branch stands for everything it contains.
pub(crate) fn leaves(nodes: &[Node], at: usize) -> Vec<usize> {
    if let Some(option) = nodes[at].at {
        return vec![option];
    }
    let depth = nodes[at].depth;
    nodes[at + 1..]
        .iter()
        .take_while(|node| node.depth > depth)
        .filter_map(|node| node.at)
        .collect()
}

/// Lists the options a tree row stands for that the user may choose. An
/// unavailable option is never part of an answer, so a branch passes over it.
fn choosable(nodes: &[Node], options: &[Choice], at: usize) -> Vec<usize> {
    leaves(nodes, at)
        .into_iter()
        .filter(|leaf| options[*leaf].available)
        .collect()
}

/// Applies containment, which `Choice::parent` contradicts. A branch turns on
/// every available option it holds until all of them are on.
pub(crate) fn check(on: &mut Vec<usize>, nodes: &[Node], options: &[Choice], at: usize) {
    let leaves = choosable(nodes, options, at);
    if leaves.is_empty() {
        return;
    }
    if leaves.iter().all(|leaf| on.contains(leaf)) {
        on.retain(|held| !leaves.contains(held));
        return;
    }
    for leaf in leaves {
        if !on.contains(&leaf) {
            on.push(leaf);
        }
    }
}

/// A branch with part of its options on reads as neither on nor off. That
/// mark is what makes a closed branch worth reading. An unavailable option
/// cannot be on, so a branch counts only the options the user may choose.
pub(crate) fn checkbox(
    on: &[usize],
    nodes: &[Node],
    options: &[Choice],
    at: usize,
) -> &'static str {
    let leaves = choosable(nodes, options, at);
    match leaves.iter().filter(|leaf| on.contains(leaf)).count() {
        0 => "[ ] ",
        held if held == leaves.len() => "[x] ",
        _ => "[-] ",
    }
}

/// Lists indices into `nodes`. Without a filter it lists every tree row no
/// closed branch hides. While a filter is typed it lists every option the filter
/// matches, which flattens the tree. If `hide` is set, then an unavailable
/// option goes, and so does a branch that holds nothing else.
pub(crate) fn shown(
    nodes: &[Node],
    options: &[Choice],
    open: &[bool],
    filter: &str,
    hide: bool,
) -> Vec<usize> {
    let kept = |at: usize| !hide || !choosable(nodes, options, at).is_empty();
    shown_all(nodes, options, open, filter)
        .into_iter()
        .filter(|at| kept(*at))
        .collect()
}

fn shown_all(nodes: &[Node], options: &[Choice], open: &[bool], filter: &str) -> Vec<usize> {
    if !filter.is_empty() {
        let filter = filter.to_lowercase();
        let matches = |choice: &Choice| {
            choice.label.to_lowercase().contains(&filter)
                || choice.detail.to_lowercase().contains(&filter)
        };
        return (0..nodes.len())
            .filter(|at| {
                nodes[*at]
                    .at
                    .is_some_and(|option| matches(&options[option]))
            })
            .collect();
    }
    let mut rows = Vec::new();
    let mut hidden: Option<usize> = None;
    for (at, node) in nodes.iter().enumerate() {
        match hidden {
            Some(depth) if node.depth > depth => continue,
            _ => hidden = None,
        }
        rows.push(at);
        if node.at.is_none() && !open[at] {
            hidden = Some(node.depth);
        }
    }
    rows
}

/// Answers with indices into `options`, as `toggle` does. It never answers with
/// positions in the tree rows the filter left on screen.
pub(crate) fn nest<B: Backend>(
    terminal: &mut ratatui::Terminal<B>,
    question: &str,
    options: &[Choice],
    held: &[usize],
) -> Result<Answer, String> {
    let tree = nodes(options);
    let mut open = vec![false; tree.len()];
    let mut on = held.to_vec();
    let mut filter = String::new();
    let mut hide = false;
    let unavailable = options.iter().any(|choice| choice.dim);
    let mut state = ListState::default().with_selected(Some(0));
    loop {
        let rows = shown(&tree, options, &open, &filter, hide);
        let legend = match (unavailable, hide) {
            (false, _) => NEST,
            (true, false) => NEST_HIDE,
            (true, true) => NEST_SHOW,
        };
        render(
            terminal,
            height(tree.len()) + 2,
            legend,
            question,
            |frame, area| {
                nested(
                    frame,
                    area,
                    question,
                    options,
                    &Tree {
                        nodes: &tree,
                        rows: &rows,
                        open: &open,
                        on: &on,
                        filter: &filter,
                        legend,
                    },
                    &mut state,
                )
            },
        )?;
        let Some(key) = read()? else { continue };
        let row = state.selected().and_then(|at| rows.get(at)).copied();
        match key {
            KeyCode::Enter => {
                on.sort_unstable();
                return Ok(Answer::Chosen(on));
            }
            KeyCode::Esc if !filter.is_empty() => filter.clear(),
            KeyCode::Esc => return Ok(Answer::Cancelled),
            KeyCode::Backspace => {
                filter.pop();
            }
            KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right => {
                if let Some(at) = row {
                    match (tree[at].at.is_some(), key) {
                        (true, KeyCode::Left | KeyCode::Right) => {}
                        (true, _) => check(&mut on, &tree, options, at),
                        (false, KeyCode::Char(' ')) => check(&mut on, &tree, options, at),
                        (false, KeyCode::Left) => open[at] = false,
                        (false, _) => open[at] = true,
                    }
                }
            }
            KeyCode::Tab if unavailable => {
                hide = !hide;
                let left = shown(&tree, options, &open, &filter, hide).len();
                state.select(Some(
                    state.selected().unwrap_or(0).min(left.saturating_sub(1)),
                ));
            }
            KeyCode::Char(letter) => filter.push(letter),
            code => move_by(code, &mut state, rows.len()),
        }
    }
}

/// Holds the state of the tree screen that `nested` draws.
pub(crate) struct Tree<'a> {
    pub(crate) nodes: &'a [Node],
    pub(crate) rows: &'a [usize],
    pub(crate) open: &'a [bool],
    pub(crate) on: &'a [usize],
    pub(crate) filter: &'a str,
    pub(crate) legend: &'a str,
}

/// Draws the tree rows, the detail of the highlighted row, and the filter as it
/// is typed. A module rule's description is prose, and the detail pane under the
/// tree has room a label row does not.
pub(crate) fn nested(
    frame: &mut Frame,
    area: Rect,
    question: &str,
    options: &[Choice],
    tree: &Tree,
    state: &mut ListState,
) {
    let Tree {
        nodes,
        rows,
        open,
        on,
        filter,
        legend,
    } = *tree;
    // Inside the widget box the question sits on the border, so it takes no tree
    // row here.
    let heading = head_row(question);
    let [head, body, detail, foot] = Layout::vertical([
        Constraint::Length(u16::from(!heading.is_empty())),
        Constraint::Min(1),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .areas(area);

    // The heading takes the same palette as the widget box, the progress bar and
    // the step line. A named colour here is one the palette cannot move.
    if !heading.is_empty() {
        frame.render_widget(
            Line::from(Span::styled(
                heading.to_string(),
                Style::new().fg(ACCENT).bold(),
            )),
            head,
        );
    }
    let under = state
        .selected()
        .and_then(|at| rows.get(at))
        .and_then(|at| nodes[*at].at)
        .map_or("", |at| options[at].detail.as_str());
    frame.render_widget(
        Paragraph::new(table::wrap(under, usize::from(detail.width)).join("\n")).dim(),
        detail,
    );

    let items: Vec<ListItem> = rows
        .iter()
        .map(|at| {
            let node = &nodes[*at];
            let sign = match (node.at.is_some(), open[*at]) {
                (true, _) => "",
                (false, true) => "\u{25be} ",
                (false, false) => "\u{25b8} ",
            };
            // Dim ink reads as absent from the answer, as it does in `draw`.
            let ink = match node.at.is_some_and(|option| options[option].dim) {
                true => Style::new().dim(),
                false => Style::new(),
            };
            ListItem::new(Line::from(vec![
                Span::styled(checkbox(on, nodes, options, *at), ink),
                Span::styled("  ".repeat(node.depth), ink),
                Span::styled(sign, ink),
                Span::styled(&node.label, ink),
            ]))
        })
        .collect();
    let shown = items.len();
    // The cursor row takes the palette's hot end, so the answer about to be taken
    // reads the same as the highlighted action button on a form.
    frame.render_stateful_widget(
        List::new(items)
            .highlight_symbol("> ")
            .highlight_style(Style::new().fg(HIGHLIGHT).bold()),
        body,
        state,
    );
    // The filter carries no `filter:` label. The key legend already says what
    // typing does, and the ten columns a label costs were the tail of that legend
    // at the narrowest terminal.
    frame.render_widget(
        Line::from(vec![
            Span::raw(match filter.is_empty() {
                true => String::new(),
                false => format!("{filter}  "),
            }),
            Span::styled(hint(legend, state, body.height, shown), Style::new().dim()),
        ]),
        foot,
    );
}
