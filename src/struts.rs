use core::slice::SlicePattern;
use std::cell::UnsafeCell;
use std::fmt::{Display, Formatter};
use std::mem::MaybeUninit;
use std::ops::{Div, Sub};
use std::simd::prelude::{SimdFloat, SimdPartialOrd};
use std::simd::Simd;

use crossbeam::atomic::AtomicCell;

use crate::helper::{ByteConvertable, read_f64, read_i32, skip_i32};
use crate::node::NodeType;

const MAX_CAPACITY : usize = 1024;
const MAX_DEPTH : i8 = 32; 

pub trait SimdPosition {
    fn position(&self) -> &Simd<f64, 2>;
}
pub trait HasIndex {
    fn index(&self) -> usize;
}

#[derive(Copy, Clone)]
pub struct NodeData {
    pub cost : f64,
    pub previous : u32,
    pub connection_len: u16,
    pub visited : bool
}

pub struct Node<T> {
    pub value : T,
    pub connections : Box<[Connection]>,
    pub node_type : UnsafeCell<NodeType>,
    pub flag: u32,
    data : AtomicCell<NodeData>,
}
impl <T : SimdPosition> SimdPosition for Node<T> {
    #[inline]
    fn position(&self) -> &Simd<f64, 2> {
        self.value.position()
    }
}
impl <T: HasIndex> HasIndex for Node<T> {
    fn index(&self) -> usize {
        self.value.index()
    }
}

impl <T : HasIndex> Node<T> {
    #[inline]
    pub fn new(value : T, flag : u32, connections: Box<[Connection]>) -> Self {
        Self {
            value,
            connections,
            node_type : UnsafeCell::new(NodeType::Normal),
            flag,
            data : AtomicCell::new(
                NodeData {
                    cost : f64::MAX,
                    previous : u32::MAX,
                    connection_len: u16::MAX,
                    visited : false
                }
            )
        }
    }

    #[inline(always)]
    pub fn get_connections(&self) -> &'_[Connection] {
        self.connections.as_slice()
    }
    #[inline(always)]
    pub fn get_connection_len(&self) -> u16 {
        self.data.load().connection_len
    }
    
    #[inline(always)]
    pub fn check_updated_and_save(&self, new_cost : f64, index : u32, length : u16) -> bool {
        if self.data.load().cost > new_cost {
            self.data.store(NodeData {
                cost: new_cost,
                previous : index,
                connection_len : length,
                visited : true
            });
            return true;
        }
        false
    }
    #[inline(always)]
    pub fn set_cost(&self, new_cost : f64) {
        let previous = self.data.load();
        self.data.store(NodeData {
            cost: new_cost,
            previous: previous.previous,
            connection_len: previous.connection_len,
            visited : previous.visited
        })
    }
    #[inline(always)]
    pub fn get_cost(&self) -> f64 {
        self.data.load().cost
    }
    #[inline(always)]
    pub fn is_lower_cost(&self, new_cost : f64) -> bool {
        self.get_cost() < new_cost
    }
    #[inline(always)]
    pub fn has_visited(&self) -> bool {
        self.data.load().visited
    }
    #[inline(always)]
    pub fn get_previous(&self) -> u32 {
        self.data.load().previous
    }
    #[inline(always)]
    pub fn reset(&self) {
        self.data.store(NodeData {
            cost: f64::MAX,
            previous : u32::MAX,
            connection_len : 0u16,
            visited : false
        });
    }
}
unsafe impl <T> Send for Node<T> {}
unsafe impl <T> Sync for Node<T> {}

#[derive(Clone)]
pub struct Connection {
    pub index : u32,
    pub cost : f64
}

unsafe impl Send for Connection {}
unsafe impl Sync for Connection {}

#[derive(Clone)]
pub struct RoadNode {
    pub index : usize,
    pub position : Simd<f64, 2>,
    pub speed : f64,
    pub node_type : u8
}

impl HasIndex for RoadNode {
    fn index(&self) -> usize {
        self.index
    }
}
impl SimdPosition for RoadNode {
    #[inline]
     fn position(&self) -> &Simd<f64, 2> {
        &self.position
    }
}

pub struct QuadTree<'life, T : SimdPosition> {
    pub top_left : Box<Option<QuadTree<'life, T>>>,
    pub top_right : Box<Option<QuadTree<'life, T>>>,
    pub bottom_left : Box<Option<QuadTree<'life, T>>>,
    pub bottom_right : Box<Option<QuadTree<'life, T>>>,
    pub depth : i8,
    pub has_children : bool,
    pub boundary : BoundarySIMD,
    pub data : Vec<&'life T>
}


impl <'life, T : SimdPosition> QuadTree<'life, T> {
    #[inline]
    pub fn new(boundary_simd: BoundarySIMD, depth : i8) -> QuadTree<'life, T> {
        QuadTree {
            top_left : Box::new(None),
            top_right : Box::new(None),
            bottom_left : Box::new(None),
            bottom_right : Box::new(None),
            has_children : false,
            boundary : boundary_simd,
            depth,
            data : Vec::new()
        }
    }
    
    #[inline]
    pub fn contains(&self, point : &Simd<f64, 2>) -> bool {
        self.boundary.contains(point)
    }
    
    
    fn find_data_top_left(&self, point : &Simd<f64, 2>) -> Option<&Vec<&T>> {
        match self.top_left.as_ref() { 
            Some(child_node) => {
                child_node.find_data(point)
            }
            None => None
        }
    }
    fn find_data_top_right(&self, point : &Simd<f64, 2>) -> Option<&Vec<&T>> {
        match self.top_right.as_ref() {
            Some(child_node) => {
                child_node.find_data(point)
            }
            None => None
        }
    }

    fn find_data_bottom_right(&self, point : &Simd<f64, 2>) -> Option<&Vec<&T>> {
        match self.bottom_right.as_ref() {
            Some(child_node) => {
                child_node.find_data(point)
            }
            None => None
        }
    }
    fn find_data_bottom_left(&self, point : &Simd<f64, 2>) -> Option<&Vec<&T>> {
        match self.bottom_left.as_ref() {
            Some(child_node) => {
                child_node.find_data(point)
            }
            None => None
        }
    }
    
    pub fn find_data(&self, point : &Simd<f64, 2>) -> Option<&Vec<&T>> {
        if self.contains(point) {
            return if self.has_children {
                match self.find_data_top_left(point) {
                    Some(data) => Some(data),
                    None => match self.find_data_top_right(point) {
                        Some(data) => Some(data),
                        None => match self.find_data_bottom_left(point) {
                            Some(data) => Some(data),
                            None => match self.find_data_bottom_right(point) {
                                Some(data) => Some(data),
                                None => panic!("Point was inside quad tree, but not inside any child of the quad tree. This should *never* happen.")
                            }
                        }
                    }
                }
            } else {
                Some(&self.data)
            }
        }
        None
    }
    
    #[inline]
    fn get_top_left_node(&mut self) -> &mut QuadTree<'life, T> {
        self.top_left.as_mut().as_mut().expect("Had Children, but no top left tree")
    }
    #[inline]
    fn get_top_right_node(&mut self) -> &mut QuadTree<'life, T> {
        self.top_right.as_mut().as_mut().expect("Had Children, but no top right tree")
    }
    #[inline]
    fn get_bottom_left_node(&mut self) -> &mut QuadTree<'life, T> {
        self.bottom_left.as_mut().as_mut().expect("Had Children, but no bottom left tree")
    }
    #[inline]
    fn get_bottom_right_node(&mut self) -> &mut QuadTree<'life, T> {
        self.bottom_right.as_mut().as_mut().expect("Had Children, but no bottom right tree")
    }

    pub fn add_data(&mut self, data : &'life T) -> bool {
        let position = data.position();
        if self.contains(position) {
            if self.has_children {
                let top_left = self.get_top_left_node();
                if top_left.contains(position) {
                    return top_left.add_data(data);
                }
                let top_right = self.get_top_right_node();
                if top_right.contains(position) {
                    return top_right.add_data(data);
                }
                let bottom_left = self.get_bottom_left_node();
                if bottom_left.contains(position) {
                    return bottom_left.add_data(data);
                }
                let bottom_right = self.get_bottom_right_node();
                if bottom_right.contains(position) {
                    return bottom_right.add_data(data)
                }
                panic!("Trying to add a point that was inside a tree that had children, but it was not inside any children!");
            } else {
                return if self.data.len() < MAX_CAPACITY {
                    self.data.push(data);
                    true
                } else if self.depth < MAX_DEPTH {
                    self.sub_divide();
                    self.add_data(data)
                } else {
                    false
                };
            }
        }
        false
    }

    pub fn sub_divide(&mut self) {
        let corner_min_simd = &self.boundary.corner_min;
        let corner_max_simd = &self.boundary.corner_max;
        let center_simd = &corner_max_simd.sub((corner_max_simd - corner_min_simd).div(Simd::from_array([2f64, 2f64]))) ;
        let corner_min_array = corner_min_simd.as_array();
        let center_array = center_simd.as_array();
        let corner_max_array = corner_max_simd.as_array();

        let new_depth = self.depth + 1;
        let top_left_boundary = BoundarySIMD {
            corner_min: Simd::from_array([corner_min_array[0], center_array[1]]),
            corner_max: Simd::from_array([center_array[0], corner_max_array[1]])
        };
        let bottom_right_boundary = BoundarySIMD {
            corner_min: Simd::from_array([center_array[0], corner_min_array[1]]),
            corner_max: Simd::from_array([corner_max_array[0], center_array[1]])
        };
        let top_right_boundary = BoundarySIMD {
            corner_min: *center_simd,
            corner_max: *corner_max_simd
        };
        let bottom_left_boundary = BoundarySIMD {
            corner_min: *corner_min_simd,
            corner_max: *center_simd
        };
        let mut top_left = QuadTree::new(top_left_boundary, new_depth);
        let mut top_right = QuadTree::new(top_right_boundary, new_depth);
        let mut bottom_left = QuadTree::new(bottom_left_boundary, new_depth);
        let mut bottom_right = QuadTree::new(bottom_right_boundary, new_depth);
        
        
        for data_object in &self.data {
            if top_left.add_data(*data_object) {
                continue;
            }
            if top_right.add_data(*data_object) {
                continue;
            }
            if bottom_left.add_data(*data_object) {
                continue;
            }
            if !bottom_right.add_data(*data_object) {
                panic!("Failed to add data to children during sub division!");
            }
        }

        self.top_left = Box::new(Some(top_left));
        self.top_right = Box::new(Some(top_right));
        self.bottom_left = Box::new(Some(bottom_left));
        self.bottom_right = Box::new(Some(bottom_right));
        
        self.has_children = true;
    }
}

pub struct BoundarySIMD {
    pub corner_max : Simd<f64, 2>,
    pub corner_min : Simd<f64, 2>
}

impl BoundarySIMD {
    #[inline]
    pub fn contains(&self, point : &Simd<f64, 2>) -> bool {
        point.simd_le(self.corner_max).all() && point.simd_ge(self.corner_min).all()
    }
    #[inline]
    pub fn does_overlap(&self, other : &BoundarySIMD) -> bool {
        self.corner_min.simd_le(other.corner_max).all() && self.corner_max.simd_ge(other.corner_min).all()
    }
}

impl Display for BoundarySIMD {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Min({}, {}), Max({},{})", self.corner_min[0], self.corner_min[1], self.corner_max[0], self.corner_max[1])
    }
}

pub struct Geometry {
    pub id : usize,
    pub boundary : BoundarySIMD,
    pub x_points : Box<[f64]>,
    pub y_points : Box<[f64]>,
}
/*
impl Clone for Geometry {
    fn clone(&self) -> Self {
        Geometry {
            x_points: Box::new([]),
            y_points: Box::new([]),
            id: self.id,
            boundary: BoundarySIMD {
                corner_max : Simd::from_array([0.0, 0.0]),
                corner_min: Simd::from_array([0.0, 0.0])
            },
        }
    }
}*/

impl Geometry {
    /**
     * Taken from and translated from the Even-Odd rule algorithm found on Wikipedia, using SIMD where possible.
     * <br>https://en.wikipedia.org/wiki/Even-odd_rule</br>
     */
    #[inline]
    pub fn is_inside(&self, pos : &Simd<f64, 2>) -> bool {
        if self.boundary.contains(pos) {
            self.is_inside_no_bound_check(pos)
        } else {
            false
        }
    }
    
    #[inline]
    pub fn is_inside_no_bound_check(&self, pos : &Simd<f64, 2>) -> bool {
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
            if (y < ay) != (y < by) && (xsimd.sub(asimd).reduce_product() - ysimd.sub(asimd).reduce_product() < 0f64) != (by < ay) {
                inside = !inside;
            }
            xsimd = Simd::from_array([x, ay]);
            ysimd = Simd::from_array([ax, y]);
            by = ay;
        }
        inside
    }
}
#[derive(Clone, Copy)]
pub struct TrafficLight {
    pub id : usize,
    pub position: Simd<f64, 2>
}

impl HasIndex for TrafficLight {
    #[inline]
    fn index(&self) -> usize {
        self.id
    }
    
}

impl SimdPosition for TrafficLight {
    #[inline]
    fn position(&self) -> &Simd<f64, 2> {
        &self.position
    }
}
impl ByteConvertable for TrafficLight {
    fn from_bytes(byte_array: &[u8]) -> Self {
        let mut index = 0;
        let id = read_i32(byte_array, &mut index);
        skip_i32(&mut index);
        let x = read_f64(byte_array, &mut index);
        let y = read_f64(byte_array, &mut index);
        Self {
            id : id as usize,
            position : Simd::from_array([x, y])
        }
    }
}
impl ByteConvertable for Node<RoadNode> {
    fn from_bytes(byte_array: &[u8]) -> Self {
        let mut index = 0;
        let id = read_i32(byte_array, &mut index);
        let x = read_f64(byte_array, &mut index);
        let y = read_f64(byte_array, &mut index);
        let speed = read_f64(byte_array, &mut index);
        let node_type = read_i32(byte_array, &mut index) as u8;
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
                RoadNode { 
                    index: id as usize, 
                    position: Simd::from_array([x, y]), 
                    speed, 
                    node_type, 
                }, 
                u32::MAX, 
                tmp_indices.assume_init()
            )
        }
    }
}