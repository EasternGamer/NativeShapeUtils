use crate::loader::{read_f64, read_i32};
use crate::traits::{ByteConvertable, Indexable, Positional};
use crate::types::{Flag, Index, Pos};
use std::simd::Simd;

#[derive(Clone, Copy)]
pub struct TrafficLight {
    pub id : Index,
    pub position: Simd<Pos, 2>,
    pub flag : Flag
}

impl Indexable for TrafficLight {
    #[inline]
    fn index(&self) -> Index {
        self.id
    }
    
}

impl Positional for TrafficLight {
    #[inline]
    fn position(&self) -> &Simd<Pos, 2> {
        &self.position
    }
}

impl ByteConvertable for TrafficLight {
    fn from_bytes(byte_array: &[u8]) -> Self {
        let mut index = 0;
        let id = read_i32(byte_array, &mut index);
        let flag = read_i32(byte_array, &mut index);
        let x = read_f64(byte_array, &mut index) as Pos;
        let y = read_f64(byte_array, &mut index) as Pos;
        Self {
            id : id as Index,
            position : Simd::from_array([x, y]),
            flag : flag as Flag
        }
    }
}