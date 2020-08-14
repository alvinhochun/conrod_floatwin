use crate::{empty_widget::EmptyWidget, util};
use layout::{FrameMetrics, WinId, WindowingState};
use window_frame::WindowFrame;

use conrod_core::{
    cursor,
    position::{self, Place},
    widget, widget_ids, Position, Positionable, Sizeable, Ui, UiCell, Widget, WidgetCommon,
    WidgetStyle,
};

pub mod layout;

mod debug;
mod window_frame;

#[derive(WidgetCommon)]
pub struct WindowingArea<'a> {
    #[conrod(common_builder)]
    pub common: widget::CommonBuilder,
    pub style: Style,
    pub windowing_state: &'a mut WindowingState,
    pub hidpi_factor: f64,
    pub enable_debug: bool,
}

pub struct State {
    ids: Ids,
    maybe_dragging_win: Option<bool>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, WidgetStyle)]
pub struct Style {}

pub struct WindowingContext<'a> {
    windowing_area_id: widget::Id,
    windowing_area_rect: conrod_core::Rect,
    windowing_state: &'a mut WindowingState,
    frame_metrics: FrameMetrics,
}

pub struct WindowSetter {
    window_frame_id: widget::Id,
    content_widget_id: widget::Id,
}

widget_ids! {
    struct Ids {
        capture_overlay,
        window_frames[],
        // window_titles[],
        window_contents[],
        debug,
    }
}

impl<'a> WindowingArea<'a> {
    pub fn new(windowing_state: &'a mut WindowingState, hidpi_factor: f64) -> Self {
        Self {
            common: widget::CommonBuilder::default(),
            style: Style::default(),
            windowing_state,
            hidpi_factor,
            enable_debug: false,
        }
    }

    pub fn with_debug(mut self, enabled: bool) -> Self {
        self.enable_debug = enabled;
        self
    }
}

impl<'a> Widget for WindowingArea<'a> {
    type State = State;
    type Style = Style;
    type Event = WindowingContext<'a>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
            maybe_dragging_win: None,
        }
    }

    fn style(&self) -> Self::Style {
        self.style.clone()
    }

    fn is_over(&self) -> widget::IsOverFn {
        // We want this widget to not capture mouse events. This does not
        // affect individual window frames as they still capture mouse events
        // on their own. Alt+Drag window movement is handled by an overlay
        // widget that captures mouse events so it is also not affected.
        |_, _, _| widget::IsOver::Bool(false)
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            id,
            state,
            rect,
            mut ui,
            ..
        } = args;
        let state: &mut conrod_core::widget::State<State> = state;
        let Self {
            windowing_state,
            hidpi_factor,
            enable_debug,
            ..
        } = self;

        // Snap the rect inward to the physical pixel grid if needed.
        let rect = if rect.dim() == ui.window_dim() {
            // The window dimensions are already aligned to the pixel grid.
            rect
        } else {
            // First, we need the coords relative to the bottom-left corner of
            // the window.
            let half_w = ui.win_w * 0.5;
            let half_h = ui.win_h * 0.5;
            let l = half_w + rect.left();
            let r = half_w + rect.right();
            let b = half_h + rect.bottom();
            let t = half_h + rect.top();

            // Then, we round the values inward.
            let sl = (l * hidpi_factor).ceil() / hidpi_factor;
            let sr = (r * hidpi_factor).floor() / hidpi_factor;
            let sb = (b * hidpi_factor).ceil() / hidpi_factor;
            let st = (t * hidpi_factor).floor() / hidpi_factor;

            // Finally, we change the rect back to be relative to the centre.
            position::Rect::from_corners([sl - half_w, sb - half_h], [sr - half_w, st - half_h])
        };

        let is_drag_move_window =
            ui.global_input().current.modifiers == conrod_core::input::ModifierKey::ALT;
        if is_drag_move_window {
            // Add an empty widget on top for mouse capturing.
            EmptyWidget::new()
                .graphics_for(id)
                .place_on_kid_area(false)
                .xy(rect.xy())
                .wh(rect.dim())
                .depth(position::Depth::MIN)
                .set(state.ids.capture_overlay, &mut ui);
        }

        if state.ids.window_frames.len() != windowing_state.win_count() {
            let target_len = windowing_state.win_count();
            state.update(|state| {
                state
                    .ids
                    .window_frames
                    .resize(target_len, &mut ui.widget_id_generator());
                state
                    .ids
                    .window_contents
                    .resize(target_len, &mut ui.widget_id_generator());
            });
        }

        windowing_state.set_dimensions([rect.w() as f32, rect.h() as f32], hidpi_factor);
        let frame_metrics = windowing_state.frame_metrics();

        let current_input = &ui.global_input().current;
        {
            for event in ui.global_input().events().ui() {
                match event {
                    conrod_core::event::Ui::Press(
                        Some(press_id),
                        conrod_core::event::Press {
                            button: conrod_core::event::Button::Mouse(_, pos),
                            ..
                        },
                    ) => {
                        let win_under_cursor =
                            if is_drag_move_window {
                                if *press_id == id {
                                    let pos = util::conrod_point_to_layout_pos(*pos, rect);
                                    windowing_state.win_hit_test(pos).map(|(win_id, _)| win_id)
                                } else {
                                    None
                                }
                            } else {
                                state.ids.window_frames.iter().enumerate().find_map(
                                    |(i, &frame_id)| {
                                        if frame_id == *press_id
                                            || ui.widget_graph().does_recursive_depth_edge_exist(
                                                frame_id, *press_id,
                                            )
                                        {
                                            Some(WinId(i as u32))
                                        } else {
                                            None
                                        }
                                    },
                                )
                            };
                        if let Some(win_id) = win_under_cursor {
                            windowing_state.bring_to_top(win_id);
                        }
                    }
                    conrod_core::event::Ui::Drag(Some(drag_id), drag)
                        if state.maybe_dragging_win != Some(false) =>
                    {
                        let is_self_event = || {
                            *drag_id == id || {
                                // Check whether the event widget id matches.
                                if state.maybe_dragging_win.is_some() {
                                    // If a window is being dragged, use it.
                                    windowing_state
                                        .current_dragging_win()
                                        .map(|(win_id, _)| win_id)
                                } else {
                                    // Otherwise, use the topmost window.
                                    windowing_state.topmost_win()
                                }
                                .map_or(false, |WinId(win)| {
                                    *drag_id == state.ids.window_frames[win as usize]
                                })
                            }
                        };
                        if drag.button == conrod_core::input::MouseButton::Left && is_self_event() {
                            let topmost_win_id = windowing_state
                                .topmost_win()
                                .unwrap_or_else(|| unreachable!());
                            let is_dragging_win = state.maybe_dragging_win.unwrap_or_else(|| {
                                let pos = util::conrod_point_to_layout_pos(drag.origin, rect);
                                let ht = windowing_state
                                    .specific_win_hit_test(topmost_win_id, pos)
                                    .map(|ht| {
                                        if is_drag_move_window {
                                            layout::HitTest::TitleBarOrDragArea
                                        } else {
                                            ht
                                        }
                                    });
                                eprintln!("drag start on {:?}", ht);
                                if let Some(ht) = ht {
                                    windowing_state.win_drag_start(topmost_win_id, ht)
                                } else {
                                    false
                                }
                            });
                            let new_is_dragging_win = if is_dragging_win {
                                let drag_delta_x = (drag.to[0] - drag.origin[0]) as f32;
                                let drag_delta_y = -(drag.to[1] - drag.origin[1]) as f32;
                                windowing_state.win_drag_update([drag_delta_x, drag_delta_y])
                            } else {
                                false
                            };
                            if state.maybe_dragging_win != Some(new_is_dragging_win) {
                                state.update(|state| {
                                    state.maybe_dragging_win = Some(new_is_dragging_win);
                                });
                            }
                        }
                    }
                    conrod_core::event::Ui::Release(
                        _,
                        conrod_core::event::Release {
                            button:
                                conrod_core::event::Button::Mouse(
                                    conrod_core::input::MouseButton::Left,
                                    _,
                                ),
                            ..
                        },
                    ) => {
                        if let Some(is_dragging_window) = state.maybe_dragging_win {
                            eprintln!("drag release");
                            if is_dragging_window {
                                windowing_state.win_drag_end(false);
                            }
                            state.update(|state| {
                                state.maybe_dragging_win = None;
                            });
                        }
                    }
                    conrod_core::event::Ui::DoubleClick(
                        Some(dblclick),
                        conrod_core::event::DoubleClick {
                            button: conrod_core::input::MouseButton::Left,
                            xy,
                            ..
                        },
                    ) => {
                        if let Some(topmost_win_id) = windowing_state
                            .topmost_win()
                            .filter(|win| state.ids.window_frames[win.0 as usize] == *dblclick)
                        {
                            let pos = util::conrod_point_to_layout_pos(*xy, rect);
                            let ht = windowing_state.specific_win_hit_test(topmost_win_id, pos);
                            if ht == Some(layout::HitTest::TitleBarOrDragArea) {
                                let is_collapsed = windowing_state.win_is_collapsed(topmost_win_id);
                                windowing_state.set_win_collapsed(topmost_win_id, !is_collapsed);
                            }
                        }
                    }
                    _ => {}
                }
            }
            if state.maybe_dragging_win != Some(true) {
                windowing_state.ensure_all_win_in_area();
            }
        }

        if let Some(cursor) = state
            .maybe_dragging_win
            .and_then(|is_dragging| {
                if is_dragging {
                    windowing_state.current_dragging_win().map(|(_, ht)| ht)
                } else {
                    None
                }
            })
            .or_else(|| {
                current_input
                    .widget_capturing_mouse
                    .and_then(|mouse_widget| {
                        // Hit test with the topmost window under the cursor.
                        let pos = util::conrod_point_to_layout_pos(current_input.mouse.xy, rect);
                        windowing_state
                            .win_hit_test_filtered(pos, |win_id| {
                                // We can skip those that are not capturing the
                                // cursor.
                                if is_drag_move_window {
                                    mouse_widget == id
                                } else {
                                    mouse_widget == state.ids.window_frames[win_id.0 as usize]
                                }
                            })
                            .map(|(win_id, ht)| match ht {
                                _ if is_drag_move_window => layout::HitTest::TitleBarOrDragArea,
                                layout::HitTest::TitleBarOrDragArea => ht,
                                _ if windowing_state.win_is_collapsed(win_id) => {
                                    // Can't resize collapsed windows.
                                    layout::HitTest::Content
                                }
                                _ => ht,
                            })
                    })
            })
            .and_then(|ht| match ht {
                layout::HitTest::Content => None,
                layout::HitTest::TitleBarOrDragArea => Some(cursor::MouseCursor::Grab),
                layout::HitTest::TopBorder | layout::HitTest::BottomBorder => {
                    Some(cursor::MouseCursor::ResizeVertical)
                }
                layout::HitTest::LeftBorder | layout::HitTest::RightBorder => {
                    Some(cursor::MouseCursor::ResizeHorizontal)
                }
                layout::HitTest::TopLeftCorner | layout::HitTest::BottomRightCorner => {
                    Some(cursor::MouseCursor::ResizeTopLeftBottomRight)
                }
                layout::HitTest::TopRightCorner | layout::HitTest::BottomLeftCorner => {
                    Some(cursor::MouseCursor::ResizeTopRightBottomLeft)
                }
            })
        {
            ui.set_mouse_cursor(cursor);
        }

        windowing_state.set_all_needed(false);

        if enable_debug {
            if let Some(win_id) = windowing_state.topmost_win() {
                debug::DebugWidget::new(&*windowing_state, win_id, hidpi_factor)
                    .graphics_for(id)
                    .place_on_kid_area(false)
                    .xy(rect.xy())
                    .wh(rect.dim())
                    .depth(std::f32::MIN)
                    .set(state.ids.debug, &mut ui);
            }
        }
        WindowingContext {
            windowing_area_id: id,
            windowing_area_rect: rect,
            windowing_state,
            frame_metrics,
        }
    }

    fn default_x_position(&self, _ui: &Ui) -> Position {
        Position::Relative(position::Relative::Place(Place::Middle), None)
    }

    fn default_y_position(&self, _ui: &Ui) -> Position {
        Position::Relative(position::Relative::Place(Place::Middle), None)
    }
}

impl<'a> WindowingContext<'a> {
    pub fn make_window<'c>(
        &mut self,
        title: &'c str,
        initial_position: Option<[f32; 2]>,
        initial_size: [f32; 2],
        win_id: WinId,
        ui: &mut UiCell,
    ) -> Option<WindowSetter> {
        self.windowing_state
            .ensure_init(win_id, || layout::WindowInitialState {
                client_size: initial_size,
                position: initial_position,
                is_collapsed: false,
            });
        self.windowing_state.set_needed(win_id, true);
        let state: &State = match ui
            .widget_graph()
            .widget(self.windowing_area_id)
            .and_then(|container| container.unique_widget_state::<WindowingArea>())
            .map(|&conrod_core::graph::UniqueWidgetState { ref state, .. }| state)
        {
            Some(state) => state,
            None => return None,
        };
        let win_idx = win_id.0 as usize;
        let window_frame_id = state.ids.window_frames[win_idx];
        let content_widget_id = state.ids.window_contents[win_idx];
        let window_depth = -(self.windowing_state.win_z_order(win_id) as position::Depth);
        let window_is_collapsed = self.windowing_state.win_is_collapsed(win_id);
        let conrod_window_rect = util::win_rect_to_conrod_rect(
            self.windowing_state.win_display_rect_f64(win_id)?,
            self.windowing_area_rect,
        );
        let is_focused = self.windowing_state.topmost_win() == Some(win_id);
        WindowFrame::new(self.frame_metrics)
            .title(title)
            .is_focused(is_focused)
            .frame_color(conrod_core::color::rgba(0.75, 0.75, 0.75, 1.0))
            .title_bar_color(conrod_core::color::LIGHT_GRAY)
            .xy(conrod_window_rect.xy())
            .wh(conrod_window_rect.dim())
            .depth(window_depth)
            .parent(self.windowing_area_id)
            .set(window_frame_id, ui);
        if window_is_collapsed {
            return None;
        }
        Some(WindowSetter {
            window_frame_id,
            content_widget_id,
        })
    }
}

impl<'a> Drop for WindowingContext<'a> {
    fn drop(&mut self) {
        self.windowing_state.sweep_unneeded();
    }
}

impl WindowSetter {
    pub fn set<W>(self, widget: W, ui: &mut UiCell) -> (widget::Id, W::Event)
    where
        W: Widget,
    {
        let event = widget
            .kid_area_wh_of(self.window_frame_id)
            .parent(self.window_frame_id)
            .set(self.content_widget_id, ui);
        (self.content_widget_id, event)
    }
}
