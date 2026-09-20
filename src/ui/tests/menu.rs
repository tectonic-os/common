use super::*;

/// The popup menu shows a menu item's children only while that item stands open.
/// The overlay window draws what the item holds.
#[test]
fn the_menu_shows_children_under_the_item_that_is_open() {
    let items = [
        MenuItem::under("Assign", &["/boot/efi", "/"]),
        MenuItem::new("Reset changes"),
    ];
    assert_eq!(menu_rows(&items, None), [(0, None), (1, None)]);
    assert_eq!(
        menu_rows(&items, Some(0)),
        [(0, None), (0, Some(0)), (0, Some(1)), (1, None)]
    );

    let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
    terminal
        .draw(|frame| {
            menu_draw(frame, frame.area(), "/dev/vda1", &items, Some(0), 1);
        })
        .unwrap();
    let drawn = terminal.backend().to_string();
    for phrase in ["Assign", "/boot/efi", "Reset changes"] {
        assert!(drawn.contains(phrase), "{phrase} is not drawn: {drawn}");
    }
}
