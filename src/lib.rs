#![feature(portable_simd)]
#![feature(duration_millis_float)]
#![feature(slice_pattern)]
#![feature(new_uninit)]
#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_slice)]
#![feature(const_for)]

extern crate core;

use std::simd::*;
use std::simd::cmp::SimdPartialOrd;
use std::simd::num::SimdFloat;

use jni::objects::AsJArrayRaw;
use jni::sys::jint;
use rayon::prelude::*;

use objects::boundary::Boundary;
use objects::pathing::node::Node;
use objects::pathing::solver::Solver;
use objects::suburb::Suburb;
use objects::traffic_light::TrafficLight;
use objects::util::parallel_list::ParallelList;
use objects::util::quad_tree::QuadTree;
use objects::util::super_cell::SuperCell;
use traits::Positional;

use crate::objects::pathing::node_type::NodeType;
use crate::traits::Indexable;
use crate::types::{Index, Pos};

pub mod loader;
pub mod types;
pub mod ffi;
pub mod traits;
pub mod objects;
pub mod debug_window;

const MULTIPLIER: Simd<Pos, 2> = Simd::from_array([85295.2, 110948.0]);

pub static mut SOLVER : Option<Solver> = None;
pub static mut SUBURBS: Option<ParallelList<Suburb>> = None;
pub static mut TRAFFIC_LIGHTS : Option<ParallelList<TrafficLight>> = None;
pub static mut NODES : Option<ParallelList<Node>> = None;
pub static mut NODE_TREE : Option<QuadTree<SuperCell<Node>>> = None;

pub fn create_tree<T : Positional + Sync>(values : &[SuperCell<T>]) -> QuadTree<SuperCell<T>> {
    let mut min = Simd::from_array([90f64 as Pos,90f64 as Pos]);
    let mut max = Simd::from_array([-90f64 as Pos,-90f64 as Pos]);
    let mut position;
    for value in values.iter() {
        position = *value.get().position();
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
    let mut tree = QuadTree::new(Boundary {
        corner_max: max,
        corner_min: min,
    }, 0);
    for value in values {
        tree.add_data(value);
    }
    tree
}

pub fn associate_traffic_lights_to_nodes() {
    for traffic_light in get_traffic_lights().as_slice() {
        if let Some(data) = get_node_tree().find_data(traffic_light.position()) {
            let d = data.as_slice();
            NodeType::assign_types(traffic_light, d);
        }
    }
}

#[inline]
pub fn distance(point1: &Simd<Pos, 2>, point2: &Simd<Pos, 2>) -> Pos {
    let displacement = (point1 - point2)*MULTIPLIER;
    (displacement * displacement).reduce_sum().sqrt()
}

#[inline]
pub fn get_suburbs() -> &'static ParallelList<Suburb> {
    unsafe { SUBURBS.as_ref().unwrap() }
}
#[inline]
pub fn get_traffic_lights() -> &'static ParallelList<TrafficLight> {
    unsafe { TRAFFIC_LIGHTS.as_ref().unwrap() }
}
#[inline]
pub fn get_nodes() -> &'static ParallelList<Node> {
    unsafe { NODES.as_ref().unwrap() }
}
#[inline]
pub fn get_solver() -> &'static mut Solver<'static> {
    unsafe { SOLVER.as_mut().unwrap() }
}
#[inline]
pub fn get_node_tree() -> &'static mut QuadTree<'static, SuperCell<Node>> {
    unsafe { NODE_TREE.as_mut().unwrap() }
}
#[inline]
pub fn add_traffic_lights(traffic_lights: ParallelList<TrafficLight>) {
    unsafe { TRAFFIC_LIGHTS = Some(traffic_lights);}
}
#[inline]
pub fn add_nodes(nodes: ParallelList<Node>) {
    unsafe { NODES = Some(nodes);}
}
#[inline]
pub fn add_suburbs(suburbs : ParallelList<Suburb>) {
    unsafe {
        SUBURBS = Some(suburbs);
    }
}
#[inline]
pub fn add_solver(solver: Solver<'static>) {
    unsafe {
        SOLVER = Some(solver);
    }
}
#[inline]
pub fn build_node_tree() {
    unsafe {
        NODE_TREE = Some(create_tree(get_nodes().get_slice()));
    }
}

pub fn get_closest_node(position : &Simd<Pos, 2>) -> Index {
    let mut closest = None;
    let mut current_distance = Pos::MAX;
    if let Some(list) = get_node_tree().find_data(&position) {
        for cell in list {
            let cell_distance = distance(cell.position(), &position);
            if current_distance > cell_distance {
                current_distance = cell_distance;
                closest = Some(cell);
            }
        }
    }
    closest.expect("Could not find cell").get().index() as Index
}

#[inline]
pub fn new_slice<T : Clone>(default : T, size: usize) -> Box<[T]> {
    vec![default; size].into_boxed_slice()
}
#[inline]
pub fn new_pos_slice(size: usize) -> Box<[Pos]> {
    new_slice(Default::default(), size)
}
#[inline]
pub fn new_double_slice(size: usize) -> Box<[f64]> {
    new_slice(0f64, size)
}
#[inline]
pub fn new_usize_slice(size: usize) -> Box<[usize]> {
    new_slice(0usize, size)
}
#[inline]
pub fn new_u8_slice(size: usize) -> Box<[u8]> {
    new_slice(0u8, size)
}

#[inline]
pub fn compute(geometries : &[Suburb], traffic_lights: &[TrafficLight]) -> Vec<(jint, jint)> {
    geometries
        .par_iter()
        .flat_map_iter(|geometry: &Suburb| {
            traffic_lights.iter()
                .filter(|traffic_light| geometry.is_inside(&traffic_light.position))
                .map(|traffic_light: &TrafficLight| (traffic_light.id as jint, geometry.id as jint))
        })
        .collect()
}