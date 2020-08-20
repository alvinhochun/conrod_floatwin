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

pub(super) fn make_button_frame(
    bottom_left: [f64; 2],
    top_right: [f64; 2],
    border_thickness: f64,
    base_color: color::Color,
    is_clicked: bool,
) -> impl Iterator<Item = widget::triangles::Triangle<widget::triangles::ColoredPoint>> {
    let hsla = base_color.to_hsl();
    let alpha = hsla.3;
    // The original colors are greyscale with luminance of:
    //     0.0, 1.0, 0.5, 0.875, 0.75
    // We treat the base colour as the fifth colour and scale the other colours
    // based on the original scales --
    //     0.875 = (1.0 - 0.75) / 2.0 + 0.75
    //     0.5 = 0.75 / 1.5
    let lower_a = color::Rgba(0.0, 0.0, 0.0, alpha);
    let upper_a = color::Rgba(1.0, 1.0, 1.0, alpha);
    let lower_b = color::hsla(hsla.0, hsla.1, hsla.2 / 1.5, alpha).to_rgb();
    let upper_b = color::hsla(hsla.0, hsla.1, (1.0 - hsla.2) / 2.0 + hsla.2, alpha).to_rgb();
    let (lower_a, upper_a, lower_b, upper_b) = if is_clicked {
        (upper_a, lower_a, upper_b, lower_b)
    } else {
        (lower_a, upper_a, lower_b, upper_b)
    };
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

pub(super) fn make_close_button_icon(
    bottom_left: [f64; 2],
    top_right: [f64; 2],
    hidpi_factor: f64,
) -> impl Iterator<Item = widget::triangles::Triangle<conrod_core::Point>> {
    let [x_o, y_o] = bottom_left;
    let [x_e, y_e] = top_right;
    let width = x_e - x_o;
    let height = y_e - y_o;
    let px_width = (width * hidpi_factor).round();
    let px_height = (height * hidpi_factor).round();
    let (icon_px_width, icon_px_height) = {
        let shape_width_from_width = (px_width / 11.0 * 8.0).round();
        let shape_height_from_width = (shape_width_from_width / 8.0 * 7.0).round();
        let shape_height_from_height = (px_height / 9.0 * 7.0).round();
        if shape_height_from_height < shape_height_from_width {
            let shape_width_from_height = (shape_height_from_height / 7.0 * 8.0).round();
            (shape_width_from_height, shape_height_from_height)
        } else {
            (shape_width_from_width, shape_height_from_width)
        }
    };
    let icon_pad_left = ((px_width - icon_px_width) / 2.0).round() / hidpi_factor;
    let icon_pad_bottom = ((px_height - icon_px_height) / 2.0).round() / hidpi_factor;
    let icon_width = icon_px_width / hidpi_factor;
    let icon_height = icon_px_height / hidpi_factor;

    let icon_bottom_left = [x_o + icon_pad_left, y_o + icon_pad_bottom];
    let icon_top_right = [
        x_o + icon_pad_left + icon_width,
        y_o + icon_pad_bottom + icon_height,
    ];

    make_close_icon_shape(icon_bottom_left, icon_top_right)
}

fn make_close_icon_shape(
    bottom_left: [f64; 2],
    top_right: [f64; 2],
) -> impl Iterator<Item = widget::triangles::Triangle<conrod_core::Point>> {
    let [x_o, y_o] = bottom_left;
    let [x_e, y_e] = top_right;
    let icon_width = x_e - x_o;
    let icon_height = y_e - y_o;
    let (sx, sy) = {
        if icon_width > icon_height {
            let sx = icon_width / 5.0;
            (sx, sx - (icon_width - icon_height))
        } else {
            let sy = icon_height / 5.0;
            (sy - (icon_height - icon_width), sy)
        }
    };
    let x_mid_offset = (icon_width / 2.0) - sx;
    let y_mid_offset = (icon_height / 2.0) - sy;

    let part1 = polygon_to_triangle_points(value_iter_chain![
        [x_o, y_o],
        [x_o, y_o + sy],
        [x_o + icon_width - sx, y_o + icon_height],
        [x_o + icon_width, y_o + icon_height],
        [x_o + icon_width, y_o + icon_height - sy],
        [x_o + sx, y_o],
    ]);
    let part2 = polygon_to_triangle_points(value_iter_chain![
        [x_o, y_o + icon_height],
        [x_o + sx, y_o + icon_height],
        [x_o + sx + x_mid_offset, y_o + icon_height - x_mid_offset],
        [x_o + y_mid_offset, y_o + sy + y_mid_offset],
        [x_o, y_o + icon_height - sy],
    ]);
    let part3 = polygon_to_triangle_points(value_iter_chain![
        [x_o + icon_width, y_o],
        [x_o + icon_width - sx, y_o],
        [x_o + sx + x_mid_offset, y_o + x_mid_offset],
        [x_o + icon_width - y_mid_offset, y_o + sy + y_mid_offset],
        [x_o + icon_width, y_o + sy],
    ]);
    iter_chain![part1, part2, part3,].map(widget::triangles::Triangle)
}

pub(super) fn make_uncollapse_button_icon(
    bottom_left: [f64; 2],
    top_right: [f64; 2],
    hidpi_factor: f64,
) -> impl Iterator<Item = widget::triangles::Triangle<conrod_core::Point>> {
    let [x_o, y_o] = bottom_left;
    let [x_e, y_e] = top_right;
    let width = x_e - x_o;
    let height = y_e - y_o;
    let px_width = (width * hidpi_factor).round();
    let px_height = (height * hidpi_factor).round();
    let (icon_px_width, icon_px_height) = {
        let shape_width_from_width = (px_width / 11.0 * 4.0).round();
        let shape_height_from_width = (shape_width_from_width / 4.0 * 7.0).round();
        let shape_height_from_height = (px_height / 9.0 * 7.0).round();
        if shape_height_from_height < shape_height_from_width {
            let shape_width_from_height = (shape_height_from_height / 7.0 * 4.0).round();
            (shape_width_from_height, shape_height_from_height)
        } else {
            (shape_width_from_width, shape_height_from_width)
        }
    };
    let icon_pad_left = {
        let mut pad = (px_width - icon_px_width) / 2.0;
        let diff = icon_px_width - icon_px_height / 2.0;
        if diff >= 1.0 {
            // This is to prevent the icon becoming imbalanced.
            pad += diff;
        }
        pad.round() / hidpi_factor
    };
    let icon_pad_bottom = ((px_height - icon_px_height) / 2.0).round() / hidpi_factor;
    let icon_width = icon_px_width / hidpi_factor;
    let icon_height = icon_px_height / hidpi_factor;

    let icon_bottom_left = [x_o + icon_pad_left, y_o + icon_pad_bottom];
    let icon_top_right = [
        x_o + icon_pad_left + icon_width,
        y_o + icon_pad_bottom + icon_height,
    ];
    make_right_arrow_icon_shape(icon_bottom_left, icon_top_right)
}

fn make_right_arrow_icon_shape(
    bottom_left: [f64; 2],
    top_right: [f64; 2],
) -> impl Iterator<Item = widget::triangles::Triangle<conrod_core::Point>> {
    let [x_o, y_o] = bottom_left;
    let [x_e, y_e] = top_right;
    let icon_width = x_e - x_o;
    let icon_height = y_e - y_o;
    let half_height = icon_height / 2.0;
    let tip_shift_x = if icon_width < half_height {
        icon_width
    } else {
        // Add a small offset to the tip so that it won't lie exactly at the
        // middle of the pixel.
        half_height + 0.01
    };

    let triangle = [
        [x_o, y_o],
        [x_o, y_e],
        [x_o + tip_shift_x, y_o + half_height],
    ];
    std::iter::once(triangle).map(widget::triangles::Triangle)
}

pub(super) fn make_collapse_button_icon(
    bottom_left: [f64; 2],
    top_right: [f64; 2],
    hidpi_factor: f64,
) -> impl Iterator<Item = widget::triangles::Triangle<conrod_core::Point>> {
    let [x_o, y_o] = bottom_left;
    let [x_e, y_e] = top_right;
    let width = x_e - x_o;
    let height = y_e - y_o;
    let px_width = (width * hidpi_factor).round();
    let px_height = (height * hidpi_factor).round();
    let (icon_px_width, icon_px_height) = {
        let shape_width_from_width = (px_width / 11.0 * 7.0).round();
        let shape_height_from_width = (shape_width_from_width / 7.0 * 4.0).round();
        let shape_height_from_height = (px_height / 9.0 * 4.0).round();
        if shape_height_from_height < shape_height_from_width {
            let shape_width_from_height = (shape_height_from_height / 4.0 * 7.0).round();
            (shape_width_from_height, shape_height_from_height)
        } else {
            (shape_width_from_width, shape_height_from_width)
        }
    };
    let icon_pad_left = ((px_width - icon_px_width) / 2.0).round() / hidpi_factor;
    let icon_pad_top = {
        let mut pad = (px_height - icon_px_height) / 2.0;
        let diff = icon_px_height - icon_px_width / 2.0;
        if diff >= 1.0 {
            // This is to prevent the icon becoming imbalanced.
            pad += diff;
        }
        pad.round() / hidpi_factor
    };
    let icon_width = icon_px_width / hidpi_factor;
    let icon_height = icon_px_height / hidpi_factor;

    let icon_bottom_left = [x_o + icon_pad_left, y_e - icon_pad_top - icon_height];
    let icon_top_right = [x_o + icon_pad_left + icon_width, y_e - icon_pad_top];
    make_down_arrow_icon_shape(icon_bottom_left, icon_top_right)
}

fn make_down_arrow_icon_shape(
    bottom_left: [f64; 2],
    top_right: [f64; 2],
) -> impl Iterator<Item = widget::triangles::Triangle<conrod_core::Point>> {
    let [x_o, y_o] = bottom_left;
    let [x_e, y_e] = top_right;
    let icon_width = x_e - x_o;
    let icon_height = y_e - y_o;
    let half_width = icon_width / 2.0;
    let tip_shift_y = if icon_height < half_width {
        icon_height
    } else {
        // Add a small offset to the tip so that it won't lie exactly at the
        // middle of the pixel.
        half_width + 0.01
    };

    let triangle = [
        [x_o, y_e],
        [x_e, y_e],
        [x_o + half_width, y_e - tip_shift_y],
    ];
    std::iter::once(triangle).map(widget::triangles::Triangle)
}
