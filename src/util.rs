use conrod_core::{Point, Rect};

pub fn conrod_point_to_layout_pos(point: Point, win_area_rect: Rect) -> [f32; 2] {
    let x = (point[0] - win_area_rect.left()) as f32;
    let y = (win_area_rect.top() - point[1]) as f32;
    [x, y]
}
