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
    window_states: Vec<Option<WindowState>>,
    window_z_orders: Vec<u32>,
    bottom_to_top_list: Vec<WinId>,
    frame_metrics: FrameMetrics,
    maybe_dragging_window: Option<(WinId, HitTest, Rect)>,
    next_auto_position: [f32; 2],
}

struct WindowState {
    rect: Rect,
    is_collapsed: bool,
}

pub struct WindowInitialState {
    pub client_size: [f32; 2],
    pub position: Option<[f32; 2]>,
    pub is_collapsed: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct FrameMetrics {
    pub(crate) border_thickness: f64,
    pub(crate) title_bar_height: f64,
    pub(crate) gap_below_title_bar: f64,
    /// The window width of a collapsed window. This includes the borders on
    /// both sides.
    pub(crate) collapsed_win_width: f64,
}

impl FrameMetrics {
    pub(crate) fn with_hidpi_factor(hidpi_factor: f64) -> Self {
        let border_thickness;
        let title_bar_height;
        let gap_below_title_bar;
        let collapsed_win_width;
        if hidpi_factor < 1.51 {
            border_thickness = 4.0 / hidpi_factor;
            gap_below_title_bar = 1.0 / hidpi_factor;
        } else {
            border_thickness = 4.0 * hidpi_factor.round() / hidpi_factor;
            gap_below_title_bar = 1.0 * hidpi_factor.round() / hidpi_factor;
        }
        title_bar_height = (20.0 * hidpi_factor).round() / hidpi_factor;
        collapsed_win_width =
            (150.0 * hidpi_factor + border_thickness * hidpi_factor * 2.0).round() / hidpi_factor;
        eprintln!(
            "{:?} - {:?}, {:?}, {:?}, {:?}",
            hidpi_factor,
            border_thickness * hidpi_factor,
            title_bar_height * hidpi_factor,
            gap_below_title_bar * hidpi_factor,
            collapsed_win_width * hidpi_factor,
        );
        dbg!(Self {
            border_thickness,
            title_bar_height,
            gap_below_title_bar,
            collapsed_win_width,
        })
    }
}

impl WindowingState {
    pub fn new() -> Self {
        Self {
            area_size: [16_777_216.0, 16_777_216.0],
            hidpi_factor: 1.0,
            window_states: Vec::new(),
            window_z_orders: Vec::new(),
            bottom_to_top_list: Vec::new(),
            frame_metrics: FrameMetrics::with_hidpi_factor(1.0),
            maybe_dragging_window: None,
            next_auto_position: [32.0, 32.0],
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
        for &mut WindowState {
            rect: ref mut window_rect,
            is_collapsed,
            ..
        } in self.window_states.iter_mut().filter_map(|x| x.as_mut())
        {
            let border_thickness = self.frame_metrics.border_thickness as f32;
            let title_bar_height = self.frame_metrics.title_bar_height as f32;
            let collapsed_win_width = self.frame_metrics.collapsed_win_width as f32;
            if window_rect.x <= -border_thickness {
                window_rect.x = -border_thickness;
            } else {
                let width_to_test = if is_collapsed {
                    collapsed_win_width - border_thickness
                } else {
                    window_rect.w - border_thickness
                };
                if window_rect.x + width_to_test > self.area_size[0] {
                    window_rect.x = self.area_size[0] - width_to_test;
                }
            }
            if !is_collapsed && window_rect.w > self.area_size[0] + border_thickness * 2.0 {
                window_rect.w = self.area_size[0] + border_thickness * 2.0;
            }
            if window_rect.y <= -border_thickness {
                window_rect.y = -border_thickness;
            } else if window_rect.y + border_thickness + title_bar_height > self.area_size[1] {
                window_rect.y = self.area_size[1] - (border_thickness + title_bar_height);
            }
            if !is_collapsed && window_rect.h > self.area_size[1] + border_thickness * 2.0 {
                window_rect.h = self.area_size[1] + border_thickness * 2.0;
            }
        }
    }

    pub fn next_id(&mut self) -> WinId {
        let id = self.window_states.len() as u32;
        self.window_states.push(None);
        self.window_z_orders.push(id);
        let win_id = WinId(id);
        self.bottom_to_top_list.push(win_id);
        win_id
    }

    /// Ensures that the window specified by `win_id` has been initialized. If
    /// the window has not been initialized, the `init` callback is called to
    /// obtain the initial states for the window.
    pub fn ensure_init<F>(&mut self, win_id: WinId, init: F)
    where
        F: FnOnce() -> WindowInitialState,
    {
        let WinId(win_idx) = win_id;
        let win = &mut self.window_states[win_idx as usize];
        if win.is_none() {
            let double_border = self.frame_metrics.border_thickness as f32 * 2.0;
            let additional_height = self.frame_metrics.title_bar_height as f32
                + self.frame_metrics.gap_below_title_bar as f32;

            let initial_state = init();
            let w = initial_state.client_size[0] + double_border;
            let h = initial_state.client_size[1] + double_border + additional_height;
            let next_auto_pos = &mut self.next_auto_position;
            let area_h = self.area_size[1];
            let [x, y] = initial_state.position.unwrap_or_else(|| {
                let mut pos = *next_auto_pos;
                if pos[1] + h > area_h {
                    let shift = pos[0] - pos[1];
                    pos = [32.0 + shift + 24.0, 32.0];
                }
                *next_auto_pos = [pos[0] + 16.0, pos[1] + 16.0];
                pos
            });
            let rect = Rect { x, y, w, h };
            *win = Some(WindowState {
                rect,
                is_collapsed: initial_state.is_collapsed,
            });
        }
    }

    pub(crate) fn frame_metrics(&self) -> FrameMetrics {
        self.frame_metrics
    }

    pub fn win_count(&self) -> usize {
        self.window_states.len()
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
        let WinId(win_idx) = win_id;
        let win = self.window_states[win_idx as usize].as_ref()?;
        let is_collapsed = win.is_collapsed;
        let win_rect = if is_collapsed {
            self.win_display_rect(win_id)?
        } else {
            win.rect
        };
        let x = pos[0] - win_rect.x;
        let y = pos[1] - win_rect.y;
        let w = win_rect.w;
        let h = win_rect.h;
        window_hit_test([w, h], [x, y], self.hidpi_factor as f32, self.frame_metrics)
    }

    pub fn topmost_win(&self) -> Option<WinId> {
        self.bottom_to_top_list.last().copied()
    }

    /// Retrieves the `Rect` of a window in its normal state. The `Rect` is
    /// adjusted to align to the physical pixel grid. Note that since the
    /// returned `Rect` contains f32 dimensions, it may not suitable for use
    /// with GUI toolkits that use f64 internally due to the limited precision.
    pub fn win_normal_rect(&self, win_id: WinId) -> Option<Rect> {
        let WinId(win_idx) = win_id;
        let win = self.window_states[win_idx as usize].as_ref()?;
        let rect = win.rect;
        let hidpi_factor = self.hidpi_factor as f32;
        Some(Rect {
            x: (rect.x * hidpi_factor).round() / hidpi_factor,
            y: (rect.y * hidpi_factor).round() / hidpi_factor,
            w: (rect.w * hidpi_factor).round() / hidpi_factor,
            h: (rect.h * hidpi_factor).round() / hidpi_factor,
        })
    }

    /// Retrieves the x, y, width and height of a window in its normal state.
    /// The dimensions are adjusted to align to the physical pixel grid. The
    /// calculations use f64 so that the results are precise enough for GUI
    /// toolkits that use f64 internally.
    pub fn win_normal_rect_f64(&self, win_id: WinId) -> Option<[f64; 4]> {
        let WinId(win_idx) = win_id;
        let win = self.window_states[win_idx as usize].as_ref()?;
        let rect = win.rect;
        let hidpi_factor = self.hidpi_factor;
        Some([
            (rect.x as f64 * hidpi_factor).round() / hidpi_factor,
            (rect.y as f64 * hidpi_factor).round() / hidpi_factor,
            (rect.w as f64 * hidpi_factor).round() / hidpi_factor,
            (rect.h as f64 * hidpi_factor).round() / hidpi_factor,
        ])
    }

    /// Retrieves the `Rect` of a window for display. The `Rect` is adjusted to
    /// align to the physical pixel grid. Note that since the returned `Rect`
    /// contains f32 dimensions, it may not suitable for use with GUI toolkits
    /// that use f64 internally due to the limited precision.
    pub fn win_display_rect(&self, win_id: WinId) -> Option<Rect> {
        let WinId(win_idx) = win_id;
        let win = self.window_states[win_idx as usize].as_ref()?;
        if win.is_collapsed {
            let rect = win.rect;
            let hidpi_factor = self.hidpi_factor as f32;
            let border_thickness = self.frame_metrics.border_thickness as f32;
            let title_bar_height = self.frame_metrics.title_bar_height as f32;
            let collapsed_win_width = self.frame_metrics.collapsed_win_width as f32;
            Some(Rect {
                x: (rect.x * hidpi_factor).round() / hidpi_factor,
                y: (rect.y * hidpi_factor).round() / hidpi_factor,
                w: collapsed_win_width,
                h: title_bar_height + border_thickness * 2.0,
            })
        } else {
            self.win_normal_rect(win_id)
        }
    }

    /// Retrieves the x, y, width and height of a window for display. The
    /// dimensions are adjusted to align to the physical pixel grid. The
    /// calculations use f64 so that the results are precise enough for GUI
    /// toolkits that use f64 internally.
    pub fn win_display_rect_f64(&self, win_id: WinId) -> Option<[f64; 4]> {
        let WinId(win_idx) = win_id;
        let win = self.window_states[win_idx as usize].as_ref()?;
        if win.is_collapsed {
            let rect = win.rect;
            let hidpi_factor = self.hidpi_factor;
            let border_thickness = self.frame_metrics.border_thickness;
            let title_bar_height = self.frame_metrics.title_bar_height;
            let collapsed_win_width = self.frame_metrics.collapsed_win_width;
            Some([
                (rect.x as f64 * hidpi_factor).round() / hidpi_factor,
                (rect.y as f64 * hidpi_factor).round() / hidpi_factor,
                collapsed_win_width,
                title_bar_height + border_thickness * 2.0,
            ])
        } else {
            self.win_normal_rect_f64(win_id)
        }
    }

    pub(crate) fn set_win_normal_rect(&mut self, win_id: WinId, rect: Rect) {
        let WinId(win_idx) = win_id;
        if let Some(win) = &mut self.window_states[win_idx as usize] {
            win.rect = rect;
        }
    }

    pub fn win_is_collapsed(&self, win_id: WinId) -> bool {
        let WinId(win_idx) = win_id;
        self.window_states[win_idx as usize]
            .as_ref()
            .map_or(false, |x| x.is_collapsed)
    }

    pub(crate) fn set_win_collapsed(&mut self, win_id: WinId, is_collapsed: bool) {
        let WinId(win_idx) = win_id;
        if let Some(win) = &mut self.window_states[win_idx as usize] {
            win.is_collapsed = is_collapsed;
        }
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

    pub fn win_drag_start(&mut self, win_id: WinId, hit_test: HitTest) -> bool {
        if let Some((dragging_win_id, _, _)) = self.maybe_dragging_window {
            if dragging_win_id == win_id {
                // Trying to drag the same window? Just continue dragging...
                self.win_drag_end(false);
            } else {
                self.win_drag_end(true);
            }
        }
        match hit_test {
            HitTest::TopBorder
            | HitTest::LeftBorder
            | HitTest::RightBorder
            | HitTest::BottomBorder
            | HitTest::TopLeftCorner
            | HitTest::TopRightCorner
            | HitTest::BottomLeftCorner
            | HitTest::BottomRightCorner
                if self.win_is_collapsed(win_id) =>
            {
                // Just don't allow resizing a collapsed window.
                return false;
            }
            _ => {}
        }
        // Use the pixel-aligned `Rect` to prevent the right/bottom edge from
        // wobbling during resize due to rounding issues.
        let initial_rect = match self.win_normal_rect(win_id) {
            Some(x) => x,
            None => return false,
        };
        self.maybe_dragging_window = Some((win_id, hit_test, initial_rect));
        true
    }

    pub fn win_drag_end(&mut self, abort: bool) {
        let (win_id, _, starting_rect) = match self.maybe_dragging_window.take() {
            Some(x) => x,
            None => return,
        };
        if abort {
            self.set_win_normal_rect(win_id, starting_rect);
        } else {
            // Round to device pixel.
            if let Some(rect) = self.win_normal_rect(win_id) {
                self.set_win_normal_rect(win_id, rect);
            }
        }
    }

    pub fn current_dragging_win(&self) -> Option<(WinId, HitTest)> {
        self.maybe_dragging_window
            .map(|(win_id, ht, _)| (win_id, ht))
    }

    pub fn win_drag_update(&mut self, offset: [f32; 2]) -> bool {
        let (win_id, dragging_hit_test, starting_rect) = match self.maybe_dragging_window {
            Some(x) => x,
            None => return false,
        };
        let hidpi_factor = self.hidpi_factor as f32;
        // Round the offset to device pixels:
        let dx = (offset[0] * hidpi_factor).round() / hidpi_factor;
        let dy = (offset[1] * hidpi_factor).round() / hidpi_factor;

        // Ensure the window being dragged is topmost.
        self.bring_to_top(win_id);

        let border_thickness = self.frame_metrics.border_thickness as f32;
        let title_bar_height = self.frame_metrics.title_bar_height as f32;

        let [area_w, area_h] = self.area_size;
        let (win_display_w, win_display_h) = {
            match self.win_display_rect(win_id) {
                Some(r) => (r.w, r.h),
                None => return false,
            }
        };

        // TODO: Make these configurable:
        let min_w = border_thickness * 2.0 + 50.0;
        let min_h = border_thickness * 2.0 + title_bar_height + 16.0;
        let snap_threshold = (12.0 * hidpi_factor).round() / hidpi_factor;
        let snap_margin = (8.0 * hidpi_factor).round() / hidpi_factor;

        let snap_move = |mut pos: f32, dim: f32, lower_edge: f32, upper_edge: f32| {
            if (pos - (lower_edge + snap_margin)).abs() < snap_threshold {
                pos = lower_edge + snap_margin;
            } else if (pos + dim - (upper_edge - snap_margin)).abs() < snap_threshold {
                pos = upper_edge - snap_margin - dim;
            }
            pos
        };
        let snap_resize_upper = |pos: f32, dim: f32, edge: f32, min_dim: f32| {
            let mut target_dim = dim;
            // Snap the border to edge if within threshold.
            if (pos + target_dim - (edge - snap_margin)).abs() < snap_threshold {
                target_dim = edge - snap_margin - pos;
            }
            // If the dimensions will end up exceeding limits, apply the
            // limits instead and don't snap.
            if target_dim < min_dim {
                dim.max(min_dim)
            } else {
                target_dim
            }
        };
        let snap_resize_lower = |pos: f32, other_pos: f32, edge: f32, min_dim: f32| {
            let mut target_pos = pos;
            // Snap the border to edge if within threshold.
            if (target_pos - (edge + snap_margin)).abs() < snap_threshold {
                target_pos = edge + snap_margin;
            }
            // If the dimensions will end up exceeding limits, apply the
            // limits instead and don't snap.
            if (other_pos - target_pos) < min_dim {
                let target_dim = (other_pos - pos).max(min_dim);
                target_pos = other_pos - target_dim;
            }
            target_pos
        };

        // Calculate horizontal dimensions:
        let (new_x, new_w);
        match dragging_hit_test {
            HitTest::Content | HitTest::TopBorder | HitTest::BottomBorder => {
                new_x = starting_rect.x;
                new_w = starting_rect.w;
            }
            HitTest::TitleBarOrDragArea => {
                new_x = snap_move(starting_rect.x + dx, win_display_w, 0.0, area_w);
                new_w = starting_rect.w;
            }
            HitTest::LeftBorder | HitTest::TopLeftCorner | HitTest::BottomLeftCorner => {
                new_x = snap_resize_lower(
                    starting_rect.x + dx,
                    starting_rect.x + starting_rect.w,
                    0.0,
                    min_w,
                );
                new_w = starting_rect.w + starting_rect.x - new_x;
            }
            HitTest::RightBorder | HitTest::TopRightCorner | HitTest::BottomRightCorner => {
                new_x = starting_rect.x;
                new_w = snap_resize_upper(starting_rect.x, starting_rect.w + dx, area_w, min_w);
            }
        }

        // Calculate vertical dimensions:
        let (new_y, new_h);
        match dragging_hit_test {
            HitTest::Content | HitTest::LeftBorder | HitTest::RightBorder => {
                new_y = starting_rect.y;
                new_h = starting_rect.h;
            }
            HitTest::TitleBarOrDragArea => {
                new_y = snap_move(starting_rect.y + dy, win_display_h, 0.0, area_h);
                new_h = starting_rect.h;
            }
            HitTest::TopBorder | HitTest::TopLeftCorner | HitTest::TopRightCorner => {
                new_y = snap_resize_lower(
                    starting_rect.y + dy,
                    starting_rect.y + starting_rect.h,
                    0.0,
                    min_h,
                );
                new_h = starting_rect.h + starting_rect.y - new_y;
            }
            HitTest::BottomBorder | HitTest::BottomLeftCorner | HitTest::BottomRightCorner => {
                new_y = starting_rect.y;
                new_h = snap_resize_upper(starting_rect.y, starting_rect.h + dy, area_h, min_h);
            }
        }

        let new_rect = Rect {
            x: new_x,
            y: new_y,
            w: new_w,
            h: new_h,
        };
        self.set_win_normal_rect(win_id, new_rect);
        true
    }
}

fn window_hit_test(
    window_size: [f32; 2],
    rel_pos: [f32; 2],
    hidpi_factor: f32,
    frame_metrics: FrameMetrics,
) -> Option<HitTest> {
    let [log_w, log_h] = window_size;
    let [log_x, log_y] = rel_pos;
    if log_x < -0.01 || log_y < -0.01 || log_x > log_w + 0.01 || log_y > log_h + 0.01 {
        return None;
    }
    let x = (log_x * hidpi_factor).round() as i32;
    let y = (log_y * hidpi_factor).round() as i32;
    let w = (log_w * hidpi_factor).round() as i32;
    let h = (log_h * hidpi_factor).round() as i32;

    let border_thickness = (frame_metrics.border_thickness as f32 * hidpi_factor).round() as i32;
    let title_bar_height = (frame_metrics.title_bar_height as f32 * hidpi_factor).round() as i32;

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

    let corner_leeway = border_thickness * 3;
    let (is_near_l, is_near_r) = if x <= corner_leeway {
        (true, false)
    } else if x >= w - corner_leeway {
        (false, true)
    } else {
        (false, false)
    };
    let (is_near_t, is_near_b) = if y <= corner_leeway {
        (true, false)
    } else if y >= h - corner_leeway {
        (false, true)
    } else {
        (false, false)
    };

    Some(match (window_part_x, window_part_y) {
        (WindowPartX::Content, WindowPartY::Content) => HitTest::Content,
        (WindowPartX::LeftBorder, WindowPartY::TopBorder) => HitTest::TopLeftCorner,
        (WindowPartX::RightBorder, WindowPartY::TopBorder) => HitTest::TopRightCorner,
        (WindowPartX::LeftBorder, WindowPartY::BottomBorder) => HitTest::BottomLeftCorner,
        (WindowPartX::RightBorder, WindowPartY::BottomBorder) => HitTest::BottomRightCorner,
        (WindowPartX::LeftBorder, _) if is_near_t => HitTest::TopLeftCorner,
        (WindowPartX::LeftBorder, _) if is_near_b => HitTest::BottomLeftCorner,
        (WindowPartX::LeftBorder, _) => HitTest::LeftBorder,
        (WindowPartX::RightBorder, _) if is_near_t => HitTest::TopRightCorner,
        (WindowPartX::RightBorder, _) if is_near_b => HitTest::BottomRightCorner,
        (WindowPartX::RightBorder, _) => HitTest::RightBorder,
        (_, WindowPartY::TopBorder) if is_near_l => HitTest::TopLeftCorner,
        (_, WindowPartY::TopBorder) if is_near_r => HitTest::TopRightCorner,
        (_, WindowPartY::TopBorder) => HitTest::TopBorder,
        (_, WindowPartY::BottomBorder) if is_near_l => HitTest::BottomLeftCorner,
        (_, WindowPartY::BottomBorder) if is_near_r => HitTest::BottomRightCorner,
        (_, WindowPartY::BottomBorder) => HitTest::BottomBorder,
        _ => HitTest::TitleBarOrDragArea,
    })
}
