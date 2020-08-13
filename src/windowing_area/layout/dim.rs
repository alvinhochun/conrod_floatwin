use std::fmt::Debug;

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
