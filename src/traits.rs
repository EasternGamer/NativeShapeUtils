use std::simd::Simd;
use crate::types::{Index, Pos};

pub trait Positional {
    fn position(&self) -> &Simd<Pos, 2>;
}

pub trait Indexable {
    fn index(&self) -> Index;
}

pub trait ByteConvertable {
    fn from_bytes(byte_array : &[u8]) -> Self;
}