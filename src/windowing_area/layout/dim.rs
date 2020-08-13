use std::{fmt::Debug, marker::PhantomData};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rect<T: Dim> {
    pub x: T,
    pub y: T,
    pub w: T,
    pub h: T,
}

pub type RectF = Rect<f32>;
pub type RectI = Rect<i32>;

pub trait Dim: Clone + Copy + PartialEq + Debug + Send + Sync {}
pub trait ContinuousDim: Dim {}
pub trait DiscreteDim: Dim + Eq {}

impl Dim for f32 {}
impl ContinuousDim for f32 {}

impl Dim for i32 {}
impl DiscreteDim for i32 {}

pub trait Dir: Clone + Copy + PartialEq + Eq + Debug + Send + Sync {
    type PerpendicularDir: Dir;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Horizontal {}
impl Dir for Horizontal {
    type PerpendicularDir = Vertical;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vertical {}
impl Dir for Vertical {
    type PerpendicularDir = Horizontal;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DimRange<T: Dim, D: Dir> {
    lower: T,
    upper: T,
    _marker: PhantomData<D>,
}

impl<T: Dim + PartialOrd, D: Dir> DimRange<T, D> {
    pub fn new(a: T, b: T) -> Self {
        if a > b {
            Self {
                lower: b,
                upper: a,
                _marker: PhantomData,
            }
        } else {
            Self {
                lower: a,
                upper: b,
                _marker: PhantomData,
            }
        }
    }

    pub fn overlaps_with(self, other: Self) -> bool {
        self.lower < other.upper && other.lower < self.upper
    }
}
