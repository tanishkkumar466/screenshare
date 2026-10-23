use link_core::{Display, DisplayLayout, InputEvent, InputRouter, PointerMotion, Rect, Route};
use uuid::Uuid;

#[test]
fn crossing_right_edge_transfers_ownership() {
    let left = Uuid::new_v4(); let right = Uuid::new_v4();
    let layout = DisplayLayout { displays: vec![
        Display { id: "left".into(), machine: left, name: "Mac".into(), bounds: Rect { x: 0, y: 0, width: 100, height: 100 }, scale: 1.0, refresh_hz: 60 },
        Display { id: "right".into(), machine: right, name: "PC".into(), bounds: Rect { x: 100, y: 0, width: 100, height: 100 }, scale: 1.0, refresh_hz: 60 },
    ]};
    let mut router = InputRouter::new("left", 99, 50);
    assert_eq!(router.route(&layout, &InputEvent::PointerMotion(PointerMotion { dx: 2, dy: 0 })), Route::Remote(right));
}
