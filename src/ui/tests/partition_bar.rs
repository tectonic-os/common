use super::*;
use ratatui::text::Span;

/// Reads a partition bar as text. A label letter reads as itself. A block
/// reads ` ` in a neighbour box, `.` in the free region, `#` in the new
/// partition and `!` in a partition with no room.
fn read(spans: &[Span]) -> String {
    spans
        .iter()
        .map(|span| match span.style.fg {
            Some(Color::Black) => span.content.to_string(),
            Some(NEIGHBOUR) => " ".to_string(),
            Some(TRACK) => ".".to_string(),
            Some(ALERT) => "!".to_string(),
            _ => "#".to_string(),
        })
        .collect()
}

fn hole(before: Option<&str>, after: Option<&str>, gb: u64) -> Hole {
    Hole {
        before: before.map(str::to_string),
        after: after.map(str::to_string),
        gb,
    }
}

#[test]
fn the_new_partition_sits_at_its_offset_inside_the_free_region() {
    let holes = [hole(Some("efi"), Some("home"), 100)];
    assert_eq!(
        read(&placed(&holes, 50, 25, 40)),
        " efi ...50.0 GB....#######........ home "
    );
}

#[test]
fn a_free_region_at_the_end_of_the_disk_draws_no_box_after_it() {
    let holes = [hole(Some("root"), None, 10)];
    assert_eq!(
        read(&placed(&holes, 0, 5, 26)),
        " root ##5.0 GB##..5.0 GB.."
    );
}

/// The first region is too small for the size, so a bar that took the
/// first region would show the partition where the calling command never
/// places it.
#[test]
fn the_bar_takes_the_lowest_free_region_with_room() {
    let holes = [
        hole(Some("efi"), Some("root"), 5),
        hole(Some("root"), None, 100),
    ];
    assert_eq!(
        read(&placed(&holes, 0, 50, 26)),
        " root #50.0 GB##.50.0 GB.."
    );
}

#[test]
fn a_partition_with_no_room_draws_red_in_the_largest_region() {
    let holes = [hole(None, None, 5), hole(None, None, 10)];
    assert_eq!(
        read(&placed(&holes, 5, 20, 30)),
        "....5.0 GB.....!!!!20.0 GB!!!!"
    );
}

#[test]
fn an_empty_size_draws_the_free_region_alone() {
    let holes = [hole(None, Some("efi"), 10)];
    assert_eq!(read(&placed(&holes, 0, 0, 20)), "....10.0 GB.... efi ");
}

/// The window asks the offset and the size on two measure fields. The
/// header bar reads them on every draw, so it moves while the user types.
#[test]
fn the_placement_line_reads_the_typed_offset_and_size() {
    let fields = [
        Field::measure("offset", "2", "GB"),
        Field::measure("size", "5", "GB"),
    ];
    let header = [HeaderLine::Placement {
        holes: vec![hole(Some("efi"), None, 10)],
        offset: 0,
        size: 1,
        available: "available".to_string(),
    }];
    let (lines, _, _) = drawn_with(&fields, &header);
    let holes = [hole(Some("efi"), None, 10)];
    assert_eq!(
        read(&lines[0].spans),
        read(&placed(&holes, 2, 5, form_room()))
    );
}

/// The first region is too small for the size, so the bar draws the second.
/// A sum of both regions would state room no one partition can take.
#[test]
fn the_available_line_states_the_room_of_the_region_drawn() {
    let fields = [
        Field::measure("offset", "0", "GB"),
        Field::measure("size", "50", "GB"),
    ];
    let header = [HeaderLine::Placement {
        holes: vec![hole(None, Some("root"), 5), hole(Some("root"), None, 100)],
        offset: 0,
        size: 1,
        available: "available".to_string(),
    }];
    let (lines, _, _) = drawn_with(&fields, &header);
    assert!(lines[1].spans.is_empty(), "{:?}", lines[1]);
    assert!(
        lines[2].to_string().ends_with("available  100.0 GB"),
        "{:?}",
        lines[2]
    );
}

/// A size too long for a `u64` must not read as an empty field, because the
/// window refuses it and a bar with no partition would say nothing is wrong.
#[test]
fn an_overflowing_size_draws_red() {
    let fields = [
        Field::measure("offset", "0", "GB"),
        Field::measure("size", "999999999999999999999", "GB"),
    ];
    let header = [HeaderLine::Placement {
        holes: vec![hole(None, None, 10)],
        offset: 0,
        size: 1,
        available: "available".to_string(),
    }];
    let (lines, _, _) = drawn_with(&fields, &header);
    assert!(read(&lines[0].spans).contains('!'), "{:?}", lines[0]);
}

#[test]
fn a_panel_line_under_the_bar_is_set_apart_by_a_blank() {
    let fields = [
        Field::measure("offset", "0", "GB"),
        Field::measure("size", "", "GB"),
    ];
    let header = [
        HeaderLine::Placement {
            holes: vec![hole(None, None, 10)],
            offset: 0,
            size: 1,
            available: "available".to_string(),
        },
        HeaderLine::Pair("disk".to_string(), "vda".to_string()),
    ];
    let (lines, _, _) = drawn_with(&fields, &header);
    assert!(lines[3].spans.is_empty(), "{:?}", lines[3]);
    assert!(lines[4].to_string().contains("disk"), "{:?}", lines[4]);
}

fn drawn_with(
    fields: &[Field],
    header: &[HeaderLine],
) -> (Vec<ratatui::text::Line<'static>>, usize, usize) {
    laid_out(
        fields,
        &[0, 1],
        1,
        0,
        &Mode::Rows,
        &[],
        None,
        false,
        header,
        &[],
    )
}
