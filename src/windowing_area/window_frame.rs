use super::layout;
use layout::FrameMetrics;

use conrod_core::{
    builder_methods,
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

widget_ids! {
    struct Ids {
        frame,
        title_bar,
    }
}

impl<'a> WindowFrame<'a> {
    pub(crate) fn new(frame_metrics: FrameMetrics) -> Self {
        Self {
            common: widget::CommonBuilder::default(),
            style: Style::default(),
            title: "",
            frame_metrics,
        }
    }

    builder_methods! {
        pub title { title = &'a str }
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
    type Event = ();

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
            frame_metrics,
            ..
        } = self;
        let style: Style = style;

        // Draw a classic frame using triangles:
        let base_color = style.frame_color(ui.theme());
        let triangles = classic_frame::make_frame(
            rect.bottom_left(),
            rect.top_right(),
            frame_metrics.border_thickness,
            base_color,
        );
        widget::Triangles::multi_color(triangles)
            .with_bounding_rect(rect)
            .middle_of(id)
            .graphics_for(id)
            .place_on_kid_area(false)
            .set(state.ids.frame, &mut ui);

        // TitleBar widget.
        {
            let color = style.title_bar_color(&ui.theme);
            let font_size = style.title_bar_font_size(&ui.theme);
            let label_color = style.title_bar_text_color(&ui.theme);
            let justify = style.title_bar_justify(&ui.theme);
            widget::TitleBar::new(title, state.ids.frame)
                .and_mut(|title_bar| {
                    title_bar.style.maybe_wrap = Some(None);
                    title_bar.style.justify = Some(justify);
                })
                .color(color)
                .border(0.0)
                .label_font_size(font_size)
                .label_color(label_color)
                .y_position_relative_to(
                    id,
                    position::Relative::Place(position::Place::End(Some(
                        frame_metrics.border_thickness,
                    ))),
                )
                .w(rect.w() - frame_metrics.border_thickness * 2.0)
                .h(frame_metrics.title_bar_height)
                .graphics_for(id)
                .place_on_kid_area(false)
                .set(state.ids.title_bar, &mut ui);
        }
    }
}
