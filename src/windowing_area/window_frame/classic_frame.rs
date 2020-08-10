use conrod_core::widget;

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

pub(super) fn make_frame(
    bottom_left: [f64; 2],
    top_right: [f64; 2],
    border_thickness: f64,
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
    //
    //
    // If we consider the thickness of the L-shapes to be 1 unit, the actual
    // border of the frame will be 4 units.

    let lower_a_color = conrod_core::color::Rgba(0.0, 0.0, 0.0, 1.0);
    let upper_a_color = conrod_core::color::Rgba(0.875, 0.875, 0.875, 1.0);
    let lower_b_color = conrod_core::color::Rgba(0.5, 0.5, 0.5, 1.0);
    let upper_b_color = conrod_core::color::Rgba(1.0, 1.0, 1.0, 1.0);
    let inside_color = conrod_core::color::Rgba(0.75, 0.75, 0.75, 1.0);

    let double_line_thickness = border_thickness / 2.0;
    let line_thickness = double_line_thickness / 2.0;
    let [x_left, y_bottom] = bottom_left;
    let [x_right, y_top] = top_right;

    // Outmost (bottom-right) dark border:
    let lower_a = polygon_to_triangle_points(
        make_l_shape_polygon([x_right, y_bottom], [x_left, y_top], line_thickness)
            .map(move |point| (point, lower_a_color)),
    );

    // Outmost (top-left) light border:
    let upper_a = polygon_to_triangle_points(
        make_l_shape_polygon(
            [x_left, y_top],
            [x_right - line_thickness, y_bottom + line_thickness],
            line_thickness,
        )
        .map(move |point| (point, upper_a_color)),
    );

    // Inner (bottom-right) dark border:
    let lower_b = polygon_to_triangle_points(
        make_l_shape_polygon(
            [x_right - line_thickness, y_bottom + line_thickness],
            [x_left + line_thickness, y_top - line_thickness],
            line_thickness,
        )
        .map(move |point| (point, lower_b_color)),
    );

    // Inner (top-left) light border:
    let upper_b = polygon_to_triangle_points(
        make_l_shape_polygon(
            [x_left + line_thickness, y_top - line_thickness],
            [
                x_right - double_line_thickness,
                y_bottom + double_line_thickness,
            ],
            line_thickness,
        )
        .map(move |point| (point, upper_b_color)),
    );

    // Inside rectangle:
    let inside = polygon_to_triangle_points(
        make_rect(
            [
                x_left + double_line_thickness,
                y_top - double_line_thickness,
            ],
            [
                x_right - double_line_thickness,
                y_bottom + double_line_thickness,
            ],
        )
        .map(move |point| (point, inside_color)),
    );

    iter_chain![lower_a, upper_a, lower_b, upper_b, inside].map(widget::triangles::Triangle)
}
