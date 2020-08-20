use super::dim::{DimRange, Dir, Horizontal, Vertical};

pub type DimRangeH = DimRange<i32, Horizontal>;
pub type DimRangeV = DimRange<i32, Vertical>;

#[derive(Clone, Copy, Debug)]
pub struct SnapSegment<D: Dir> {
    perpendicular_dim: i32,
    dim_range: DimRange<i32, D>,
}

pub type SnapSegmentH = SnapSegment<Horizontal>;
pub type SnapSegmentV = SnapSegment<Vertical>;

impl<D: Dir> SnapSegment<D> {
    pub fn new(perpendicular_dim: i32, dim_range: DimRange<i32, D>) -> Self {
        Self {
            perpendicular_dim,
            dim_range,
        }
    }

    pub fn perpendicular_dim(self) -> i32 {
        self.perpendicular_dim
    }

    pub fn dim_range(self) -> DimRange<i32, D> {
        self.dim_range
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Anchor {
    None,
    LowerEdge,
    UpperEdge,
    LowerAndUpperEdges,
}
