use conrod_core::{
    builder_method, builder_methods,
    position::{self, Place},
    text, widget, widget_ids, Borderable, Color, Colorable, Dimensions, FontSize, Labelable, Point,
    Position, Positionable, Scalar, Sizeable, Theme, Ui, Widget, WidgetCommon, WidgetStyle,
};
use widget::KidAreaArgs;

#[derive(WidgetCommon)]
pub struct WindowFrame<'a> {
    #[conrod(common_builder)]
    pub common: widget::CommonBuilder,
    pub style: Style,
    pub title: &'a str,
}

pub struct State {
    ids: Ids,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, WidgetStyle)]
pub struct Style {
    /// The color of the Canvas' rectangle surface.
    #[conrod(default = "theme.background_color")]
    pub color: Option<Color>,
    /// The width of the border surrounding the Canvas' rectangle.
    // #[conrod(default = "theme.border_width")]
    #[conrod(default = "2.0")]
    pub border: Option<Scalar>,
    /// The color of the Canvas' border.
    #[conrod(default = "theme.border_color")]
    pub border_color: Option<Color>,

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
    #[conrod(default = "None")]
    pub title_bar_color: Option<Option<Color>>,
    /// The color of the title bar's text.
    #[conrod(default = "theme.label_color")]
    pub title_bar_text_color: Option<Color>,
    /// The font size for the title bar's text.
    #[conrod(default = "theme.font_size_small")]
    pub title_bar_font_size: Option<FontSize>,
    /// The way in which the title bar's text should wrap.
    #[conrod(default = "None")]
    pub title_bar_maybe_wrap: Option<Option<widget::text::Wrap>>,
    /// The distance between lines for multi-line title bar text.
    #[conrod(default = "1.0")]
    pub title_bar_line_spacing: Option<Scalar>,
    /// The label's typographic alignment over the *x* axis.
    #[conrod(default = "text::Justify::Left")]
    pub title_bar_justify: Option<text::Justify>,
}

widget_ids! {
    struct Ids {
        rectangle,
        title_bar,
    }
}

// #[derive(Clone, Copy, PartialEq, Debug)]
// pub enum HitTest {
//     Content,
//     TitleBar,
//     TopBorder,
//     LeftBorder,
//     RightBorder,
//     BottomBorder,
//     TopLeftCorner,
//     TopRightCorner,
//     BorromLeftCorner,
//     BottomRightCorner,
//     // CollapseButton,
//     // CloseButton,
// }

// const WINDOW_BORDER: Scalar = 4.0;

impl<'a> WindowFrame<'a> {
    pub fn new() -> Self {
        Self {
            common: widget::CommonBuilder::default(),
            style: Style::default(),
            title: "",
        }
    }

    builder_methods! {
        pub title { title = &'a str }
    }

    // fn hit_test(&self, dim: Dimensions, style: &Style, theme: &Theme, point: Point) -> HitTest {
    //     let title_bar_rect = {
    //         // let (xy, dim) = rect.xy_dim();
    //         let font_size = style.title_bar_font_size(theme);
    //         let (h, rel_y) = title_bar_h_rel_y(dim[1], font_size);
    //         let rel_xy = [0.0, rel_y];
    //         let dim = [dim[0], h];
    //         // let title_bar_rect = Rect::from_xy_dim([rel_xy[0] + xy[0], rel_xy[1] + xy[1]], dim);
    //         Rect::from_xy_dim(rel_xy, dim)
    //     };
    //     if title_bar_rect.is_over(point) {
    //         HitTest::TitleBar
    //     } else {
    //         let bottom_right_grip = {
    //             let [w, h] = dim;
    //             let grip_size = GRIP_SIZE;
    //             Rect::from_xy_dim([w / 2.0 - grip_size / 2.0, -h / 2.0 + grip_size / 2.0], [grip_size, grip_size])
    //         };
    //         if bottom_right_grip.is_over(point) {
    //             HitTest::BottomRightCorner
    //         } else {
    //             HitTest::Content
    //         }
    //     }
    // }
}

impl<'a> Colorable for WindowFrame<'a> {
    builder_method!(color { style.color = Some(Color) });
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
        widget::KidArea {
            rect: args
                .rect
                .pad(layout::WINDOW_BORDER as Scalar)
                .pad_top((layout::TITLE_BAR_HEIGHT + layout::WINDOW_BORDER) as Scalar),
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
        let Self { style, title, .. } = self;

        // BorderedRectangle widget as the rectangle backdrop.
        let dim = rect.dim();
        let color = style.color(ui.theme());
        let border = style.border(ui.theme());
        let border_color = style.border_color(ui.theme());
        widget::BorderedRectangle::new(dim)
            .color(color)
            .border(border)
            .border_color(border_color)
            .middle_of(id)
            .graphics_for(id)
            .place_on_kid_area(false)
            .set(state.ids.rectangle, &mut ui);

        // // BorderedRectangle widget for the bottom-right size grip.
        // {
        //     let dim = [WINDOW_BORDER, WINDOW_BORDER];
        //     let color = Color::Rgba(1.0, 0.0, 0.0, 1.0);
        //     let border = 1.0;
        //     let border_color = Color::Rgba(0.0, 1.0, 0.0, 1.0);
        //     widget::BorderedRectangle::new(dim)
        //         .color(color)
        //         .border(border)
        //         .border_color(border_color)
        //         .bottom_right_of(id)
        //         .graphics_for(id)
        //         .place_on_kid_area(false)
        //         .set(state.ids.size_grip, &mut ui);
        // }

        // TitleBar widget if we were given some label.
        {
            let color = style.title_bar_color(&ui.theme).unwrap_or(color);
            let font_size = style.title_bar_font_size(&ui.theme);
            let label_color = style.title_bar_text_color(&ui.theme);
            let justify = style.title_bar_justify(&ui.theme);
            let line_spacing = style.title_bar_line_spacing(&ui.theme);
            let maybe_wrap = style.title_bar_maybe_wrap(&ui.theme);
            widget::TitleBar::new(title, state.ids.rectangle)
                .and_mut(|title_bar| {
                    title_bar.style.maybe_wrap = Some(maybe_wrap);
                    title_bar.style.justify = Some(justify);
                })
                .color(color)
                .border(border)
                .border_color(border_color)
                .label_font_size(font_size)
                .label_color(label_color)
                .line_spacing(line_spacing)
                .h(layout::TITLE_BAR_HEIGHT as Scalar)
                .graphics_for(id)
                .place_on_kid_area(false)
                .set(state.ids.title_bar, &mut ui);
        }

        // if let Some(m) = ui.widget_input(id).mouse() {
        //     let ht = self.hit_test(dim, &style, &ui.theme, m.rel_xy());
        //     match ht {
        //         HitTest::Content => {}
        //         HitTest::TitleBar => {
        //             ui.set_mouse_cursor(cursor::MouseCursor::Grab);
        //         }
        //         HitTest::TopBorder => {}
        //         HitTest::LeftBorder => {}
        //         HitTest::RightBorder => {}
        //         HitTest::BottomBorder => {}
        //         HitTest::TopLeftCorner => {}
        //         HitTest::TopRightCorner => {}
        //         HitTest::BorromLeftCorner => {}
        //         HitTest::BottomRightCorner => {
        //             ui.set_mouse_cursor(cursor::MouseCursor::ResizeTopLeftBottomRight);
        //         }
        //     }
        // }
    }
}
