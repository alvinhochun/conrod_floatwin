use super::WindowingState;

impl WindowingState {
    pub(crate) fn debug(&self) -> Debug {
        Debug {
            windowing_state: self,
        }
    }
}

pub(crate) struct Debug<'a> {
    windowing_state: &'a WindowingState,
}

pub(crate) struct LineSegment {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl<'a> Debug<'a> {
    pub fn snap_x_segments<'b>(&'b self) -> impl Iterator<Item = LineSegment> + 'a {
        let win_state = self.windowing_state;
        let hidpi_factor = win_state.hidpi_factor as f32;
        win_state
            .maybe_dragging_window
            .as_ref()
            .map(|s| {
                s.snap_candidates_x.iter().map(move |(_, snap_seg)| {
                    let x = snap_seg.perpendicular_dim() as f32 / hidpi_factor;
                    let y1 = snap_seg.dim_range().lower() as f32 / hidpi_factor;
                    let y2 = snap_seg.dim_range().upper() as f32 / hidpi_factor;
                    LineSegment {
                        x1: x,
                        y1,
                        x2: x,
                        y2,
                    }
                })
            })
            .into_iter()
            .flatten()
    }

    pub fn snap_y_segments<'b>(&'b self) -> impl Iterator<Item = LineSegment> + 'a {
        let win_state = self.windowing_state;
        let hidpi_factor = win_state.hidpi_factor as f32;
        win_state
            .maybe_dragging_window
            .as_ref()
            .map(|s| {
                s.snap_candidates_y.iter().map(move |(_, snap_seg)| {
                    let y = snap_seg.perpendicular_dim() as f32 / hidpi_factor;
                    let x1 = snap_seg.dim_range().lower() as f32 / hidpi_factor;
                    let x2 = snap_seg.dim_range().upper() as f32 / hidpi_factor;
                    LineSegment {
                        x1,
                        y1: y,
                        x2,
                        y2: y,
                    }
                })
            })
            .into_iter()
            .flatten()
    }
}
