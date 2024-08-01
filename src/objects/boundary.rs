use std::simd::Simd;
use std::fmt::{Display, Formatter};
use std::simd::cmp::SimdPartialOrd;
use crate::types::Pos;

pub struct Boundary {
    pub corner_max : Simd<Pos, 2>,
    pub corner_min : Simd<Pos, 2>
}

impl Boundary {
    #[inline]
    pub fn contains(&self, point : &Simd<Pos, 2>) -> bool {
        point.simd_le(self.corner_max).all() && point.simd_ge(self.corner_min).all()
    }
    #[inline]
    pub fn does_overlap(&self, other : &Boundary) -> bool {
        self.corner_min.simd_le(other.corner_max).all() && self.corner_max.simd_ge(other.corner_min).all()
    }
}

impl Display for Boundary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Min({}, {}), Max({},{})", self.corner_min[0], self.corner_min[1], self.corner_max[0], self.corner_max[1])
    }
}