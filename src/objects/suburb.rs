use std::simd::Simd;
use core::slice::SlicePattern;
use std::ops::Sub;
use std::simd::num::SimdFloat;
use crate::loader::{read_f64, read_i32, skip_string};
use crate::new_pos_slice;
use crate::objects::boundary::Boundary;
use crate::traits::{ByteConvertable, Indexable};
use crate::types::{Index, Pos};

pub struct Suburb {
    pub id : Index,
    pub boundary : Boundary,
    pub x_points : Box<[Pos]>,
    pub y_points : Box<[Pos]>,
}

impl Suburb {
    /**
     * Taken from and translated from the Even-Odd rule algorithm found on Wikipedia, using SIMD where possible.
     * <br>https://en.wikipedia.org/wiki/Even-odd_rule</br>
     */
    #[inline]
    pub fn is_inside(&self, pos : &Simd<Pos, 2>) -> bool {
        if self.boundary.contains(pos) {
            self.is_inside_no_bound_check(pos)
        } else {
            false
        }
    }
    
    #[inline]
    pub fn is_inside_no_bound_check(&self, pos : &Simd<Pos, 2>) -> bool {
        let pos_array = pos.as_array();
        let x = pos_array[0];
        let y = pos_array[1];
        let bound_x = self.x_points.as_slice();
        let bound_y = self.y_points.as_slice();
        let point_count = bound_x.len();
        let mut by = bound_y[0];
        let mut ax;
        let mut ay;
        let mut xsimd = Simd::from_array([x, by]);
        let mut ysimd = Simd::from_array([bound_x[0], y]);
        let mut asimd;
        let mut inside = false;
        for eb in 1..point_count {
            ax = bound_x[eb];
            ay = bound_y[eb];
            asimd = Simd::from_array([ax, ay]);
            if (y < ay) != (y < by) && (xsimd.sub(asimd).reduce_product() - ysimd.sub(asimd).reduce_product() < (0f64 as Pos)) != (by < ay) {
                inside = !inside;
            }
            xsimd = Simd::from_array([x, ay]);
            ysimd = Simd::from_array([ax, y]);
            by = ay;
        }
        inside
    }
}

impl Indexable for Suburb {
    fn index(&self) -> usize { self.id as usize }
}

impl ByteConvertable for Suburb {
    fn from_bytes(byte_array: &[u8]) -> Self {
        let mut index = 0;
        let id = read_i32(byte_array, &mut index);
        let name_length = read_i32(byte_array, &mut index) as usize;
        let coordinate_length = read_i32(byte_array, &mut index) as usize;
        let min_x = read_f64(byte_array, &mut index);
        let min_y = read_f64(byte_array, &mut index);
        let max_x = read_f64(byte_array, &mut index);
        let max_y = read_f64(byte_array, &mut index);
        skip_string(&mut index, name_length);
        let mut x_points = new_pos_slice(coordinate_length);
        let mut y_points = new_pos_slice(coordinate_length);
        for index_c in 0..coordinate_length {
            x_points[index_c] = read_f64(byte_array, &mut index) as Pos;
            y_points[index_c] = read_f64(byte_array, &mut index) as Pos;
        }
        let id = id as usize;
        let boundary = Boundary {
            corner_max: Simd::from_array([max_x as Pos, max_y as Pos]),
            corner_min: Simd::from_array([min_x as Pos, min_y as Pos])
        };
        Suburb {
            id,
            x_points,
            y_points,
            boundary
        }
    }
}