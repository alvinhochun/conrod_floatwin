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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WinId(pub(super) u32);

pub struct WindowingState {
    pub(super) window_rects: Vec<Rect>,
    pub(super) window_z_orders: Vec<u32>,
    pub(super) bottom_to_top_list: Vec<WinId>,
}

impl WindowingState {
    pub fn new() -> Self {
        Self {
            window_rects: Vec::new(),
            window_z_orders: Vec::new(),
            bottom_to_top_list: Vec::new(),
        }
    }

    pub fn add(&mut self, x: f32, y: f32, w: f32, h: f32) -> WinId {
        let id = self.window_rects.len() as u32;
        self.window_rects.push(Rect { x, y, w, h });
        self.window_z_orders.push(id);
        let win_id = WinId(id);
        self.bottom_to_top_list.push(win_id);
        win_id
    }

    pub fn win_count(&self) -> usize {
        self.window_rects.len()
    }

    pub fn win_hit_test(&self, pos: [f32; 2]) -> Option<(WinId, HitTest)> {
        self.bottom_to_top_list.iter().rev().find_map(|&win_id| {
            self.specific_win_hit_test(win_id, pos)
                .map(|ht| (win_id, ht))
        })
    }

    pub fn win_hit_test_filtered<F>(&self, pos: [f32; 2], mut f: F) -> Option<(WinId, HitTest)>
    where
        F: FnMut(WinId) -> bool,
    {
        self.bottom_to_top_list.iter().rev().find_map(|&win_id| {
            if f(win_id) {
                self.specific_win_hit_test(win_id, pos)
                    .map(|ht| (win_id, ht))
            } else {
                None
            }
        })
    }

    pub fn specific_win_hit_test(&self, win_id: WinId, pos: [f32; 2]) -> Option<HitTest> {
        let WinId(win_id) = win_id;
        let win_rect = &self.window_rects[win_id as usize];
        let x = pos[0] - win_rect.x;
        let y = pos[1] - win_rect.y;
        let w = win_rect.w;
        let h = win_rect.h;
        window_hit_test([w, h], [x, y])
    }

    pub fn topmost_win(&self) -> Option<WinId> {
        self.bottom_to_top_list.last().copied()
    }

    pub fn bring_to_top(&mut self, win_id: WinId) {
        let WinId(win_idx) = win_id;
        if *self
            .bottom_to_top_list
            .last()
            .expect("There must already be at least one window.")
            != win_id
        {
            let z_order = self.window_z_orders[win_idx as usize] as usize;
            let subslice = &mut self.bottom_to_top_list[z_order..];
            subslice.rotate_left(1);
            for (i, &WinId(win)) in subslice.iter().enumerate() {
                self.window_z_orders[win as usize] = (i + z_order) as u32;
            }
        }
    }
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
