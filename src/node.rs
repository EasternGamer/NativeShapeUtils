use core::slice::SlicePattern;
use std::mem::MaybeUninit;
use std::simd::Simd;
use rayon::prelude::ParallelSlice;
use crate::data::distance;
use crate::helper::{ByteConvertable, read_f64, read_i32, skip_f64, skip_i32};
use crate::struts::{HasIndex, RoadNode, SimdPosition, SuperCell, TrafficLight};

#[derive(Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum NodeType {
    AtTrafficLight = 2,
    NearTrafficLight = 1,
    Normal = 0
}
impl NodeType {
    pub fn assign_types(traffic_light : &TrafficLight, nodes : &[&SuperCell<Node>]) {
        nodes.as_parallel_slice().for_each(|node| {
            let mutable_node = node.get_mut();
            match mutable_node.node_type {
                NodeType::Normal => {
                    let distance = distance(&traffic_light.position, mutable_node.position());
                    if distance < 10f64 {
                        mutable_node.node_type = NodeType::AtTrafficLight
                    } else if distance < 20f64 {
                        mutable_node.node_type = NodeType::NearTrafficLight
                    }
                },
                NodeType::NearTrafficLight => {
                    let distance = distance(&traffic_light.position, mutable_node.position());
                    if distance < 10f64 {
                        mutable_node.node_type = NodeType::AtTrafficLight
                    }
                },
                NodeType::AtTrafficLight => {}
            }
        });
    }
}

type Cost = f64;
type Flag = u32;
#[derive(Clone)]
pub struct Connection {
    pub index : u32,
    pub cost : Cost
}
unsafe impl Send for Connection {}
unsafe impl Sync for Connection {}
pub struct Node {
    pub index : u32,
    cost : Cost,
    previous : u32,
    connection_len: u16,
    pub position : Simd<f64, 2>,
    pub flag : Flag,
    pub node_type : NodeType,
    pub connections : Box<[Connection]>
}

impl SimdPosition for Node {
    #[inline]
    fn position(&self) -> &Simd<f64, 2> {
        &self.position
    }
}
impl HasIndex for Node {
    fn index(&self) -> usize {
        self.index as usize
    }
}

impl Node {
    #[inline]
    pub fn new(index: u32, position: Simd<f64, 2>, connections: Box<[Connection]>) -> Self {
        Self {
            index,
            position,
            cost : -1f64,
            flag : u32::MAX,
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
    pub fn get_cost(&self) -> f64 {
        self.cost
    }
    #[inline(always)]
    pub fn is_lower_cost(&self, new_cost : f64) -> bool {
        self.get_cost() < new_cost
    }
    #[inline(always)]
    pub fn has_visited(&self) -> bool {
        self.cost != -1f64
    }
    #[inline(always)]
    pub fn get_previous(&self) -> u32 {
        self.previous
    }
    #[inline(always)]
    pub fn reset(&mut self) {
        self.cost = f64::MAX;
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
                    index: read_i32(byte_array, &mut index) as u32,
                    cost: read_f64(byte_array, &mut index)
                });
            }

            Node::new(
                id as u32,
                Simd::from_array([x, y]),
                tmp_indices.assume_init()
            )
        }
    }
}