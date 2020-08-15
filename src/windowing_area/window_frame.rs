use super::layout;
use layout::FrameMetrics;

use crate::empty_widget::EmptyWidget;
use conrod_core::{
    builder_methods, color,
    position::{self},
    text, widget, widget_ids, Borderable, Color, Colorable, FontSize, Labelable, Positionable,
    Sizeable, Widget, WidgetCommon, WidgetStyle,
};
use widget::KidAreaArgs;

mod classic_frame;

#[derive(WidgetCommon)]
pub struct WindowFrame<'a> {
    #[conrod(common_builder)]
    pub common: widget::CommonBuilder,
    pub style: Style,
    pub title: &'a str,
    pub is_focused: bool,
    pub is_collapsible: bool,
    pub is_closable: bool,
    pub(crate) frame_metrics: FrameMetrics,
}

pub struct State {
    ids: Ids,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, WidgetStyle)]
pub struct Style {
    /// The color of the window frame.
    #[conrod(default = "theme.background_color")]
    pub frame_color: Option<Color>,
    // /// The width of the border surrounding the Canvas' rectangle.
    // #[conrod(default = "theme.border_width")]
    // #[conrod(default = "2.0")]
    // pub border: Option<Scalar>,
    // /// The color of the Canvas' border.
    // #[conrod(default = "theme.border_color")]
    // pub border_color: Option<Color>,

    // /// Padding for the left edge of the Canvas' kid area.
    // #[conrod(default = "theme.padding.x.start")]
    // pub pad_left: Option<Scalar>,
    // /// Padding for the right edge of the Canvas' kid area.
    // #[conrod(default = "theme.padding.x.end")]
    // pub pad_right: Option<Scalar>,
    // /// Padding for the bottom edge of the Canvas' kid area.
    // #[conrod(default = "theme.padding.y.start")]
    // pub pad_bottom: Option<Scalar>,
    // /// Padding for the top edge of the Canvas' kid area.
    // #[conrod(default = "theme.padding.y.end")]
    // pub pad_top: Option<Scalar>,
    /// The color of the title bar. Defaults to the color of the Canvas.
    #[conrod(default = "theme.shape_color")]
    pub title_bar_color: Option<Color>,
    /// The color of the title bar's text.
    #[conrod(default = "theme.label_color")]
    pub title_bar_text_color: Option<Color>,
    /// The font size for the title bar's text.
    #[conrod(default = "theme.font_size_small")]
    pub title_bar_font_size: Option<FontSize>,
    // /// The way in which the title bar's text should wrap.
    // #[conrod(default = "None")]
    // pub title_bar_maybe_wrap: Option<Option<widget::text::Wrap>>,
    // /// The distance between lines for multi-line title bar text.
    // #[conrod(default = "1.0")]
    // pub title_bar_line_spacing: Option<Scalar>,
    /// The label's typographic alignment over the *x* axis.
    #[conrod(default = "text::Justify::Left")]
    pub title_bar_justify: Option<text::Justify>,
}

#[derive(Clone, Debug)]
pub struct Event {
    pub collapse_clicked: widget::button::TimesClicked,
    pub close_clicked: widget::button::TimesClicked,
}

widget_ids! {
    struct Ids {
        frame,
        title_bar_box,
        title_text_clip,
        title_text,
        button_collapse,
        button_close,
    }
}

impl<'a> WindowFrame<'a> {
    pub(crate) fn new(frame_metrics: FrameMetrics) -> Self {
        Self {
            common: widget::CommonBuilder::default(),
            style: Style::default(),
            title: "",
            frame_metrics,
            is_focused: true,
            is_collapsible: true,
            is_closable: false,
        }
    }

    builder_methods! {
        pub title { title = &'a str }
        pub is_focused { is_focused = bool }
        pub is_collapsible { is_collapsible = bool }
        pub is_closable { is_closable = bool }
    }

    pub fn frame_color(mut self, color: Color) -> Self {
        self.style.frame_color = Some(color);
        self
    }

    pub fn title_bar_color(mut self, color: Color) -> Self {
        self.style.title_bar_color = Some(color);
        self
    }
}

impl<'a> Widget for WindowFrame<'a> {
    type State = State;
    type Style = Style;
    type Event = Event;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        self.style.clone()
    }

    fn kid_area(&self, args: KidAreaArgs<Self>) -> widget::KidArea {
        let rect = args
            .rect
            .pad(self.frame_metrics.border_thickness)
            .pad_top(self.frame_metrics.title_bar_height + self.frame_metrics.gap_below_title_bar);
        widget::KidArea {
            rect,
            pad: conrod_core::position::Padding::none(),
        }
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            id,
            state,
            rect,
            mut ui,
            ..
        } = args;
        let state: &mut widget::State<State> = state;
        let Self {
            style,
            title,
            is_focused,
            is_collapsible,
            is_closable,
            frame_metrics,
            ..
        } = self;
        let style: Style = style;

        // Draw a classic frame using triangles:
        let base_color = style.frame_color(ui.theme());
        let triangles = classic_frame::make_panel_frame(
            rect.bottom_left(),
            rect.top_right(),
            // The frame border is 4 units, but the actual panel frame border
            // is only 2 units.
            frame_metrics.border_thickness / 2.0,
            base_color,
        );
        widget::Triangles::multi_color(triangles)
            .with_bounding_rect(rect)
            .middle_of(id)
            .graphics_for(id)
            .place_on_kid_area(false)
            .set(state.ids.frame, &mut ui);

        let left = rect.pad_left(frame_metrics.border_thickness).left();
        let right = rect.pad_right(frame_metrics.border_thickness).right();
        let top = rect.pad_top(frame_metrics.border_thickness).top();
        let bottom = top - frame_metrics.title_bar_height;
        let title_bar_rect = conrod_core::Rect::from_corners([left, bottom], [right, top]);

        // Draw a title bar rect:
        let (color_left, color_right);
        if is_focused {
            color_left = color::rgba(0.0, 0.0, 0.5, 1.0);
            color_right = color::rgba(0.05, 0.5, 0.8, 1.0);
        } else {
            color_left = color::rgba(0.5, 0.5, 0.5, 1.0);
            color_right = color::rgba(0.7, 0.7, 0.7, 1.0);
        }
        let triangles = classic_frame::make_title_bar_gradient(
            title_bar_rect.bottom_left(),
            title_bar_rect.top_right(),
            color_left,
            color_right,
        );
        widget::Triangles::multi_color(triangles)
            .with_bounding_rect(title_bar_rect)
            .graphics_for(id)
            .place_on_kid_area(false)
            .set(state.ids.title_bar_box, &mut ui);

        let button_width = frame_metrics.title_button_width;
        let button_height =
            frame_metrics.title_bar_height - frame_metrics.title_button_padding * 2.0;

        // Collapse (minimize) button:
        let collapse_clicked = if is_collapsible {
            widget::Button::new()
                .label("-")
                .small_font(ui)
                .label_y(position::Relative::Align(position::Align::Middle))
                .label_color(color::BLACK)
                .color(color::GRAY)
                .mid_left_with_margin_on(
                    state.ids.title_bar_box,
                    frame_metrics.title_button_padding,
                )
                .w_h(button_width, button_height)
                .parent(id)
                .place_on_kid_area(false)
                .set(state.ids.button_collapse, &mut ui)
        } else {
            widget::button::TimesClicked(0)
        };

        // Close button:
        let close_clicked = if is_closable {
            widget::Button::new()
                .label("x")
                .small_font(ui)
                .label_y(position::Relative::Align(position::Align::Middle))
                .label_color(color::BLACK)
                .color(color::GRAY)
                .mid_right_with_margin_on(
                    state.ids.title_bar_box,
                    frame_metrics.title_button_padding,
                )
                .w_h(button_width, button_height)
                .parent(id)
                .place_on_kid_area(false)
                .set(state.ids.button_close, &mut ui)
        } else {
            widget::button::TimesClicked(0)
        };

        // Set the clipping box for the title bar text:
        let left_padding =
            frame_metrics.title_text_padding + if is_collapsible { button_width } else { 0.0 };
        let right_padding =
            frame_metrics.title_text_padding + if is_closable { button_width } else { 0.0 };
        EmptyWidget::new()
            .x_position_relative_to(
                state.ids.title_bar_box,
                position::Relative::Place(position::Place::Start(Some(left_padding))),
            )
            .align_middle_y_of(state.ids.title_bar_box)
            .padded_w_of(
                state.ids.title_bar_box,
                (left_padding + right_padding) / 2.0,
            )
            .h_of(state.ids.title_bar_box)
            .graphics_for(state.ids.title_bar_box)
            .place_on_kid_area(false)
            .crop_kids()
            .set(state.ids.title_text_clip, &mut ui);

        // Draw the title bar text:
        let font_size = style.title_bar_font_size(&ui.theme);
        widget::Text::new(title)
            .no_line_wrap()
            .left_justify()
            .w_of(state.ids.title_text_clip)
            .middle_of(state.ids.title_text_clip)
            .color(color::WHITE)
            .font_size(font_size)
            .graphics_for(state.ids.title_text_clip)
            .place_on_kid_area(false)
            .set(state.ids.title_text, &mut ui);

        Event {
            collapse_clicked,
            close_clicked,
        }
    }
}
