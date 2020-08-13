use std::{fmt::Debug, marker::PhantomData, ops::Add};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rect<T: Dim> {
    pub x: T,
    pub y: T,
    pub w: T,
    pub h: T,
}

pub type RectF = Rect<f32>;
pub type RectI = Rect<i32>;

impl<T: Dim> Rect<T> {
    pub fn pos(self) -> Point<T> {
        Point {
            x: self.x,
            y: self.y,
        }
    }

    pub fn size(self) -> Size<T> {
        Size {
            w: self.w,
            h: self.h,
        }
    }
}

impl<T> Rect<T>
where
    T: Dim + PartialOrd + Add<Output = T>,
{
    pub fn range_h(self) -> DimRange<T, Horizontal> {
        DimRange::new(self.x, self.x + self.w)
    }

    pub fn range_v(self) -> DimRange<T, Vertical> {
        DimRange::new(self.y, self.y + self.h)
    }

    pub fn range<D: Dir>(self) -> DimRange<T, D> {
        let lower = self.pos().dim::<D>();
        let upper = lower + self.size().dim::<D>();
        DimRange::new(lower, upper)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Point<T: Dim> {
    pub x: T,
    pub y: T,
}

pub type PointF = Point<f32>;
pub type PointI = Point<i32>;

impl<T: Dim> Point<T> {
    pub fn from_array(p: [T; 2]) -> Self {
        let [x, y] = p;
        Self { x, y }
    }
    pub fn into_array(self) -> [T; 2] {
        [self.x, self.y]
    }
    pub fn dim<D: Dir>(self) -> T {
        <D as Dir>::dim_from_point(self)
    }
}

impl<T: Dim> From<[T; 2]> for Point<T> {
    fn from(p: [T; 2]) -> Self {
        Self::from_array(p)
    }
}

impl<T: Dim> From<Point<T>> for [T; 2] {
    fn from(p: Point<T>) -> Self {
        p.into_array()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Size<T: Dim> {
    pub w: T,
    pub h: T,
}

pub type SizeF = Size<f32>;
pub type SizeI = Size<i32>;

impl<T: Dim> Size<T> {
    pub fn from_array(s: [T; 2]) -> Self {
        let [w, h] = s;
        Self { w, h }
    }
    pub fn into_array(self) -> [T; 2] {
        [self.w, self.h]
    }
    pub fn dim<D: Dir>(self) -> T {
        <D as Dir>::dim_from_size(self)
    }
}

impl<T: Dim> From<[T; 2]> for Size<T> {
    fn from(s: [T; 2]) -> Self {
        Self::from_array(s)
    }
}

impl<T: Dim> From<Size<T>> for [T; 2] {
    fn from(s: Size<T>) -> Self {
        s.into_array()
    }
}

pub trait Dim: Clone + Copy + PartialEq + Debug + Send + Sync {}
pub trait ContinuousDim: Dim {}
pub trait DiscreteDim: Dim + Eq {}

impl Dim for f32 {}
impl ContinuousDim for f32 {}

impl Dim for i32 {}
impl DiscreteDim for i32 {}

pub trait Dir: Clone + Copy + PartialEq + Eq + Debug + Send + Sync {
    type PerpendicularDir: Dir;
    fn dim_from_point<T: Dim>(p: Point<T>) -> T;
    fn dim_from_size<T: Dim>(s: Size<T>) -> T;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Horizontal {}
impl Dir for Horizontal {
    type PerpendicularDir = Vertical;

    fn dim_from_point<T: Dim>(p: Point<T>) -> T {
        p.x
    }

    fn dim_from_size<T: Dim>(s: Size<T>) -> T {
        s.w
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vertical {}
impl Dir for Vertical {
    type PerpendicularDir = Horizontal;

    fn dim_from_point<T: Dim>(p: Point<T>) -> T {
        p.y
    }

    fn dim_from_size<T: Dim>(s: Size<T>) -> T {
        s.h
    }
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
