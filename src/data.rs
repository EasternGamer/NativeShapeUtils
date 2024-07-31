use std::ptr::addr_of;
use std::simd::prelude::{SimdFloat, SimdPartialOrd};
use std::simd::Simd;

use heapless::Vec;
use jni::sys::{jdouble, jint};
use kiss3d::nalgebra::SimdBool;
use rayon::prelude::*;

use crate::solver::Solver;
use crate::struts::{BoundarySIMD, Connection, Geometry, Node, QuadTree, RoadNode, SimdPosition, TrafficLight};

pub static mut GEOMETRIES : Vec<Geometry, 27922> = Vec::new();

pub static mut TRAFFIC_LIGHTS : Vec<TrafficLight, 14645> = Vec::new();

pub static mut NODES : std::vec::Vec<Node<RoadNode>> = std::vec::Vec::new();

pub static mut SOLVER : Option<Solver<RoadNode>> = None;

pub static mut NODE_TREE : Option<QuadTree<Node<RoadNode>>> = None;

pub fn create_tree<T : SimdPosition + Sync>(values : &[T]) -> QuadTree<T> {
    let mut min = Simd::from_array([90f64,90f64]);
    let mut max = Simd::from_array([-90f64,-90f64]);
    let mut position;
    for value in values.iter() {
        position = *value.position();
        if min.simd_gt(position).any() {
            min = min.simd_min(position);
        }
        if max.simd_lt(position).any() {
            max = max.simd_max(position);
        }
    }
    let max_x = max[0];
    let max_y = max[1];
    let min_x = min[0];
    let min_y = min[1];
    println!("Boundary: ({max_x},{max_y}),({min_x},{min_y})");
    let mut tree = QuadTree::new(BoundarySIMD {
        corner_max: max,
        corner_min: min,
    }, 0);
    for value in values {
        tree.add_data(value);
    }
    tree
}
const MULTIPLIER: Simd<f64, 2> = Simd::from_array([85.2952, 110.9480]);
#[inline]
pub fn distance(point1: &Simd<f64, 2>, point2: &Simd<f64, 2>) -> f64 {
    let displacement = (point1 - point2)*MULTIPLIER;
    (displacement * displacement).reduce_sum().sqrt()
}
#[inline]
pub fn get_geometry() -> &'static Vec<Geometry, 27922> {
    unsafe { &*addr_of!(GEOMETRIES) }
}

#[inline]
pub fn get_traffic_lights() -> &'static Vec<TrafficLight, 14645> {
    unsafe { &*addr_of!(TRAFFIC_LIGHTS) }
}

#[inline]
pub fn get_nodes() -> &'static std::vec::Vec<Node<RoadNode>> {
    unsafe { &*addr_of!(NODES) }
}

#[inline]
pub fn get_solver() -> &'static mut Solver<'static, RoadNode> {
    unsafe { SOLVER.as_mut().unwrap() }
}

#[inline]
pub fn get_node_tree() -> &'static mut QuadTree<'static, Node<RoadNode>> {
    unsafe { NODE_TREE.as_mut().unwrap() }
}

#[inline]
pub fn add_node(id : jint, x : jdouble, y : jdouble, speed : jdouble, node_type : u8, connections : Box<[Connection]>) {
    let index = id as usize;
    let node = Node::new(
        RoadNode { 
            index, 
            position: Simd::from_array([x, y]), 
            speed, 
            node_type 
        },
        u32::MAX,
        connections
    );
    unsafe {
        NODES.push(node);
    }
    
}

#[inline]
pub fn add_geometry(id : jint, max_x : jdouble, min_x : jdouble, max_y : jdouble, min_y : jdouble, x_points : Box<[f64]>, y_points : Box<[f64]>) {
    let id = id as usize;
    let boundary = BoundarySIMD {
        corner_max: Simd::from_array([max_x, max_y]),
        corner_min: Simd::from_array([min_x, min_y])
    };
    unsafe {
        let _ = GEOMETRIES.push(
            Geometry { 
                id, 
                x_points, 
                y_points, 
                boundary 
            }); 
    }
}

#[inline]
pub fn add_traffic_light (id : jint, x : jdouble, y : jdouble) {
    let id = id as usize;
    unsafe {
        let _ = TRAFFIC_LIGHTS.push(TrafficLight {
            id,
            position: Simd::from_array([x, y])
        });
    }
}

#[inline]
pub fn new_slice<T : Clone>(default : T, size: usize) -> Box<[T]> {
    vec![default; size].into_boxed_slice()
}

#[inline]
pub fn new_double_slice(size: usize) -> Box<[f64]> {
    new_slice(0f64, size)
}

#[allow(dead_code)]
#[inline]
pub fn new_usize_slice(size: usize) -> Box<[usize]> {
    new_slice(0usize, size)
}

#[allow(dead_code)]
#[inline]
pub fn new_u8_slice(size: usize) -> Box<[u8]> {
    new_slice(0u8, size)
}


