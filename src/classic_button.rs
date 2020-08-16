use crate::classic_frame;

use conrod_core::{color, widget, widget_ids, Positionable, UiCell, Widget, WidgetCommon};
use widget::button::TimesClicked;

#[derive(Clone, Copy, Debug, WidgetCommon)]
pub struct ClassicButton {
    #[conrod(common_builder)]
    pub common: widget::CommonBuilder,
    pub button_type: ButtonType,
    pub hidpi_factor: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonType {
    Collapse,
    Uncollapse,
    Close,
}

widget_ids! {
    pub struct Ids {
        frame,
        icon,
    }
}

impl ClassicButton {
    pub fn new(button_type: ButtonType, hidpi_factor: f64) -> Self {
        ClassicButton {
            common: widget::CommonBuilder::default(),
            button_type,
            hidpi_factor,
        }
    }
}

impl Widget for ClassicButton {
    type State = Ids;
    type Style = ();
    type Event = TimesClicked;

    fn init_state(&self, id_gen: conrod_core::widget::id::Generator) -> Self::State {
        Ids::new(id_gen)
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: conrod_core::widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            id,
            state,
            rect,
            ui,
            ..
        } = args;
        let state: &mut widget::State<Ids> = state;
        let Self {
            button_type,
            hidpi_factor,
            ..
        } = self;

        let (interaction, times_triggered) = interaction_and_times_triggered(id, ui);

        // Draw a classic frame using triangles:
        let base_color = color::rgba(0.75, 0.75, 0.75, 1.0);
        let dpi_int = if hidpi_factor.fract() < 0.51 {
            hidpi_factor.trunc()
        } else {
            hidpi_factor.trunc() + 1.0
        };
        let border_thickness = 2.0 * dpi_int / hidpi_factor;
        let triangles = classic_frame::make_button_frame(
            rect.bottom_left(),
            rect.top_right(),
            border_thickness,
            base_color,
            interaction == Interaction::Press,
        );
        widget::Triangles::multi_color(triangles)
            .with_bounding_rect(rect)
            .middle_of(id)
            .graphics_for(id)
            .place_on_kid_area(false)
            .set(state.frame, ui);

        let icon_padding = (2.0 * hidpi_factor).round() / hidpi_factor;
        let mut icon_rect = rect
            .pad(border_thickness)
            .pad_left(icon_padding)
            .pad_right(icon_padding)
            .pad_bottom(icon_padding);
        if interaction == Interaction::Press {
            let shift =
                ((2.0 * hidpi_factor).round() - (1.0 * hidpi_factor).round()) / hidpi_factor;
            icon_rect = icon_rect.shift_x(shift).shift_y(-shift);
        }
        match button_type {
            ButtonType::Collapse => {
                let icon_triangles = classic_frame::make_collapse_button_icon(
                    icon_rect.bottom_left(),
                    icon_rect.top_right(),
                    hidpi_factor,
                );
                widget::Triangles::single_color(color::BLACK, icon_triangles)
                    .with_bounding_rect(icon_rect)
                    .top_left_with_margin_on(id, border_thickness)
                    .graphics_for(id)
                    .place_on_kid_area(false)
                    .set(state.icon, ui);
            }
            ButtonType::Uncollapse => {
                let icon_triangles = classic_frame::make_uncollapse_button_icon(
                    icon_rect.bottom_left(),
                    icon_rect.top_right(),
                    hidpi_factor,
                );
                widget::Triangles::single_color(color::BLACK, icon_triangles)
                    .with_bounding_rect(icon_rect)
                    .top_left_with_margin_on(id, border_thickness)
                    .graphics_for(id)
                    .place_on_kid_area(false)
                    .set(state.icon, ui);
            }
            ButtonType::Close => {
                let icon_triangles = classic_frame::make_close_button_icon(
                    icon_rect.bottom_left(),
                    icon_rect.top_right(),
                    hidpi_factor,
                );
                widget::Triangles::single_color(color::BLACK, icon_triangles)
                    .with_bounding_rect(icon_rect)
                    .top_left_with_margin_on(id, border_thickness)
                    .graphics_for(id)
                    .place_on_kid_area(false)
                    .set(state.icon, ui);
            }
        }

        TimesClicked(times_triggered)
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Interaction {
    Idle,
    Hover,
    Press,
}

fn interaction_and_times_triggered(button_id: widget::Id, ui: &UiCell) -> (Interaction, u16) {
    let input = ui.widget_input(button_id);
    let mouse_interaction = input.mouse().map_or(Interaction::Idle, |mouse| {
        if mouse.buttons.left().is_down() {
            if ui.global_input().current.widget_under_mouse == Some(button_id) {
                Interaction::Press
            } else {
                Interaction::Idle
            }
        } else {
            Interaction::Hover
        }
    });
    let interaction = match mouse_interaction {
        Interaction::Idle | Interaction::Hover => {
            let is_touch_press = ui
                .global_input()
                .current
                .touch
                .values()
                .any(|t| t.start.widget == Some(button_id) && t.widget == Some(button_id));
            if is_touch_press {
                Interaction::Press
            } else {
                mouse_interaction
            }
        }
        Interaction::Press => Interaction::Press,
    };
    let times_triggered = (input.clicks().left().count() + input.taps().count()) as u16;
    (interaction, times_triggered)
}
