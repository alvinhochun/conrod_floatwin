#[derive(Clone, Copy, Debug)]
pub struct DimRange {
    start: i32,
    end: i32,
}

impl DimRange {
    pub fn new(a: i32, b: i32) -> Self {
        if a > b {
            Self { start: b, end: a }
        } else {
            Self { start: a, end: b }
        }
    }

    pub fn overlaps_with(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SnapSegment {
    perpendicular_dim: i32,
    dim_range: DimRange,
}

impl SnapSegment {
    pub fn new(perpendicular_dim: i32, dim_range: DimRange) -> Self {
        Self {
            perpendicular_dim,
            dim_range,
        }
    }

    pub fn perpendicular_dim(self) -> i32 {
        self.perpendicular_dim
    }

    pub fn dim_range(self) -> DimRange {
        self.dim_range
    }
}
