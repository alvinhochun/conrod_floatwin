use conrod_core::{Point, Rect};

pub fn conrod_point_to_layout_pos(point: Point, win_area_rect: Rect) -> [f32; 2] {
    let x = (point[0] - win_area_rect.left()) as f32;
    let y = (win_area_rect.top() - point[1]) as f32;
    [x, y]
}

pub fn win_rect_to_conrod_rect(win_rect: [f64; 4], win_area_rect: Rect) -> Rect {
    let [x, y, w, h] = win_rect;
    let [left, top] = win_area_rect.top_left();
    let x1 = left + x;
    let y1 = top - y;
    let x2 = left + x + w;
    let y2 = top - y - h;
    conrod_core::Rect::from_corners([x1, y1], [x2, y2])
}
