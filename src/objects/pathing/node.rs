use crate::loader::{read_f64, read_i32};
use crate::objects::pathing::connection::Connection;
use crate::objects::pathing::node_type::NodeType;
use crate::traits::{ByteConvertable, Indexable, Positional};
use crate::types::{Cost, Flag, Index, Pos};
use core::slice::SlicePattern;
use std::mem::MaybeUninit;
use std::simd::Simd;

pub struct Node {
    pub index : Index,
    pub flag : Flag,
    pub node_type : NodeType,
    pub position : Simd<Pos, 2>,
    pub connections : Box<[Connection]>
}
impl Node {
    #[inline]
    pub fn new(index: Index, position: Simd<Pos, 2>, connections: Box<[Connection]>) -> Self {
        Self {
            index,
            position,
            flag : Flag::MAX,
            node_type : NodeType::Normal,
            connections
        }
    }

    #[inline(always)]
    pub fn get_connections(&self) -> &'_[Connection] {
        self.connections.as_slice()
    }
}

unsafe impl Send for Node {}

unsafe impl Sync for Node {}

impl ByteConvertable for Node {
    fn from_bytes(byte_array: &[u8]) -> Self {
        let mut index = 0;
        let id = read_i32(byte_array, &mut index);
        let x = read_f64(byte_array, &mut index);
        let y = read_f64(byte_array, &mut index);
        let speed = read_i32(byte_array, &mut index) as u16;
        let connected_indices_size = read_i32(byte_array, &mut index) as usize;
        let mut tmp_indices = Box::new_uninit_slice(connected_indices_size);
        unsafe {
            for index_c in 0..connected_indices_size {
                tmp_indices[index_c] = MaybeUninit::new(Connection {
                    index: read_i32(byte_array, &mut index) as Index,
                    cost: read_f64(byte_array, &mut index) as Cost,
                    speed
                });
            }

            Node::new(
                id as Index,
                Simd::from_array([x as Pos, y as Pos]),
                tmp_indices.assume_init()
            )
        }
    }
}

impl Clone for Node {
    fn clone(&self) -> Self {
        Self {
            index : self.index,
            flag : self.flag,
            position : self.position,
            node_type: self.node_type,
            connections: self.connections.clone(),
        }
    }
}

impl Positional for Node {
    #[inline]
    fn position(&self) -> &Simd<Pos, 2> {
        &self.position
    }
}

impl Indexable for Node {
    fn index(&self) -> Index {
        self.index
    }
}