use std::simd::Simd;
use std::mem::MaybeUninit;
use core::slice::SlicePattern;
use crate::loader::{read_f64, read_i32, skip_f64, skip_i32};
use crate::objects::pathing::connection::Connection;
use crate::objects::pathing::node_type::NodeType;
use crate::traits::{ByteConvertable, Indexable, Positional};
use crate::types::{Cost, Flag, Index, Pos};

pub struct Node {
    pub index : Index,
    cost : Cost,
    pub flag : Flag,
    previous : u32,
    connection_len: u16,
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
            cost : Cost::MAX,
            flag : Flag::MAX,
            previous : u32::MAX,
            connection_len: 0,
            node_type : NodeType::Normal,
            connections
        }
    }

    #[inline(always)]
    pub fn get_connections(&self) -> &'_[Connection] {
        self.connections.as_slice()
    }
    #[inline(always)]
    pub fn get_connection_len(&self) -> u16 {
        self.connection_len
    }

    #[inline(always)]
    pub fn check_updated_and_save(&mut self, new_cost : Cost, index : u32, length : u16) -> bool {
        if self.cost > new_cost {
            self.cost = new_cost;
            self.previous = index;
            self.connection_len = length;
            return true;
        }
        false
    }
    #[inline(always)]
    pub fn set_cost(&mut self, new_cost : Cost) {
        self.cost = new_cost;
    }
    #[inline(always)]
    pub fn get_cost(&self) -> Cost {
        self.cost
    }
    #[inline(always)]
    pub fn is_lower_cost(&self, new_cost : Cost) -> bool {
        self.get_cost() < new_cost
    }
    #[inline(always)]
    pub fn has_visited(&self) -> bool {
        self.cost != Cost::MAX
    }
    #[inline(always)]
    pub fn get_previous(&self) -> u32 {
        self.previous
    }
    #[inline(always)]
    pub fn reset(&mut self) {
        self.cost = Cost::MAX;
        self.previous = u32::MAX;
        self.connection_len = 0u16;
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
        skip_f64(&mut index);
        skip_i32(&mut index);
        let connected_indices_size = read_i32(byte_array, &mut index) as usize;
        let mut tmp_indices = Box::new_uninit_slice(connected_indices_size);
        unsafe {
            for index_c in 0..connected_indices_size {
                tmp_indices[index_c] = MaybeUninit::new(Connection {
                    index: read_i32(byte_array, &mut index) as Index,
                    cost: read_f64(byte_array, &mut index) as Cost
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
            cost : self.cost,
            flag : self.flag,
            position : self.position,
            previous : self.previous,
            connection_len: self.connection_len,
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
    fn index(&self) -> usize {
        self.index as usize
    }
}