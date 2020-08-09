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
    area_size: [f32; 2],
    hidpi_factor: f64,
    window_rects: Vec<Rect>,
    window_z_orders: Vec<u32>,
    bottom_to_top_list: Vec<WinId>,
    frame_metrics: FrameMetrics,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct FrameMetrics {
    pub(crate) border_thickness: f64,
    pub(crate) title_bar_height: f64,
    pub(crate) gap_below_title_bar: f64,
}

impl FrameMetrics {
    pub(crate) fn with_hidpi_factor(hidpi_factor: f64) -> Self {
        let border_thickness;
        let title_bar_height;
        let gap_below_title_bar;
        if hidpi_factor < 1.51 {
            border_thickness = 4.0 / hidpi_factor;
            gap_below_title_bar = 1.0 / hidpi_factor;
        } else {
            border_thickness = 4.0 * hidpi_factor.round() / hidpi_factor;
            gap_below_title_bar = 1.0 * hidpi_factor.round() / hidpi_factor;
        }
        title_bar_height = (20.0 * hidpi_factor).round() / hidpi_factor;
        eprintln!(
            "{:?}, {:?}, {:?}",
            border_thickness * hidpi_factor,
            title_bar_height * hidpi_factor,
            gap_below_title_bar * hidpi_factor
        );
        dbg!(Self {
            border_thickness,
            title_bar_height,
            gap_below_title_bar,
        })
    }
}

impl WindowingState {
    pub fn new() -> Self {
        Self {
            area_size: [16_777_216.0, 16_777_216.0],
            hidpi_factor: 1.0,
            window_rects: Vec::new(),
            window_z_orders: Vec::new(),
            bottom_to_top_list: Vec::new(),
            frame_metrics: FrameMetrics::with_hidpi_factor(1.0),
        }
    }

    pub(crate) fn set_dimensions(&mut self, area_size: [f32; 2], hidpi_factor: f64) {
        if self.hidpi_factor != hidpi_factor {
            self.frame_metrics = FrameMetrics::with_hidpi_factor(hidpi_factor);
        }
        self.area_size = area_size;
        self.hidpi_factor = hidpi_factor;
    }

    pub(crate) fn ensure_all_win_in_area(&mut self) {
        for window_rect in self.window_rects.iter_mut() {
            let border_thickness = self.frame_metrics.border_thickness as f32;
            let title_bar_height = self.frame_metrics.title_bar_height as f32;
            if window_rect.x <= -border_thickness {
                window_rect.x = -border_thickness;
            } else if window_rect.x + (window_rect.w - border_thickness * 2.0) > self.area_size[0] {
                window_rect.x = self.area_size[0] - (window_rect.w - border_thickness * 2.0);
            }
            if window_rect.w > self.area_size[0] + border_thickness * 2.0 {
                window_rect.w = self.area_size[0] + border_thickness * 2.0;
            }
            if window_rect.y <= -border_thickness {
                window_rect.y = -border_thickness;
            } else if window_rect.y + border_thickness + title_bar_height > self.area_size[1] {
                window_rect.y = self.area_size[1] - (border_thickness + title_bar_height);
            }
            if window_rect.h > self.area_size[1] + border_thickness * 2.0 {
                window_rect.h = self.area_size[1] + border_thickness * 2.0;
            }
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

    pub(crate) fn frame_metrics(&self) -> FrameMetrics {
        self.frame_metrics
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
        window_hit_test([w, h], [x, y], self.frame_metrics)
    }

    pub fn topmost_win(&self) -> Option<WinId> {
        self.bottom_to_top_list.last().copied()
    }

    /// Retrieves the `Rect` of a window. The `Rect` is adjusted to align to
    /// the physical pixel grid. Note that since the returned `Rect` contains
    /// f32 dimensions, it may not suitable for use with GUI toolkits that uses
    /// f64 internally due to the limited precision.
    pub fn win_rect(&self, win_id: WinId) -> Rect {
        let WinId(win_idx) = win_id;
        let rect = self.window_rects[win_idx as usize];
        let hidpi_factor = self.hidpi_factor as f32;
        Rect {
            x: (rect.x * hidpi_factor).round() / hidpi_factor,
            y: (rect.y * hidpi_factor).round() / hidpi_factor,
            w: (rect.w * hidpi_factor).round() / hidpi_factor,
            h: (rect.h * hidpi_factor).round() / hidpi_factor,
        }
    }

    /// Retrieves the x, y, width and height of a window. The dimensions are
    /// adjusted to align to the physical pixel grid. The calculations use f64
    /// so that the results are precise enough for GUI toolkits that uses f64
    /// internally.
    pub fn win_rect_f64(&self, win_id: WinId) -> [f64; 4] {
        let WinId(win_idx) = win_id;
        let rect = self.window_rects[win_idx as usize];
        let hidpi_factor = self.hidpi_factor;
        [
            (rect.x as f64 * hidpi_factor).round() / hidpi_factor,
            (rect.y as f64 * hidpi_factor).round() / hidpi_factor,
            (rect.w as f64 * hidpi_factor).round() / hidpi_factor,
            (rect.h as f64 * hidpi_factor).round() / hidpi_factor,
        ]
    }

    pub(crate) fn set_win_rect(&mut self, win_id: WinId, rect: Rect) {
        let WinId(win_idx) = win_id;
        self.window_rects[win_idx as usize] = rect;
    }

    pub fn win_z_order(&self, win_id: WinId) -> u32 {
        let WinId(win_idx) = win_id;
        self.window_z_orders[win_idx as usize]
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

fn window_hit_test(
    window_size: [f32; 2],
    rel_pos: [f32; 2],
    frame_metrics: FrameMetrics,
) -> Option<HitTest> {
    let [w, h] = window_size;
    let [x, y] = rel_pos;
    if x < 0.0 || y < 0.0 || x > w || y > h {
        return None;
    }

    let border_thickness = frame_metrics.border_thickness as f32;
    let title_bar_height = frame_metrics.title_bar_height as f32;

    let window_part_x = if x <= border_thickness {
        WindowPartX::LeftBorder
    } else if x >= w - border_thickness {
        WindowPartX::RightBorder
    } else {
        WindowPartX::Content
    };
    let window_part_y = if y <= border_thickness {
        WindowPartY::TopBorder
    } else if y >= h - border_thickness {
        WindowPartY::BottomBorder
    } else if y <= border_thickness + title_bar_height {
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
