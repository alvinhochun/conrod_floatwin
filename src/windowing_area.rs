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
    hidpi_factor: f64,
}

#[derive(Clone, Debug)]
pub struct WindowBuilder<'a> {
    pub title: &'a str,
    pub initial_position: Option<[f32; 2]>,
    pub initial_size: Option<[f32; 2]>,
    pub min_size: Option<[f32; 2]>,
    pub is_hidden: bool,
    pub is_collapsible: bool,
    pub is_closable: bool,
    pub is_collapsed: Option<bool>,
    _private: (),
}

pub struct WindowEvent {
    pub collapse_clicked: widget::button::TimesClicked,
    pub close_clicked: widget::button::TimesClicked,
    pub title_bar_double_click_count: u32,
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

        // Remove the windows that weren't used in the last iteration.
        windowing_state.sweep_unneeded();

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
                            if is_dragging_window {
                                windowing_state.win_drag_end(false);
                            }
                            state.update(|state| {
                                state.maybe_dragging_win = None;
                            });
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
            hidpi_factor,
        }
    }

    fn default_x_position(&self, _ui: &Ui) -> Position {
        Position::Relative(position::Relative::Place(Place::Middle), None)
    }

    fn default_y_position(&self, _ui: &Ui) -> Position {
        Position::Relative(position::Relative::Place(Place::Middle), None)
    }
}

impl<'a> WindowBuilder<'a> {
    pub fn new() -> Self {
        Self {
            title: "",
            initial_position: None,
            initial_size: None,
            min_size: None,
            is_hidden: false,
            is_collapsible: true,
            is_closable: false,
            is_collapsed: None,
            _private: (),
        }
    }

    pub fn title(self, title: &'a str) -> Self {
        Self { title, ..self }
    }

    pub fn initial_position(self, initial_position: [f32; 2]) -> Self {
        Self {
            initial_position: Some(initial_position),
            ..self
        }
    }

    pub fn initial_size(self, initial_size: [f32; 2]) -> Self {
        Self {
            initial_size: Some(initial_size),
            ..self
        }
    }

    pub fn min_size(self, min_size: [f32; 2]) -> Self {
        Self {
            min_size: Some(min_size),
            ..self
        }
    }

    pub fn is_hidden(self, is_hidden: bool) -> Self {
        Self { is_hidden, ..self }
    }

    pub fn is_collapsible(self, is_collapsible: bool) -> Self {
        Self {
            is_collapsible,
            ..self
        }
    }

    /// Sets whether this window should have a close button on its frame. Note
    /// that the close button does nothing by default and you will need to
    /// handle the event yourself by using the `WindowEvent` data returned by
    /// `WindowingContext::make_window`.
    pub fn is_closable(self, is_closable: bool) -> Self {
        Self {
            is_closable,
            ..self
        }
    }

    /// Sets whether the window is collapsed. Note that if the collapsed status
    /// has not been set explicitly, the `WindowingContext` will automatically
    /// toggle the collapsed state when the collapse button is pressed or the
    /// title bar is double-clicked. If you require explicitly setting the
    /// collapse state of the window, you should handle these events yourself
    /// by using the `WindowEvent` data returned by `WindowingContext::make_window`.
    pub fn collapse(self, is_collapsed: bool) -> Self {
        Self {
            is_collapsed: Some(is_collapsed),
            ..self
        }
    }
}

impl<'a> WindowingContext<'a> {
    pub fn make_window<'c>(
        &mut self,
        builder: WindowBuilder,
        win_id: WinId,
        ui: &mut UiCell,
    ) -> (WindowEvent, Option<WindowSetter>) {
        self.windowing_state
            .ensure_init(win_id, || layout::WindowInitialState {
                client_size: builder.initial_size.unwrap_or_else(|| [200.0, 200.0]),
                position: builder.initial_position,
                min_size: builder.min_size,
                is_collapsed: false,
            });
        self.windowing_state.set_needed(win_id, true);
        if let Some(min_size) = builder.min_size {
            self.windowing_state.set_win_min_size(win_id, min_size);
        }
        if builder.is_collapsible {
            if let Some(is_collapsed) = builder.is_collapsed {
                self.windowing_state.set_win_collapsed(win_id, is_collapsed);
            }
        } else {
            self.windowing_state.set_win_collapsed(win_id, false);
        }
        self.windowing_state
            .set_win_hidden(win_id, builder.is_hidden);
        if builder.is_hidden {
            return (
                WindowEvent {
                    collapse_clicked: widget::button::TimesClicked(0),
                    close_clicked: widget::button::TimesClicked(0),
                    title_bar_double_click_count: 0,
                },
                None,
            );
        }

        let state: &State = match ui
            .widget_graph()
            .widget(self.windowing_area_id)
            .and_then(|container| container.unique_widget_state::<WindowingArea>())
            .map(|&conrod_core::graph::UniqueWidgetState { ref state, .. }| state)
        {
            Some(state) => state,
            None => {
                if cfg!(debug_assertions) {
                    panic!("Expected to get the widget state of `WindowingArea` without fail");
                }
                return (
                    WindowEvent {
                        collapse_clicked: widget::button::TimesClicked(0),
                        close_clicked: widget::button::TimesClicked(0),
                        title_bar_double_click_count: 0,
                    },
                    None,
                );
            }
        };
        let win_idx = win_id.0 as usize;
        let window_frame_id = state.ids.window_frames[win_idx];
        let content_widget_id = state.ids.window_contents[win_idx];
        let window_depth = -(self.windowing_state.win_z_order(win_id) as position::Depth);
        let window_is_collapsed = self.windowing_state.win_is_collapsed(win_id);
        let conrod_window_rect = util::win_rect_to_conrod_rect(
            self.windowing_state
                .win_display_rect_f64(win_id)
                .expect("Window must have already been initialized"),
            self.windowing_area_rect,
        );
        let is_focused = self.windowing_state.topmost_win() == Some(win_id);
        let event = WindowFrame::new(self.frame_metrics, self.hidpi_factor)
            .title(builder.title)
            .is_focused(is_focused)
            .is_collapsed(window_is_collapsed)
            .is_collapsible(builder.is_collapsible)
            .is_closable(builder.is_closable)
            .frame_color(conrod_core::color::rgba(0.75, 0.75, 0.75, 1.0))
            .title_bar_color(conrod_core::color::LIGHT_GRAY)
            .xy(conrod_window_rect.xy())
            .wh(conrod_window_rect.dim())
            .depth(window_depth)
            .parent(self.windowing_area_id)
            .set(window_frame_id, ui);

        let title_bar_double_click_count = ui
            .global_input()
            .events()
            .ui()
            .map(|event| match event {
                conrod_core::event::Ui::DoubleClick(
                    Some(dblclick),
                    conrod_core::event::DoubleClick {
                        button: conrod_core::input::MouseButton::Left,
                        xy,
                        ..
                    },
                ) if *dblclick == window_frame_id => {
                    let pos = util::conrod_point_to_layout_pos(*xy, self.windowing_area_rect);
                    let ht = self.windowing_state.specific_win_hit_test(win_id, pos);
                    ht == Some(layout::HitTest::TitleBarOrDragArea)
                }
                _ => false,
            })
            .filter(|&x| x)
            .count() as u32;
        // Toggle the collapse state if the collapse button was pressed or the
        // title bar was double-clicked, but only if the caller has not
        // explicitly set the collapse state.
        if builder.is_collapsible
            && builder.is_collapsed.is_none()
            && (event.collapse_clicked.0 as u32 + title_bar_double_click_count) % 2 == 1
        {
            self.windowing_state
                .set_win_collapsed(win_id, !window_is_collapsed);
            // Since we are toggling the collapse state after the WindowFrame
            // has already been set to the UI, the new collapse state will only
            // be reflected on the next update. We explicitly ask the UI to
            // redraw to make sure the next update will happen in case nothing
            // else has changed in the rest of the UI during this update.
            ui.needs_redraw();
        }
        let event = WindowEvent {
            collapse_clicked: event.collapse_clicked,
            close_clicked: event.close_clicked,
            title_bar_double_click_count,
        };
        if window_is_collapsed {
            (event, None)
        } else {
            (
                event,
                Some(WindowSetter {
                    window_frame_id,
                    content_widget_id,
                }),
            )
        }
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
