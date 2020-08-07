#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HitTest {
    Content,
    TitleBarOrDragArea,
    TopBorder,
    LeftBorder,
    RightBorder,
    BottomBorder,
    TopLeftCorner,
    TopRightCorner,
    BottomLeftCorner,
    BottomRightCorner,
    // CollapseButton,
    // CloseButton,
}

const WINDOW_BORDER: f32 = 4.0;
const TITLE_BAR_HEIGHT: f32 = 24.0;

enum WindowPartX {
    LeftBorder,
    Content,
    RightBorder,
}

enum WindowPartY {
    TopBorder,
    TitleBar,
    Content,
    BottomBorder,
}

pub fn window_hit_test(window_size: [f32; 2], rel_pos: [f32; 2]) -> Option<HitTest> {
    let [w, h] = window_size;
    let [x, y] = rel_pos;
    if x < 0.0 || y < 0.0 || x > w || y > h {
        return None;
    }

    let window_part_x = if x <= WINDOW_BORDER {
        WindowPartX::LeftBorder
    } else if x >= w - WINDOW_BORDER {
        WindowPartX::RightBorder
    } else {
        WindowPartX::Content
    };
    let window_part_y = if y <= WINDOW_BORDER {
        WindowPartY::TopBorder
    } else if y >= h - WINDOW_BORDER {
        WindowPartY::BottomBorder
    } else if y <= WINDOW_BORDER + TITLE_BAR_HEIGHT {
        WindowPartY::TitleBar
    } else {
        WindowPartY::Content
    };

    Some(match (window_part_x, window_part_y) {
        (WindowPartX::Content, WindowPartY::Content) => HitTest::Content,
        (WindowPartX::LeftBorder, WindowPartY::TopBorder) => HitTest::TopLeftCorner,
        (WindowPartX::RightBorder, WindowPartY::TopBorder) => HitTest::TopRightCorner,
        (WindowPartX::LeftBorder, WindowPartY::BottomBorder) => HitTest::BottomLeftCorner,
        (WindowPartX::RightBorder, WindowPartY::BottomBorder) => HitTest::BottomRightCorner,
        (WindowPartX::LeftBorder, _) => HitTest::LeftBorder,
        (WindowPartX::RightBorder, _) => HitTest::RightBorder,
        (_, WindowPartY::TopBorder) => HitTest::TopBorder,
        (_, WindowPartY::BottomBorder) => HitTest::BottomBorder,
        _ => HitTest::TitleBarOrDragArea,
    })
}
