pub use dim::{Rect, RectF, RectI};

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

struct DraggingState {
    win_id: WinId,
    dragging_hit_test: HitTest,
    starting_rect: RectI,
    snap_candidates_x: Vec<(WinId, snapping::SnapSegmentV)>,
    last_snapped_x: Option<u32>,
    snap_candidates_y: Vec<(WinId, snapping::SnapSegmentH)>,
    last_snapped_y: Option<u32>,
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
            let rect = RectF { x, y, w, h };
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
        let (win_display_w, win_display_h) = {
            match self.win_display_rect_int(win_id) {
                Some(r) => (r.w, r.h),
                None => return false,
            }
        };

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
        let x_iter = base_iter.clone().map(|(win_id, rect)| {
            let dim_range = snapping::DimRangeV::new(rect.y, rect.y + rect.h);
            (win_id, rect, dim_range)
        });
        let y_iter = base_iter.map(|(win_id, rect)| {
            let dim_range = snapping::DimRangeH::new(rect.x, rect.x + rect.w);
            (win_id, rect, dim_range)
        });
        let snap_candidates_x;
        match dragging_hit_test {
            HitTest::Content | HitTest::TopBorder | HitTest::BottomBorder => {
                // Nothing to snap in the x direction.
                snap_candidates_x = Vec::new();
            }
            HitTest::TitleBarOrDragArea => {
                // Gather a list of all left and right borders.
                snap_candidates_x = x_iter
                    .flat_map(|(win_id, rect, dim_range)| {
                        std::iter::once((
                            win_id,
                            snapping::SnapSegment::new(
                                rect.x - snap_margin - win_display_w,
                                dim_range,
                            ),
                        ))
                        .chain(std::iter::once((
                            win_id,
                            snapping::SnapSegment::new(rect.x + rect.w + snap_margin, dim_range),
                        )))
                    })
                    .collect();
            }
            HitTest::LeftBorder | HitTest::TopLeftCorner | HitTest::BottomLeftCorner => {
                // Gather a list of all right borders.
                snap_candidates_x = x_iter
                    .map(|(win_id, rect, dim_range)| {
                        (
                            win_id,
                            snapping::SnapSegment::new(rect.x + rect.w + snap_margin, dim_range),
                        )
                    })
                    .collect();
            }
            HitTest::RightBorder | HitTest::TopRightCorner | HitTest::BottomRightCorner => {
                // Gather a list of all left borders.
                snap_candidates_x = x_iter
                    .map(|(win_id, rect, dim_range)| {
                        (
                            win_id,
                            snapping::SnapSegment::new(rect.x - snap_margin, dim_range),
                        )
                    })
                    .collect();
            }
        }
        let snap_candidates_y;
        match dragging_hit_test {
            HitTest::Content | HitTest::LeftBorder | HitTest::RightBorder => {
                // Nothing to snap in the y direction.
                snap_candidates_y = Vec::new();
            }
            HitTest::TitleBarOrDragArea => {
                // Gather a list of all top and bottom borders.
                snap_candidates_y = y_iter
                    .flat_map(|(win_id, rect, dim_range)| {
                        std::iter::once((
                            win_id,
                            snapping::SnapSegment::new(
                                rect.y - snap_margin - win_display_h,
                                dim_range,
                            ),
                        ))
                        .chain(std::iter::once((
                            win_id,
                            snapping::SnapSegment::new(rect.y + rect.h + snap_margin, dim_range),
                        )))
                    })
                    .collect();
            }
            HitTest::TopBorder | HitTest::TopLeftCorner | HitTest::TopRightCorner => {
                // Gather a list of all bottom borders.
                snap_candidates_y = y_iter
                    .map(|(win_id, rect, dim_range)| {
                        (
                            win_id,
                            snapping::SnapSegment::new(rect.y + rect.h + snap_margin, dim_range),
                        )
                    })
                    .collect();
            }
            HitTest::BottomBorder | HitTest::BottomLeftCorner | HitTest::BottomRightCorner => {
                // Gather a list of all top borders.
                snap_candidates_y = y_iter
                    .map(|(win_id, rect, dim_range)| {
                        (
                            win_id,
                            snapping::SnapSegment::new(rect.y - snap_margin, dim_range),
                        )
                    })
                    .collect();
            }
        }
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
            // Round to device pixel.
            if let Some(rect) = self.win_normal_rect(win_id) {
                self.set_win_normal_rect(win_id, rect);
            }
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

        let area_w = (self.area_size[0] * hidpi_factor) as i32;
        let area_h = (self.area_size[1] * hidpi_factor) as i32;
        let (win_display_w, win_display_h) = (prev_rect.w, prev_rect.h);

        // TODO: Make these configurable:
        let min_w = ((border_thickness * 2.0 + 50.0) * hidpi_factor).round() as i32;
        let min_h =
            ((border_thickness * 2.0 + title_bar_height + 16.0) * hidpi_factor).round() as i32;
        let snap_threshold = (12.0 * hidpi_factor).round() as i32;
        let snap_margin = (8.0 * hidpi_factor).round() as i32;

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
        // Calculate horizontal dimensions:
        let y_dim_range = snapping::DimRangeV::new(prev_rect.y, prev_rect.y + prev_rect.h);
        let (new_x, new_w);
        match dragging_hit_test {
            HitTest::Content | HitTest::TopBorder | HitTest::BottomBorder => {
                new_x = starting_rect.x;
                new_w = starting_rect.w;
            }
            HitTest::TitleBarOrDragArea => {
                let target_x = starting_rect.x + dx;
                let try_snap = |x_to_snap: i32| snap_move(target_x, x_to_snap);

                new_x = {
                    // Try snapping to the left and right edges of area.
                    let maybe_snap = try_snap(0 + snap_margin)
                        .or_else(|| try_snap(area_w - snap_margin - win_display_w));
                    if maybe_snap.is_some() {
                        dragging_state.last_snapped_x = None;
                    }
                    maybe_snap
                }
                .or_else(|| {
                    snap_dimension(
                        try_snap,
                        y_dim_range,
                        &dragging_state.snap_candidates_x,
                        &mut dragging_state.last_snapped_x,
                    )
                })
                .unwrap_or_else(|| {
                    // Nothing to snap
                    target_x
                });
                new_w = starting_rect.w;
            }
            HitTest::LeftBorder | HitTest::TopLeftCorner | HitTest::BottomLeftCorner => {
                let target_x = starting_rect.x + dx;
                let try_snap = |x_to_snap: i32| {
                    snap_resize_lower(
                        target_x,
                        starting_rect.x + starting_rect.w,
                        x_to_snap,
                        min_w,
                    )
                };

                new_x = ({
                    // Try snapping to left edge of area.
                    let maybe_snap = try_snap(0 + snap_margin);
                    if maybe_snap.is_some() {
                        dragging_state.last_snapped_x = None;
                    }
                    maybe_snap
                })
                .or_else(|| {
                    snap_dimension(
                        try_snap,
                        y_dim_range,
                        &dragging_state.snap_candidates_x,
                        &mut dragging_state.last_snapped_x,
                    )
                })
                .unwrap_or_else(|| {
                    // Nothing to snap.
                    starting_rect.x + starting_rect.w - (starting_rect.w - dx).max(min_w)
                });
                new_w = starting_rect.w + starting_rect.x - new_x;
            }
            HitTest::RightBorder | HitTest::TopRightCorner | HitTest::BottomRightCorner => {
                let target_x = starting_rect.x + starting_rect.w + dx;
                let try_snap =
                    |x_to_snap: i32| snap_resize_upper(starting_rect.x, target_x, x_to_snap, min_w);

                new_x = starting_rect.x;
                new_w = ({
                    // Try snapping to right edge of area.
                    let maybe_snap = try_snap(area_w - snap_margin);
                    if maybe_snap.is_some() {
                        dragging_state.last_snapped_x = None;
                    }
                    maybe_snap
                })
                .or_else(|| {
                    snap_dimension(
                        try_snap,
                        y_dim_range,
                        &dragging_state.snap_candidates_x,
                        &mut dragging_state.last_snapped_x,
                    )
                })
                .map(|x| x - starting_rect.x)
                .unwrap_or_else(|| {
                    // Nothing to snap.
                    (starting_rect.w + dx).max(min_w)
                });
            }
        }

        // Calculate vertical dimensions:
        let x_dim_range = snapping::DimRangeH::new(prev_rect.x, prev_rect.x + prev_rect.h);
        let (new_y, new_h);
        match dragging_hit_test {
            HitTest::Content | HitTest::LeftBorder | HitTest::RightBorder => {
                new_y = starting_rect.y;
                new_h = starting_rect.h;
            }
            HitTest::TitleBarOrDragArea => {
                let target_y = starting_rect.y + dy;
                let try_snap = |y_to_snap: i32| snap_move(target_y, y_to_snap);

                new_y = {
                    // Try snapping to the top and bottom edges of area.
                    let maybe_snap = try_snap(0 + snap_margin)
                        .or_else(|| try_snap(area_h - snap_margin - win_display_h));
                    if maybe_snap.is_some() {
                        dragging_state.last_snapped_y = None;
                    }
                    maybe_snap
                }
                .or_else(|| {
                    snap_dimension(
                        try_snap,
                        x_dim_range,
                        &dragging_state.snap_candidates_y,
                        &mut dragging_state.last_snapped_y,
                    )
                })
                .unwrap_or_else(|| {
                    // Nothing to snap
                    target_y
                });
                new_h = starting_rect.h;
            }
            HitTest::TopBorder | HitTest::TopLeftCorner | HitTest::TopRightCorner => {
                let target_y = starting_rect.y + dy;
                let try_snap = |y_to_snap: i32| {
                    snap_resize_lower(
                        target_y,
                        starting_rect.y + starting_rect.h,
                        y_to_snap,
                        min_h,
                    )
                };

                new_y = ({
                    // Try snapping to top edge of area.
                    let maybe_snap = try_snap(0 + snap_margin);
                    if maybe_snap.is_some() {
                        dragging_state.last_snapped_y = None;
                    }
                    maybe_snap
                })
                .or_else(|| {
                    snap_dimension(
                        try_snap,
                        x_dim_range,
                        &dragging_state.snap_candidates_y,
                        &mut dragging_state.last_snapped_y,
                    )
                })
                .unwrap_or_else(|| {
                    // Nothing to snap.
                    starting_rect.y + starting_rect.h - (starting_rect.h - dy).max(min_h)
                });
                new_h = starting_rect.h + starting_rect.y - new_y;
            }
            HitTest::BottomBorder | HitTest::BottomLeftCorner | HitTest::BottomRightCorner => {
                let target_y = starting_rect.y + starting_rect.h + dy;
                let try_snap =
                    |y_to_snap: i32| snap_resize_upper(starting_rect.y, target_y, y_to_snap, min_h);

                new_y = starting_rect.y;
                new_h = ({
                    // Try snapping to bottom edge of area.
                    let maybe_snap = try_snap(area_h - snap_margin);
                    if maybe_snap.is_some() {
                        dragging_state.last_snapped_y = None;
                    }
                    maybe_snap
                })
                .or_else(|| {
                    snap_dimension(
                        try_snap,
                        x_dim_range,
                        &dragging_state.snap_candidates_y,
                        &mut dragging_state.last_snapped_y,
                    )
                })
                .map(|y| y - starting_rect.y)
                .unwrap_or_else(|| {
                    // Nothing to snap.
                    (starting_rect.h + dy).max(min_h)
                });
            }
        }

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
