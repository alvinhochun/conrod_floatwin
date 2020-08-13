use super::layout::{WinId, WindowingState};
use crate::util;
use conrod_core::{
    position, widget, widget_ids, Colorable, Position, Positionable, Sizeable, Ui, Widget,
    WidgetCommon, WidgetStyle,
};
use position::Place;

#[derive(WidgetCommon)]
pub struct DebugWidget<'a> {
    #[conrod(common_builder)]
    pub common: widget::CommonBuilder,
    pub style: Style,
    pub windowing_state: &'a WindowingState,
    pub debug_win_id: WinId,
    pub hidpi_factor: f64,
}

pub struct State {
    ids: Ids,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, WidgetStyle)]
pub struct Style {}

widget_ids! {
    struct Ids {
        window_rect_display,
        snap_candidates_x[],
        snap_candidates_y[],
    }
}

impl<'a> DebugWidget<'a> {
    pub fn new(
        windowing_state: &'a WindowingState,
        debug_win_id: WinId,
        hidpi_factor: f64,
    ) -> Self {
        Self {
            common: widget::CommonBuilder::default(),
            style: Style::default(),
            windowing_state,
            debug_win_id,
            hidpi_factor,
        }
    }
}

impl<'a> Widget for DebugWidget<'a> {
    type State = State;
    type Style = Style;
    type Event = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        self.style.clone()
    }

    fn is_over(&self) -> widget::IsOverFn {
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
            debug_win_id: win_id,
            ..
        } = self;

        let win_rect = match windowing_state.win_normal_rect_f64(win_id) {
            Some(x) => x,
            None => return,
        };

        let win_rect_in_conrod = util::win_rect_to_conrod_rect(win_rect, rect);

        widget::Rectangle::fill_with(rect.dim(), conrod_core::color::rgba(1.0, 0.8, 0.0, 0.3))
            .xy(win_rect_in_conrod.xy())
            .wh(win_rect_in_conrod.dim())
            .graphics_for(id)
            .set(state.ids.window_rect_display, &mut ui);

        let debug = windowing_state.debug();

        macro_rules! get_id {
            ($list:ident, $i:expr) => {{
                let i: usize = $i;
                if i >= state.ids.$list.len() {
                    let new_len = if i == 0 { 4 } else { i * 2 };
                    state.update(|state| {
                        state
                            .ids
                            .$list
                            .resize(new_len, &mut ui.widget_id_generator());
                    });
                }
                state.ids.$list[i]
            }};
        }

        // Draw snapping segments.
        for (i, seg) in debug.snap_x_segments().enumerate() {
            let item_id = get_id!(snap_candidates_x, i);
            let pt1 = util::layout_pos_to_conrod_point([seg.x1 as f64, seg.y1 as f64], rect);
            let pt2 = util::layout_pos_to_conrod_point([seg.x2 as f64, seg.y2 as f64], rect);
            widget::Line::abs(pt1, pt2)
                .solid()
                .thickness(2.0)
                .color(conrod_core::color::RED.alpha(0.8))
                .graphics_for(id)
                .set(item_id, ui);
        }
        for (i, seg) in debug.snap_y_segments().enumerate() {
            let item_id = get_id!(snap_candidates_y, i);
            let pt1 = util::layout_pos_to_conrod_point([seg.x1 as f64, seg.y1 as f64], rect);
            let pt2 = util::layout_pos_to_conrod_point([seg.x2 as f64, seg.y2 as f64], rect);
            widget::Line::abs(pt1, pt2)
                .solid()
                .thickness(2.0)
                .color(conrod_core::color::GREEN.alpha(0.8))
                .graphics_for(id)
                .set(item_id, ui);
        }

    }

    fn default_x_position(&self, _ui: &Ui) -> Position {
        Position::Relative(position::Relative::Place(Place::Middle), None)
    }

    fn default_y_position(&self, _ui: &Ui) -> Position {
        Position::Relative(position::Relative::Place(Place::Middle), None)
    }
}
