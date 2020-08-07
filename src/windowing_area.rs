use crate::empty_widget::EmptyWidget;
use layout::{WinId, WindowingState};
use window_frame::WindowFrame;

use conrod_core::{
    cursor,
    position::{self, Place},
    widget, widget_ids, Colorable, Position, Positionable, Sizeable, Ui, UiCell, Widget,
    WidgetCommon, WidgetStyle,
};

pub mod layout;

mod window_frame;

#[derive(WidgetCommon)]
pub struct WindowingArea<'a> {
    #[conrod(common_builder)]
    pub common: widget::CommonBuilder,
    pub style: Style,
    pub windowing_state: &'a mut WindowingState,
}

pub struct State {
    ids: Ids,
    maybe_drag_start_tuple: Option<(Option<layout::HitTest>, layout::Rect)>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, WidgetStyle)]
pub struct Style {}

pub struct WindowingContext<'a> {
    windowing_area_id: widget::Id,
    windowing_area_rect: conrod_core::Rect,
    windowing_state: &'a mut WindowingState,
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
    }
}

impl<'a> WindowingArea<'a> {
    pub fn new(windowing_state: &'a mut WindowingState) -> Self {
        Self {
            common: widget::CommonBuilder::default(),
            style: Style::default(),
            windowing_state,
        }
    }
}

impl<'a> Widget for WindowingArea<'a> {
    type State = State;
    type Style = Style;
    type Event = WindowingContext<'a>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
            maybe_drag_start_tuple: None,
        }
    }

    fn style(&self) -> Self::Style {
        self.style.clone()
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
            style,
            windowing_state,
            ..
        } = self;

        let is_drag_move_window =
            ui.global_input().current.modifiers == conrod_core::input::ModifierKey::ALT;
        if is_drag_move_window {
            // Add an empty widget on top for mouse capturing.
            EmptyWidget::new()
                .middle_of(id)
                .graphics_for(id)
                .place_on_kid_area(false)
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

        let current_input = &ui.global_input().current;
        {
            let mut maybe_drag_start_tuple = state.maybe_drag_start_tuple;
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
                                    // Find the window under the cursor.
                                    windowing_state
                                        .bottom_to_top_list
                                        .iter()
                                        .rev()
                                        .copied()
                                        .find(|&win| {
                                            let win_rect =
                                                &windowing_state.window_rects[win as usize];
                                            let x = (pos[0] - rect.left()) as f32 - win_rect.x;
                                            let y = (rect.top() - pos[1]) as f32 - win_rect.y;
                                            let w = win_rect.w;
                                            let h = win_rect.h;
                                            layout::window_hit_test([w, h], [x, y]).is_some()
                                        })
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
                                            Some(i as u32)
                                        } else {
                                            None
                                        }
                                    },
                                )
                            };
                        if let Some(win_id) = win_under_cursor {
                            windowing_state.bring_to_top(WinId(win_id));
                        }
                    }
                    conrod_core::event::Ui::Drag(Some(drag_id), drag) => {
                        let is_self_event = || {
                            *drag_id == id
                                || windowing_state.bottom_to_top_list.last().map_or(
                                    false,
                                    |&top_win| {
                                        *drag_id == state.ids.window_frames[top_win as usize]
                                    },
                                )
                        };
                        if drag.button == conrod_core::input::MouseButton::Left && is_self_event() {
                            let topmost_win_idx = *windowing_state
                                .bottom_to_top_list
                                .last()
                                .unwrap_or_else(|| unreachable!())
                                as usize;
                            let (drag_start_hit_test, drag_start_rect) = maybe_drag_start_tuple
                                .unwrap_or_else(|| {
                                    let win_rect = windowing_state.window_rects[topmost_win_idx];
                                    let x = (drag.origin[0] - rect.left()) as f32 - win_rect.x;
                                    let y = (rect.top() - drag.origin[1]) as f32 - win_rect.y;
                                    let w = win_rect.w;
                                    let h = win_rect.h;
                                    let ht = layout::window_hit_test([w, h], [x, y]).map(|ht| {
                                        if is_drag_move_window {
                                            layout::HitTest::TitleBarOrDragArea
                                        } else {
                                            ht
                                        }
                                    });
                                    eprintln!("drag start on {:?}", ht);
                                    (ht, win_rect)
                                });
                            // TODO: Make these configurable:
                            let min_w = 2.0 * 2.0 + 50.0;
                            let min_h = 2.0 * 2.0 + 24.0 + 16.0;
                            let drag_delta_x = (drag.to[0] - drag.origin[0]) as f32;
                            let drag_delta_y = -(drag.to[1] - drag.origin[1]) as f32;
                            let new_rect = match drag_start_hit_test {
                                Some(layout::HitTest::TitleBarOrDragArea) => {
                                    let new_x = drag_start_rect.x + drag_delta_x;
                                    let new_y = drag_start_rect.y + drag_delta_y;
                                    layout::Rect {
                                        x: new_x,
                                        y: new_y,
                                        ..drag_start_rect
                                    }
                                }
                                Some(layout::HitTest::TopBorder) => {
                                    let new_h = (drag_start_rect.h - drag_delta_y).max(min_h);
                                    let new_y = drag_start_rect.y + (drag_start_rect.h - new_h);
                                    layout::Rect {
                                        y: new_y,
                                        h: new_h,
                                        ..drag_start_rect
                                    }
                                }
                                Some(layout::HitTest::BottomBorder) => {
                                    let new_h = (drag_start_rect.h + drag_delta_y).max(min_h);
                                    layout::Rect {
                                        h: new_h,
                                        ..drag_start_rect
                                    }
                                }
                                Some(layout::HitTest::LeftBorder) => {
                                    let new_w = (drag_start_rect.w - drag_delta_x).max(min_w);
                                    let new_x = drag_start_rect.x + (drag_start_rect.w - new_w);
                                    layout::Rect {
                                        x: new_x,
                                        w: new_w,
                                        ..drag_start_rect
                                    }
                                }
                                Some(layout::HitTest::RightBorder) => {
                                    let new_w = (drag_start_rect.w + drag_delta_x).max(min_w);
                                    layout::Rect {
                                        w: new_w,
                                        ..drag_start_rect
                                    }
                                }
                                Some(layout::HitTest::TopLeftCorner) => {
                                    let new_w = (drag_start_rect.w - drag_delta_x).max(min_w);
                                    let new_h = (drag_start_rect.h - drag_delta_y).max(min_h);
                                    let new_x = drag_start_rect.x + (drag_start_rect.w - new_w);
                                    let new_y = drag_start_rect.y + (drag_start_rect.h - new_h);
                                    layout::Rect {
                                        x: new_x,
                                        y: new_y,
                                        w: new_w,
                                        h: new_h,
                                    }
                                }
                                Some(layout::HitTest::TopRightCorner) => {
                                    let new_h = (drag_start_rect.h - drag_delta_y).max(min_h);
                                    let new_y = drag_start_rect.y + (drag_start_rect.h - new_h);
                                    let new_w = (drag_start_rect.w + drag_delta_x).max(min_w);
                                    layout::Rect {
                                        y: new_y,
                                        w: new_w,
                                        h: new_h,
                                        ..drag_start_rect
                                    }
                                }
                                Some(layout::HitTest::BottomLeftCorner) => {
                                    let new_w = (drag_start_rect.w - drag_delta_x).max(min_w);
                                    let new_x = drag_start_rect.x + (drag_start_rect.w - new_w);
                                    let new_h = (drag_start_rect.h + drag_delta_y).max(min_h);
                                    layout::Rect {
                                        x: new_x,
                                        w: new_w,
                                        h: new_h,
                                        ..drag_start_rect
                                    }
                                }
                                Some(layout::HitTest::BottomRightCorner) => {
                                    let new_w = (drag_start_rect.w + drag_delta_x).max(min_w);
                                    let new_h = (drag_start_rect.h + drag_delta_y).max(min_h);
                                    layout::Rect {
                                        w: new_w,
                                        h: new_h,
                                        ..drag_start_rect
                                    }
                                }
                                _ => drag_start_rect,
                            };
                            maybe_drag_start_tuple = Some((drag_start_hit_test, drag_start_rect));
                            state.update(|state| {
                                state.maybe_drag_start_tuple = maybe_drag_start_tuple;
                            });
                            windowing_state.window_rects[topmost_win_idx] = new_rect;
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
                        if maybe_drag_start_tuple.is_some() {
                            eprintln!("drag release");
                            maybe_drag_start_tuple = None;
                            state.update(|state| {
                                state.maybe_drag_start_tuple = maybe_drag_start_tuple;
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Some(cursor) = state
            .maybe_drag_start_tuple
            .and_then(|(ht, _)| ht)
            .or_else(|| {
                current_input
                    .widget_capturing_mouse
                    .and_then(|mouse_widget| {
                        // Hit test with the topmost window under the cursor.
                        windowing_state
                            .bottom_to_top_list
                            .iter()
                            .rev()
                            .find_map(|&win| {
                                if mouse_widget == id
                                    || mouse_widget == state.ids.window_frames[win as usize]
                                {
                                    let pos = current_input.mouse.xy;
                                    let win_rect = &windowing_state.window_rects[win as usize];
                                    let x = (pos[0] - rect.left()) as f32 - win_rect.x;
                                    let y = (rect.top() - pos[1]) as f32 - win_rect.y;
                                    let w = win_rect.w;
                                    let h = win_rect.h;
                                    layout::window_hit_test([w, h], [x, y]).map(|ht| {
                                        if is_drag_move_window {
                                            layout::HitTest::TitleBarOrDragArea
                                        } else {
                                            ht
                                        }
                                    })
                                } else {
                                    None
                                }
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
        WindowingContext {
            windowing_area_id: id,
            windowing_area_rect: rect,
            windowing_state,
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
        &self,
        title: &'c str,
        win_id: WinId,
        ui: &mut UiCell,
    ) -> Option<WindowSetter> {
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
        let window_rect = self.windowing_state.window_rects[win_idx];
        let window_frame_id = state.ids.window_frames[win_idx];
        let content_widget_id = state.ids.window_contents[win_idx];
        let window_depth = -(self.windowing_state.window_z_orders[win_idx] as position::Depth);
        let [left, top] = self.windowing_area_rect.top_left();
        let conrod_window_rect = conrod_core::Rect::from_corners(
            [left + window_rect.x as f64, top - window_rect.y as f64],
            [
                left + window_rect.x as f64 + window_rect.w as f64,
                top - window_rect.y as f64 - window_rect.h as f64,
            ],
        );
        WindowFrame::new()
            .title(title)
            .color(conrod_core::color::LIGHT_GREY)
            .xy(conrod_window_rect.xy())
            .wh(conrod_window_rect.dim())
            .depth(window_depth)
            .parent(self.windowing_area_id)
            .set(window_frame_id, ui);
        Some(WindowSetter {
            window_frame_id,
            content_widget_id,
        })
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
