use conrod_core::{color, widget};

macro_rules! value_iter_chain{
    ($item:expr, $(,)?) => {
        ::std::iter::once($item)
    };
    ($first:expr, $($others:expr),+ $(,)?) => {
        value_iter_chain!($first,).chain(value_iter_chain!($($others,)+))
    };
}

macro_rules! iter_chain{
    ($item:expr, $(,)?) => {
        $item
    };
    ($first:expr, $($others:expr),+ $(,)?) => {
        iter_chain!($first,).chain(iter_chain!($($others,)+))
    };
}

fn make_l_shape_polygon(
    origin: [f64; 2],
    extents: [f64; 2],
    thickness: f64,
) -> impl Iterator<Item = [f64; 2]> {
    // Imagine a L shape like the following:
    //
    //   a  ________  b
    //     |  ______|
    //     | | d      c
    //     | |
    //     |_|
    //   f     e
    //
    // The origin represents point `a`.
    // The extents represents `[b.x, f.y]`.
    // The thickness represents `abs(c - b)` and `abs(e - f)`.
    //
    // We produce points in the alphabetical order `a` to `f`. This order is
    // suitable for use with simple fan triangluation.

    let [x_o, y_o] = origin;
    let [x_bc, y_ef] = extents;
    let y_cd = y_o + thickness.copysign(if y_ef > y_o { 1.0 } else { -1.0 });
    let x_de = x_o + thickness.copysign(if x_bc > x_o { 1.0 } else { -1.0 });
    value_iter_chain![
        [x_o, y_o],
        [x_bc, y_o],
        [x_bc, y_cd],
        [x_de, y_cd],
        [x_de, y_ef],
        [x_o, y_ef],
    ]
}

fn make_rect(origin: [f64; 2], extents: [f64; 2]) -> impl Iterator<Item = [f64; 2]> {
    let [x_o, y_o] = origin;
    let [x_e, y_e] = extents;
    value_iter_chain![[x_o, y_o], [x_o, y_e], [x_e, y_e], [x_e, y_o]]
}

fn polygon_to_triangle_points<P, Iter>(mut points: Iter) -> impl Iterator<Item = [P; 3]>
where
    P: Copy,
    Iter: Iterator<Item = P>,
{
    let first = points.next();
    let mut first_and_prev = first.and_then(|first| points.next().map(|second| (first, second)));
    std::iter::from_fn(move || {
        first_and_prev
            .as_mut()
            .and_then(|&mut (first, ref mut prev)| {
                points.next().map(|point| {
                    let triangle = [first, *prev, point];
                    *prev = point;
                    triangle
                })
            })
    })
}

struct FrameColor {
    lower_a: color::Rgba,
    upper_a: color::Rgba,
    lower_b: color::Rgba,
    upper_b: color::Rgba,
    inside: color::Rgba,
}

fn make_frame(
    bottom_left: [f64; 2],
    top_right: [f64; 2],
    border_thickness: f64,
    frame_color: FrameColor,
) -> impl Iterator<Item = widget::triangles::Triangle<widget::triangles::ColoredPoint>> {
    // The frame is constructed from 4 L-shapes and a rectangle, laid out as
    // follow:
    //
    //      ________________
    //     |  ____________| |
    //     | |  ________| | |
    //     | | |        | | |
    //     | | |        | | |
    //     | | |        | | |
    //     | |_|________| | |
    //     |_|____________| |
    //     |________________|

    let lower_a_color = frame_color.lower_a;
    let upper_a_color = frame_color.upper_a;
    let lower_b_color = frame_color.lower_b;
    let upper_b_color = frame_color.upper_b;
    let inside_color = frame_color.inside;

    let line_thickness = border_thickness / 2.0;
    let [x_left, y_bottom] = bottom_left;
    let [x_right, y_top] = top_right;

    // Outmost (bottom-right) border:
    let lower_a = polygon_to_triangle_points(
        make_l_shape_polygon([x_right, y_bottom], [x_left, y_top], line_thickness)
            .map(move |point| (point, lower_a_color)),
    );

    // Outmost (top-left) border:
    let upper_a = polygon_to_triangle_points(
        make_l_shape_polygon(
            [x_left, y_top],
            [x_right - line_thickness, y_bottom + line_thickness],
            line_thickness,
        )
        .map(move |point| (point, upper_a_color)),
    );

    // Inner (bottom-right) border:
    let lower_b = polygon_to_triangle_points(
        make_l_shape_polygon(
            [x_right - line_thickness, y_bottom + line_thickness],
            [x_left + line_thickness, y_top - line_thickness],
            line_thickness,
        )
        .map(move |point| (point, lower_b_color)),
    );

    // Inner (top-left) border:
    let upper_b = polygon_to_triangle_points(
        make_l_shape_polygon(
            [x_left + line_thickness, y_top - line_thickness],
            [x_right - border_thickness, y_bottom + border_thickness],
            line_thickness,
        )
        .map(move |point| (point, upper_b_color)),
    );

    // Inside rectangle:
    let inside = polygon_to_triangle_points(
        make_rect(
            [x_left + border_thickness, y_top - border_thickness],
            [x_right - border_thickness, y_bottom + border_thickness],
        )
        .map(move |point| (point, inside_color)),
    );

    iter_chain![lower_a, upper_a, lower_b, upper_b, inside].map(widget::triangles::Triangle)
}

pub(super) fn make_panel_frame(
    bottom_left: [f64; 2],
    top_right: [f64; 2],
    border_thickness: f64,
    base_color: color::Color,
) -> impl Iterator<Item = widget::triangles::Triangle<widget::triangles::ColoredPoint>> {
    let hsla = base_color.to_hsl();
    let alpha = hsla.3;
    // The original colors are greyscale with luminance of:
    //     0.0, 0.875, 0.5, 1.0, 0.75
    // We treat the base colour as the fifth colour and scale the other colours
    // based on the original scales --
    //     0.875 = (1.0 - 0.75) / 2.0 + 0.75
    //     0.5 = 0.75 / 1.5
    let lower_a = color::Rgba(0.0, 0.0, 0.0, alpha);
    let upper_a = color::hsla(hsla.0, hsla.1, (1.0 - hsla.2) / 2.0 + hsla.2, alpha).to_rgb();
    let lower_b = color::hsla(hsla.0, hsla.1, hsla.2 / 1.5, alpha).to_rgb();
    let upper_b = color::Rgba(1.0, 1.0, 1.0, alpha);
    let inside = base_color.to_rgb();
    let frame_color = FrameColor {
        lower_a,
        upper_a,
        lower_b,
        upper_b,
        inside,
    };

    make_frame(bottom_left, top_right, border_thickness, frame_color)
}

pub(super) fn make_title_bar_gradient(
    bottom_left: [f64; 2],
    top_right: [f64; 2],
    color_left: color::Color,
    color_right: color::Color,
) -> impl Iterator<Item = widget::triangles::Triangle<widget::triangles::ColoredPoint>> {
    let [x_o, y_o] = bottom_left;
    let [x_e, y_e] = top_right;
    let color_left = color_left.to_rgb();
    let color_right = color_right.to_rgb();
    polygon_to_triangle_points(value_iter_chain![
        ([x_o, y_o], color_left),
        ([x_o, y_e], color_left),
        ([x_e, y_e], color_right),
        ([x_e, y_o], color_right),
    ])
    .map(widget::triangles::Triangle)
}
