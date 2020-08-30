pub use dim::{Rect, RectF, RectI};

mod debug;
mod dim;
mod snapping;

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindowDragAction1D {
    None,
    MoveWindow,
    ResizeLower,
    ResizeUpper,
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
    maybe_dragging_window: Option<DraggingState>,
    next_auto_position: [f32; 2],
}

struct WindowState {
    rect: RectF,
    min_size: dim::SizeF,
    is_hidden: bool,
    is_collapsed: bool,
    /// This flag is used to keep track of whether the window is still being
    /// used. The method `sweep_unneeded` will remove all windows with this
    /// flag set to `false`.
    is_needed: bool,
    anchor_x: snapping::Anchor,
    anchor_y: snapping::Anchor,
}

pub struct WindowInitialState {
    pub client_size: [f32; 2],
    pub position: Option<[f32; 2]>,
    pub min_size: Option<[f32; 2]>,
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
    pub(crate) title_button_padding: f64,
    pub(crate) title_button_width: f64,
    pub(crate) title_text_padding: f64,
}

struct DraggingState {
    win_id: WinId,
    dragging_hit_test: HitTest,
    starting_rect: RectI,
    snap_candidates_x: Vec<(WinId, snapping::SnapSegmentV)>,
    last_snapped_x: Option<u32>,
    snap_candidates_y: Vec<(WinId, snapping::SnapSegmentH)>,
    last_snapped_y: Option<u32>,
}

impl HitTest {
    pub fn to_drag_action_1d<D: dim::Dir>(self) -> WindowDragAction1D {
        match <D as dim::Dir>::DIR {
            dim::Direction::Horizontal => match self {
                HitTest::Content | HitTest::TopBorder | HitTest::BottomBorder => {
                    WindowDragAction1D::None
                }
                HitTest::TitleBarOrDragArea => WindowDragAction1D::MoveWindow,
                HitTest::LeftBorder | HitTest::TopLeftCorner | HitTest::BottomLeftCorner => {
                    WindowDragAction1D::ResizeLower
                }
                HitTest::RightBorder | HitTest::TopRightCorner | HitTest::BottomRightCorner => {
                    WindowDragAction1D::ResizeUpper
                }
            },
            dim::Direction::Vertical => match self {
                HitTest::Content | HitTest::LeftBorder | HitTest::RightBorder => {
                    WindowDragAction1D::None
                }
                HitTest::TitleBarOrDragArea => WindowDragAction1D::MoveWindow,
                HitTest::TopBorder | HitTest::TopLeftCorner | HitTest::TopRightCorner => {
                    WindowDragAction1D::ResizeLower
                }
                HitTest::BottomBorder | HitTest::BottomLeftCorner | HitTest::BottomRightCorner => {
                    WindowDragAction1D::ResizeUpper
                }
            },
        }
    }
}

impl FrameMetrics {
    pub(crate) fn with_hidpi_factor(hidpi_factor: f64) -> Self {
        let dpi_int = if hidpi_factor.fract() < 0.51 {
            hidpi_factor.trunc()
        } else {
            hidpi_factor.trunc() + 1.0
        };
        let border_thickness = 4.0 * dpi_int / hidpi_factor;
        let gap_below_title_bar = 1.0 * dpi_int / hidpi_factor;
        let title_bar_height = (18.0 * hidpi_factor).round() / hidpi_factor;
        let collapsed_win_width =
            (150.0 * hidpi_factor + border_thickness * hidpi_factor * 2.0).round() / hidpi_factor;
        let title_button_padding = (2.0 * hidpi_factor).round() / hidpi_factor;
        let title_button_width = (16.0 * hidpi_factor).round() / hidpi_factor;
        let title_text_padding = (4.0 * hidpi_factor).round() / hidpi_factor;
        Self {
            border_thickness,
            title_bar_height,
            gap_below_title_bar,
            collapsed_win_width,
            title_button_padding,
            title_button_width,
            title_text_padding,
        }
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
        let mut has_changed = false;
        if self.area_size != area_size {
            has_changed = true;
        }
        if self.hidpi_factor != hidpi_factor {
            self.frame_metrics = FrameMetrics::with_hidpi_factor(hidpi_factor);
            has_changed = true;
        }
        self.area_size = area_size;
        self.hidpi_factor = hidpi_factor;
        if has_changed {
            self.recompute_snapped_win_rects();
        }
    }

    fn recompute_snapped_win_rects(&mut self) {
        for i in 0..self.window_states.len() {
            let win_id = WinId(i as u32);
            self.win_recompute_snapping_rect(win_id);
        }
    }

    fn win_recompute_snapping_rect(&mut self, win_id: WinId) {
        let win_idx = win_id.0 as usize;
        let win = match &self.window_states[win_idx] {
            Some(win) => win,
            None => return,
        };
        if win.is_hidden {
            return;
        }
        if win.anchor_x == snapping::Anchor::None && win.anchor_y == snapping::Anchor::None {
            return;
        }

        let hidpi_factor = self.hidpi_factor as f32;
        let border_thickness = self.frame_metrics.border_thickness as f32;
        let title_bar_height = self.frame_metrics.title_bar_height as f32;
        let area_w = (self.area_size[0] * hidpi_factor) as i32;
        let area_h = (self.area_size[1] * hidpi_factor) as i32;
        let snap_margin = (8.0 * hidpi_factor).round() as i32;

        let mut rect = self
            .win_normal_rect_int(win_id)
            .unwrap_or_else(|| unreachable!());
        let display_size = self
            .win_display_rect_int(win_id)
            .unwrap_or_else(|| unreachable!())
            .size();
        let min_w = ((border_thickness * 2.0 + win.min_size.w) * hidpi_factor).round() as i32;
        let min_h = ((border_thickness * 2.0 + title_bar_height + win.min_size.h) * hidpi_factor)
            .round() as i32;

        match win.anchor_x {
            snapping::Anchor::None => {}
            snapping::Anchor::LowerEdge => {
                rect.x = 0 + snap_margin;
            }
            snapping::Anchor::UpperEdge => {
                rect.x = area_w - display_size.w - snap_margin;
            }
            snapping::Anchor::LowerAndUpperEdges => {
                rect.x = 0 + snap_margin;
                rect.w = min_w.max(area_w - rect.x - snap_margin);
            }
        }
        match win.anchor_y {
            snapping::Anchor::None => {}
            snapping::Anchor::LowerEdge => {
                rect.y = 0 + snap_margin;
            }
            snapping::Anchor::UpperEdge => {
                rect.y = area_h - display_size.h - snap_margin;
            }
            snapping::Anchor::LowerAndUpperEdges => {
                rect.y = 0 + snap_margin;
                rect.h = min_h.max(area_h - rect.y - snap_margin);
            }
        }
        self.set_win_normal_rect_int(win_id, rect);
    }

    pub(crate) fn ensure_all_win_in_area(&mut self) {
        let border_thickness = self.frame_metrics.border_thickness as f32;
        let title_bar_height = self.frame_metrics.title_bar_height as f32;
        let collapsed_win_width = self.frame_metrics.collapsed_win_width as f32;

        for &mut WindowState {
            rect: ref mut window_rect,
            min_size,
            is_hidden,
            is_collapsed,
            ..
        } in self.window_states.iter_mut().filter_map(|x| x.as_mut())
        {
            if is_hidden {
                continue;
            }
            let width_to_test = if is_collapsed {
                collapsed_win_width - border_thickness
            } else {
                collapsed_win_width.min(window_rect.w) - border_thickness
            };
            let display_width = if is_collapsed {
                collapsed_win_width
            } else {
                window_rect.w
            };
            if window_rect.x <= width_to_test - display_width - border_thickness {
                window_rect.x = width_to_test - display_width - border_thickness;
            } else if window_rect.x > self.area_size[0] - width_to_test {
                window_rect.x = self.area_size[0] - width_to_test;
            }
            if window_rect.y <= -border_thickness {
                window_rect.y = -border_thickness;
            } else if window_rect.y > self.area_size[1] - (border_thickness + title_bar_height) {
                window_rect.y = self.area_size[1] - (border_thickness + title_bar_height);
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
            let min_size: dim::SizeF = initial_state
                .min_size
                .unwrap_or_else(|| [150.0, 50.0])
                .into();
            let w = initial_state.client_size[0].max(min_size.w) + double_border;
            let h =
                initial_state.client_size[1].max(min_size.h) + double_border + additional_height;
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
            let rect = RectF { x, y, w, h };
            *win = Some(WindowState {
                rect,
                min_size,
                is_hidden: false,
                is_collapsed: initial_state.is_collapsed,
                is_needed: true,
                anchor_x: snapping::Anchor::None,
                anchor_y: snapping::Anchor::None,
            });
            self.bring_to_top(win_id);
        }
    }

    pub(crate) fn frame_metrics(&self) -> FrameMetrics {
        self.frame_metrics
    }

    pub fn win_count(&self) -> usize {
        self.window_states.len()
    }

    pub(crate) fn set_needed(&mut self, win_id: WinId, is_needed: bool) {
        let WinId(win_idx) = win_id;
        if let Some(win) = &mut self.window_states[win_idx as usize] {
            win.is_needed = is_needed;
        }
    }

    pub(crate) fn set_all_needed(&mut self, is_needed: bool) {
        for win in self.window_states.iter_mut().filter_map(|x| x.as_mut()) {
            win.is_needed = is_needed;
        }
    }

    pub(crate) fn sweep_unneeded(&mut self) {
        for win in &mut self.window_states {
            if win.as_ref().map_or(false, |x| !x.is_needed) {
                *win = None;
            }
        }
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
        if win.is_hidden {
            return None;
        }
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
    pub fn win_normal_rect(&self, win_id: WinId) -> Option<RectF> {
        let WinId(win_idx) = win_id;
        let win = self.window_states[win_idx as usize].as_ref()?;
        let rect = win.rect;
        let hidpi_factor = self.hidpi_factor as f32;
        Some(RectF {
            x: (rect.x * hidpi_factor).round() / hidpi_factor,
            y: (rect.y * hidpi_factor).round() / hidpi_factor,
            w: (rect.w * hidpi_factor).round() / hidpi_factor,
            h: (rect.h * hidpi_factor).round() / hidpi_factor,
        })
    }

    /// Retrieves the `RectInt` of a window in its normal state. The `RectInt`
    /// is in unscaled physical pixels.
    pub fn win_normal_rect_int(&self, win_id: WinId) -> Option<RectI> {
        let WinId(win_idx) = win_id;
        let win = self.window_states[win_idx as usize].as_ref()?;
        let rect = win.rect;
        let hidpi_factor = self.hidpi_factor as f32;
        Some(RectI {
            x: (rect.x * hidpi_factor).round() as i32,
            y: (rect.y * hidpi_factor).round() as i32,
            w: (rect.w * hidpi_factor).round() as i32,
            h: (rect.h * hidpi_factor).round() as i32,
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
    pub fn win_display_rect(&self, win_id: WinId) -> Option<RectF> {
        let WinId(win_idx) = win_id;
        let win = self.window_states[win_idx as usize].as_ref()?;
        if win.is_hidden {
            return None;
        }
        if win.is_collapsed {
            let rect = win.rect;
            let hidpi_factor = self.hidpi_factor as f32;
            let border_thickness = self.frame_metrics.border_thickness as f32;
            let title_bar_height = self.frame_metrics.title_bar_height as f32;
            let collapsed_win_width = self.frame_metrics.collapsed_win_width as f32;
            Some(RectF {
                x: (rect.x * hidpi_factor).round() / hidpi_factor,
                y: (rect.y * hidpi_factor).round() / hidpi_factor,
                w: collapsed_win_width,
                h: title_bar_height + border_thickness * 2.0,
            })
        } else {
            self.win_normal_rect(win_id)
        }
    }

    /// Retrieves the `RectInt` of a window for display. The `RectInt` is in
    /// unscaled physical pixels.
    pub fn win_display_rect_int(&self, win_id: WinId) -> Option<RectI> {
        let WinId(win_idx) = win_id;
        let win = self.window_states[win_idx as usize].as_ref()?;
        if win.is_hidden {
            return None;
        }
        if win.is_collapsed {
            let rect = win.rect;
            let hidpi_factor = self.hidpi_factor as f32;
            let border_thickness = self.frame_metrics.border_thickness as f32;
            let title_bar_height = self.frame_metrics.title_bar_height as f32;
            let collapsed_win_width = self.frame_metrics.collapsed_win_width as f32;
            Some(RectI {
                x: (rect.x * hidpi_factor).round() as i32,
                y: (rect.y * hidpi_factor).round() as i32,
                w: (collapsed_win_width * hidpi_factor).round() as i32,
                h: ((title_bar_height + border_thickness * 2.0) * hidpi_factor).round() as i32,
            })
        } else {
            self.win_normal_rect_int(win_id)
        }
    }

    /// Retrieves the x, y, width and height of a window for display. The
    /// dimensions are adjusted to align to the physical pixel grid. The
    /// calculations use f64 so that the results are precise enough for GUI
    /// toolkits that use f64 internally.
    pub fn win_display_rect_f64(&self, win_id: WinId) -> Option<[f64; 4]> {
        let WinId(win_idx) = win_id;
        let win = self.window_states[win_idx as usize].as_ref()?;
        if win.is_hidden {
            return None;
        }
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

    pub(crate) fn set_win_normal_rect(&mut self, win_id: WinId, rect: RectF) {
        let WinId(win_idx) = win_id;
        if let Some(win) = &mut self.window_states[win_idx as usize] {
            win.rect = rect;
        }
    }

    pub(crate) fn set_win_normal_rect_int(&mut self, win_id: WinId, rect: RectI) {
        let WinId(win_idx) = win_id;
        let hidpi_factor = self.hidpi_factor as f32;
        if let Some(win) = &mut self.window_states[win_idx as usize] {
            win.rect = RectF {
                x: rect.x as f32 / hidpi_factor,
                y: rect.y as f32 / hidpi_factor,
                w: rect.w as f32 / hidpi_factor,
                h: rect.h as f32 / hidpi_factor,
            };
        }
    }

    pub fn set_win_min_size(&mut self, win_id: WinId, min_size: [f32; 2]) {
        let WinId(win_idx) = win_id;
        if let Some(win) = &mut self.window_states[win_idx as usize] {
            let min_size: dim::SizeF = min_size.into();
            if win.min_size.w < min_size.w || win.min_size.h < min_size.h {
                // The new `min_size` is larger than the existing one, so we
                // might need to expand the window.
                let border_thickness = self.frame_metrics.border_thickness as f32;
                let title_bar_height = self.frame_metrics.title_bar_height as f32;
                let min_w = border_thickness * 2.0 + min_size.w;
                let min_h = border_thickness * 2.0 + title_bar_height + min_size.h;
                if win.rect.w < min_w {
                    win.rect.w = min_w;
                }
                if win.rect.h < min_h {
                    win.rect.h = min_h;
                }
            }
            win.min_size = min_size;
        }
    }

    pub fn win_is_hidden(&self, win_id: WinId) -> bool {
        let WinId(win_idx) = win_id;
        self.window_states[win_idx as usize]
            .as_ref()
            .map_or(true, |x| x.is_hidden)
    }

    pub(crate) fn set_win_hidden(&mut self, win_id: WinId, is_hidden: bool) {
        let WinId(win_idx) = win_id;
        let win = match &self.window_states[win_idx as usize] {
            Some(win) => win,
            None => return,
        };
        if win.is_hidden == is_hidden {
            return;
        }

        let win = self.window_states[win_idx as usize]
            .as_mut()
            .unwrap_or_else(|| unreachable!());
        win.is_hidden = is_hidden;

        self.win_recompute_snapping_rect(win_id);
    }

    pub fn win_is_collapsed(&self, win_id: WinId) -> bool {
        let WinId(win_idx) = win_id;
        self.window_states[win_idx as usize]
            .as_ref()
            .map_or(false, |x| x.is_collapsed)
    }

    pub(crate) fn set_win_collapsed(&mut self, win_id: WinId, is_collapsed: bool) {
        let WinId(win_idx) = win_id;
        let win = match &self.window_states[win_idx as usize] {
            Some(win) => win,
            None => return,
        };
        if win.is_collapsed == is_collapsed {
            return;
        }

        let win = self.window_states[win_idx as usize]
            .as_mut()
            .unwrap_or_else(|| unreachable!());
        win.is_collapsed = is_collapsed;

        self.win_recompute_snapping_rect(win_id);
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

    pub fn win_drag_start(&mut self, win_id: WinId, dragging_hit_test: HitTest) -> bool {
        if let Some(DraggingState {
            win_id: dragging_win_id,
            ..
        }) = self.maybe_dragging_window
        {
            if dragging_win_id == win_id {
                // Trying to drag the same window? Just continue dragging...
                self.win_drag_end(false);
            } else {
                self.win_drag_end(true);
            }
        }
        match dragging_hit_test {
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
        let starting_rect = match self.win_normal_rect_int(win_id) {
            Some(x) => x,
            None => return false,
        };

        let hidpi_factor = self.hidpi_factor as f32;
        let snap_margin = (8.0 * hidpi_factor).round() as i32;
        let win_display_size = {
            match self.win_display_rect_int(win_id) {
                Some(r) => r.size(),
                None => return false,
            }
        };

        fn snap_candidates<D: dim::Dir, Iter>(
            dragging_hit_test: HitTest,
            win_rect_iter: Iter,
            snap_margin: i32,
            win_display_size: dim::SizeI,
        ) -> Vec<(WinId, snapping::SnapSegment<D::PerpendicularDir>)>
        where
            Iter: Iterator<Item = (WinId, RectI)>,
        {
            let iter = win_rect_iter.map(|(win_id, rect)| {
                let dim_range = rect.range::<D::PerpendicularDir>();
                (win_id, rect, dim_range)
            });
            match dragging_hit_test.to_drag_action_1d::<D>() {
                WindowDragAction1D::None => {
                    // Nothing to snap in this direction.
                    Vec::new()
                }
                WindowDragAction1D::MoveWindow => {
                    // Gather a list of all lower and upper borders.
                    iter.flat_map(|(win_id, rect, dim_range)| {
                        std::iter::once((
                            win_id,
                            snapping::SnapSegment::new(
                                rect.pos().dim::<D>() - snap_margin - win_display_size.dim::<D>(),
                                dim_range,
                            ),
                        ))
                        .chain(std::iter::once((
                            win_id,
                            snapping::SnapSegment::new(
                                rect.pos().dim::<D>() + rect.size().dim::<D>() + snap_margin,
                                dim_range,
                            ),
                        )))
                    })
                    .collect()
                }
                WindowDragAction1D::ResizeLower => {
                    // Gather a list of all upper borders.
                    iter.map(|(win_id, rect, dim_range)| {
                        (
                            win_id,
                            snapping::SnapSegment::new(
                                rect.pos().dim::<D>() + rect.size().dim::<D>() + snap_margin,
                                dim_range,
                            ),
                        )
                    })
                    .collect()
                }
                WindowDragAction1D::ResizeUpper => {
                    // Gather a list of all lower borders.
                    iter.map(|(win_id, rect, dim_range)| {
                        (
                            win_id,
                            snapping::SnapSegment::new(
                                rect.pos().dim::<D>() - snap_margin,
                                dim_range,
                            ),
                        )
                    })
                    .collect()
                }
            }
        }

        // Gather a list of borders of other windows that could
        // possibly be snapped to.
        // TODO: Possible optimization by filtering out impossible borders.
        let base_iter = self
            .window_states
            .iter()
            .enumerate()
            .filter_map(|(i, _)| {
                let i_win_id = WinId(i as u32);
                if i_win_id != win_id {
                    Some(i_win_id)
                } else {
                    None
                }
            })
            .filter_map(|win_id| {
                let rect = self.win_display_rect_int(win_id)?;
                Some((win_id, rect))
            });
        let snap_candidates_x = snap_candidates::<dim::Horizontal, _>(
            dragging_hit_test,
            base_iter.clone(),
            snap_margin,
            win_display_size,
        );
        let snap_candidates_y = snap_candidates::<dim::Vertical, _>(
            dragging_hit_test,
            base_iter,
            snap_margin,
            win_display_size,
        );
        self.maybe_dragging_window = Some(DraggingState {
            win_id,
            dragging_hit_test,
            starting_rect,
            snap_candidates_x,
            last_snapped_x: None,
            snap_candidates_y,
            last_snapped_y: None,
        });
        true
    }

    pub fn win_drag_end(&mut self, abort: bool) {
        let DraggingState {
            win_id,
            starting_rect,
            ..
        } = match self.maybe_dragging_window.take() {
            Some(x) => x,
            None => return,
        };
        if abort {
            self.set_win_normal_rect_int(win_id, starting_rect);
        } else {
            // Check whether the window was snapped to the area edges.
            let rect = match self.win_normal_rect_int(win_id) {
                Some(r) => r,
                None => return,
            };
            let display_size = match self.win_display_rect_int(win_id) {
                Some(r) => r.size(),
                None => return,
            };
            let WinId(win_idx) = win_id;
            let hidpi_factor = self.hidpi_factor as f32;
            let snap_margin = (8.0 * hidpi_factor).round() as i32;
            let area_size = dim::Size::from([
                (self.area_size[0] * hidpi_factor) as i32,
                (self.area_size[1] * hidpi_factor) as i32,
            ]);

            fn check_snap_anchor<D: dim::Dir>(
                rect: RectI,
                display_size: dim::SizeI,
                area_size: dim::SizeI,
                snap_margin: i32,
            ) -> snapping::Anchor {
                let pos = rect.pos().dim::<D>();
                let is_snap_lower = pos == 0 + snap_margin;
                let is_snap_upper =
                    pos + display_size.dim::<D>() == area_size.dim::<D>() - snap_margin;
                match (is_snap_lower, is_snap_upper) {
                    (true, true) => snapping::Anchor::LowerAndUpperEdges,
                    (true, false) => snapping::Anchor::LowerEdge,
                    (false, true) => snapping::Anchor::UpperEdge,
                    (false, false) => snapping::Anchor::None,
                }
            }

            let anchor_x =
                check_snap_anchor::<dim::Horizontal>(rect, display_size, area_size, snap_margin);
            let anchor_y =
                check_snap_anchor::<dim::Vertical>(rect, display_size, area_size, snap_margin);

            let win = match self.window_states[win_idx as usize].as_mut() {
                Some(x) => x,
                None => unreachable!(),
            };
            win.anchor_x = anchor_x;
            win.anchor_y = anchor_y;

            // Round to device pixel.
            self.set_win_normal_rect_int(win_id, rect);
        }
    }

    pub fn current_dragging_win(&self) -> Option<(WinId, HitTest)> {
        self.maybe_dragging_window
            .as_ref()
            .map(|dragging| (dragging.win_id, dragging.dragging_hit_test))
    }

    pub fn win_drag_update(&mut self, offset: [f32; 2]) -> bool {
        let &DraggingState {
            win_id,
            dragging_hit_test,
            starting_rect,
            ..
        } = match self.maybe_dragging_window.as_ref() {
            Some(x) => x,
            None => return false,
        };
        let prev_rect = match self.win_display_rect_int(win_id) {
            Some(x) => x,
            None => {
                self.win_drag_end(true);
                return false;
            }
        };
        let hidpi_factor = self.hidpi_factor as f32;
        // Round the offset to device pixels:
        let dx = (offset[0] * hidpi_factor).round() as i32;
        let dy = (offset[1] * hidpi_factor).round() as i32;

        // Ensure the window being dragged is topmost.
        self.bring_to_top(win_id);

        let border_thickness = self.frame_metrics.border_thickness as f32;
        let title_bar_height = self.frame_metrics.title_bar_height as f32;

        let area_size = dim::SizeI {
            w: (self.area_size[0] * hidpi_factor) as i32,
            h: (self.area_size[1] * hidpi_factor) as i32,
        };

        let win = self.window_states[win_id.0 as usize]
            .as_ref()
            .unwrap_or_else(|| unreachable!());
        let min_w = ((border_thickness * 2.0 + win.min_size.w) * hidpi_factor).round() as i32;
        let min_h = ((border_thickness * 2.0 + title_bar_height + win.min_size.h) * hidpi_factor)
            .round() as i32;

        // TODO: Make these configurable:
        let snap_threshold = (12.0 * hidpi_factor).round() as i32;
        let snap_margin = (8.0 * hidpi_factor).round() as i32;

        let dragging_state = self
            .maybe_dragging_window
            .as_mut()
            .unwrap_or_else(|| unreachable!());

        fn snap_dimension<D: dim::Dir>(
            try_snap: impl Fn(i32) -> Option<i32>,
            dim_range: dim::DimRange<i32, D>,
            snap_candidates: &[(WinId, snapping::SnapSegment<D>)],
            last_snapped: &mut Option<u32>,
        ) -> Option<i32> {
            ({
                last_snapped.and_then(|last_snapped_idx| {
                    // Check the previously snapped window border.
                    let (_, seg) = snap_candidates[last_snapped_idx as usize];
                    if seg.dim_range().overlaps_with(dim_range) {
                        try_snap(seg.perpendicular_dim())
                    } else {
                        None
                    }
                })
            })
            .or_else(|| {
                // Try to find a window border to snap to.
                // TODO: Possible optimization if the candidates are sorted.
                let maybe_snap = snap_candidates
                    .iter()
                    .enumerate()
                    .find_map(|(i, (_, seg))| {
                        if seg.dim_range().overlaps_with(dim_range) {
                            try_snap(seg.perpendicular_dim()).map(|snap| (i, snap))
                        } else {
                            None
                        }
                    });
                if let Some((i, snap)) = maybe_snap {
                    *last_snapped = Some(i as u32);
                    Some(snap)
                } else {
                    *last_snapped = None;
                    None
                }
            })
        };

        fn calc_new_dimensions<D: dim::Dir>(
            dragging_hit_test: HitTest,
            starting_rect: RectI,
            prev_display_rect: RectI,
            delta: i32,
            win_min_size: i32,
            area_size: dim::SizeI,
            snap_margin: i32,
            snap_threshold: i32,
            snap_candidates: &[(WinId, snapping::SnapSegment<D::PerpendicularDir>)],
            last_snapped: &mut Option<u32>,
        ) -> (i32, i32) {
            let snap_move = |pos: i32, edge: i32| {
                if (pos - edge).abs() < snap_threshold {
                    Some(edge)
                } else {
                    None
                }
            };
            let snap_resize_upper = |lower_pos: i32, upper_pos: i32, edge: i32, min_dim: i32| {
                snap_move(upper_pos, edge).and_then(|edge| {
                    if (edge - lower_pos) < min_dim {
                        None
                    } else {
                        Some(edge)
                    }
                })
            };
            let snap_resize_lower = |lower_pos: i32, upper_pos: i32, edge: i32, min_dim: i32| {
                snap_move(lower_pos, edge).and_then(|edge| {
                    if (upper_pos - edge) < min_dim {
                        None
                    } else {
                        Some(edge)
                    }
                })
            };
            let dim_range = prev_display_rect.range::<D::PerpendicularDir>();
            let (new_pos, new_size);
            match dragging_hit_test.to_drag_action_1d::<D>() {
                WindowDragAction1D::None => {
                    new_pos = starting_rect.pos().dim::<D>();
                    new_size = starting_rect.size().dim::<D>();
                }
                WindowDragAction1D::MoveWindow => {
                    let target_pos = starting_rect.pos().dim::<D>() + delta;
                    let try_snap = |pos_to_snap: i32| snap_move(target_pos, pos_to_snap);

                    new_pos = {
                        // Try snapping to the lower and upper edges of area.
                        let maybe_snap = try_snap(0 + snap_margin).or_else(|| {
                            try_snap(
                                area_size.dim::<D>()
                                    - snap_margin
                                    - prev_display_rect.size().dim::<D>(),
                            )
                        });
                        if maybe_snap.is_some() {
                            *last_snapped = None;
                        }
                        maybe_snap
                    }
                    .or_else(|| snap_dimension(try_snap, dim_range, snap_candidates, last_snapped))
                    .unwrap_or_else(|| {
                        // Nothing to snap
                        target_pos
                    });
                    new_size = starting_rect.size().dim::<D>();
                }
                WindowDragAction1D::ResizeLower => {
                    let target_pos = starting_rect.pos().dim::<D>() + delta;
                    let try_snap = |pos_to_snap: i32| {
                        snap_resize_lower(
                            target_pos,
                            starting_rect.pos().dim::<D>() + starting_rect.size().dim::<D>(),
                            pos_to_snap,
                            win_min_size,
                        )
                    };

                    new_pos = ({
                        // Try snapping to lower edge of area.
                        let maybe_snap = try_snap(0 + snap_margin);
                        if maybe_snap.is_some() {
                            *last_snapped = None;
                        }
                        maybe_snap
                    })
                    .or_else(|| snap_dimension(try_snap, dim_range, snap_candidates, last_snapped))
                    .unwrap_or_else(|| {
                        // Nothing to snap.
                        starting_rect.pos().dim::<D>() + starting_rect.size().dim::<D>()
                            - (starting_rect.size().dim::<D>() - delta).max(win_min_size)
                    });
                    new_size =
                        starting_rect.size().dim::<D>() + starting_rect.pos().dim::<D>() - new_pos;
                }
                WindowDragAction1D::ResizeUpper => {
                    let target_pos =
                        starting_rect.pos().dim::<D>() + starting_rect.size().dim::<D>() + delta;
                    let try_snap = |pos_to_snap: i32| {
                        snap_resize_upper(
                            starting_rect.pos().dim::<D>(),
                            target_pos,
                            pos_to_snap,
                            win_min_size,
                        )
                    };

                    new_pos = starting_rect.pos().dim::<D>();
                    new_size = ({
                        // Try snapping to upper edge of area.
                        let maybe_snap = try_snap(area_size.dim::<D>() - snap_margin);
                        if maybe_snap.is_some() {
                            *last_snapped = None;
                        }
                        maybe_snap
                    })
                    .or_else(|| snap_dimension(try_snap, dim_range, snap_candidates, last_snapped))
                    .map(|pos| pos - starting_rect.pos().dim::<D>())
                    .unwrap_or_else(|| {
                        // Nothing to snap.
                        (starting_rect.size().dim::<D>() + delta).max(win_min_size)
                    });
                }
            }
            (new_pos, new_size)
        }

        // Calculate horizontal dimensions:=
        let (new_x, new_w) = calc_new_dimensions::<dim::Horizontal>(
            dragging_hit_test,
            starting_rect,
            prev_rect,
            dx,
            min_w,
            area_size,
            snap_margin,
            snap_threshold,
            &dragging_state.snap_candidates_x,
            &mut dragging_state.last_snapped_x,
        );
        // Calculate vertical dimensions:
        let (new_y, new_h) = calc_new_dimensions::<dim::Vertical>(
            dragging_hit_test,
            starting_rect,
            prev_rect,
            dy,
            min_h,
            area_size,
            snap_margin,
            snap_threshold,
            &dragging_state.snap_candidates_y,
            &mut dragging_state.last_snapped_y,
        );

        let new_rect = RectI {
            x: new_x,
            y: new_y,
            w: new_w,
            h: new_h,
        };
        self.set_win_normal_rect_int(win_id, new_rect);
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
    } else if x > w - border_thickness {
        WindowPartX::RightBorder
    } else {
        WindowPartX::Content
    };
    let window_part_y = if y <= border_thickness {
        WindowPartY::TopBorder
    } else if y > h - border_thickness {
        WindowPartY::BottomBorder
    } else if y <= border_thickness + title_bar_height {
        WindowPartY::TitleBar
    } else {
        WindowPartY::Content
    };

    let corner_leeway = border_thickness * 3;
    let (is_near_l, is_near_r) = if x <= corner_leeway {
        (true, false)
    } else if x > w - corner_leeway {
        (false, true)
    } else {
        (false, false)
    };
    let (is_near_t, is_near_b) = if y <= corner_leeway {
        (true, false)
    } else if y > h - corner_leeway {
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
