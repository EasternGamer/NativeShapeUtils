use std::simd::Simd;
use crate::types::Pos;

pub trait Positional {
    fn position(&self) -> &Simd<Pos, 2>;
}

pub trait Indexable {
    fn index(&self) -> usize;
}

pub trait ByteConvertable {
    fn from_bytes(byte_array : &[u8]) -> Self;
}