#![feature(portable_simd)]
#![feature(duration_millis_float)]
#![feature(slice_pattern)]
#![feature(new_uninit)]
#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_slice)]
extern crate core;

use std::simd::*;
use std::time::Instant;

// This is the interface to the JVM that we'll call the majority of our
// methods on.
use jni::JNIEnv;
use jni::objects::{AsJArrayRaw, JClass, JDoubleArray, JIntArray};
use jni::signature::Primitive::Void;
use jni::signature::ReturnType;
// This is just a pointer. We'll be returning it from our function. We
// can't return one of the objects with lifetime information because the
// lifetime checker won't let us.
use jni::sys::{jboolean, jdouble, jint, jintArray, jsize, jvalue};
use rayon::prelude::*;
use types::{Cost, Pos};
use crate::data::{add_geometry, add_node, add_traffic_light, get_nodes, new_double_slice, new_pos_slice, new_slice, SOLVER};
use crate::node::Connection;
use crate::solver::Solver;
use crate::struts::{BoundarySIMD, Geometry, TrafficLight};

pub mod struts;
pub mod data;
pub mod stop_watch;
pub mod solver;
pub mod helper;
pub mod parallel_list;
pub mod node;
mod types;

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFITraffic_sendTrafficLight<'l>(_env: JNIEnv<'l>, _class: JClass<'l>,  id : jint, x : jdouble, y : jdouble) {
    add_traffic_light(id, x, y);
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFITraffic_getSuburbsInBounds<'l>(env: JNIEnv<'l>, _class: JClass<'l>,
                                                                                                 max_x : jdouble, min_x : jdouble,
                                                                                                 max_y : jdouble, min_y : jdouble,
                                                                                                 limit : jint, debug : jboolean) -> jintArray {
    let start_time = Instant::now();
    let result = data::get_geometry();
    let geometries = result.as_slice();
    let boundary = BoundarySIMD {
        corner_max: Simd::from_array([max_x as Pos, max_y as Pos]),
        corner_min : Simd::from_array([min_x as Pos, min_y as Pos])
    };
    let time_delta_init = (start_time.elapsed().as_nanos() as f64)/1e6;

    let start_filter_time = Instant::now();
    let limit= limit as usize;
    let mut ids = Vec::with_capacity(limit);
    for geometry in geometries {
        if boundary.does_overlap(&geometry.boundary) && ids.len() <= limit {
            ids.push(geometry.id as jint);
        }
    }
    let time_delta_filter = (start_filter_time.elapsed().as_nanos() as f64)/1e6;

    let start_copy_time = Instant::now();
    let indexes = &env.new_int_array(ids.len() as jsize).unwrap();
    env.set_int_array_region(indexes, 0, ids.as_slice()).expect("TODO: panic message");
    let time_delta_copy = (start_copy_time.elapsed().as_nanos() as f64)/1e6;

    if debug == 1u8 {
        println!("Rust Binding - Initialization Time: {time_delta_init}ms");
        println!("Rust Binding - Filter Time: {time_delta_filter}ms");
        println!("Rust Binding - Copy Time: {time_delta_copy}ms");
    }

    indexes.as_jarray_raw()
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFITraffic_getTrafficLightsInBounds<'l>(_env: JNIEnv<'l>, _class: JClass<'l>,
                                                                                                 max_x : jdouble, min_x : jdouble,
                                                                                                 max_y : jdouble, min_y : jdouble,
                                                                                                 limit : jint, debug : jboolean) -> jintArray {
    let start_time = Instant::now();
    let result = data::get_traffic_lights();
    let traffic_lights = result.as_slice();
    let boundary = BoundarySIMD {
        corner_max : Simd::from_array([max_x as Pos, max_y as Pos]),
        corner_min: Simd::from_array([min_x as Pos, min_y as Pos])
    };
    let time_delta_init = (start_time.elapsed().as_nanos() as f64)/1e6;

    let start_filter_time = Instant::now();
    let limit= limit as usize;
    let mut ids = Vec::with_capacity(limit);
    for traffic_light in traffic_lights {
        if boundary.contains(&traffic_light.position) && ids.len() <= limit {
            ids.push(traffic_light.id as jint);
        }
    }
    let time_delta_filter = (start_filter_time.elapsed().as_nanos() as f64)/1e6;

    let start_copy_time = Instant::now();
    let indexes = &_env.new_int_array(ids.len() as jsize).unwrap();
    _env.set_int_array_region(indexes, 0, ids.as_slice()).expect("TODO: panic message");
    let time_delta_copy = (start_copy_time.elapsed().as_nanos() as f64)/1e6;

    if debug == 1u8 {
        println!("Rust Binding - Initialization Time: {time_delta_init}ms");
        println!("Rust Binding - Filter Time: {time_delta_filter}ms");
        println!("Rust Binding - Copy Time: {time_delta_copy}ms");
    }

    indexes.as_jarray_raw()
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFISolver_buildSolver<'l>(_env: JNIEnv<'l>, _class: JClass<'l>) {
    unsafe {
        SOLVER = Some(Solver::new(get_nodes().get_slice(), 0, 0, 100_000))
    }
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi__FFISolver_findPath<'l>(_env: JNIEnv<'l>, _class: JClass<'l>,
                                                                                 source_x : jdouble, source_y : jdouble,
                                                                                 destination_x : jdouble, destination_y : jdouble) {
    
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFITraffic_sendSuburb<'v> (env: JNIEnv<'v>, _class: JClass<'v>,
                                                                                      id : jint, x_points_j: JDoubleArray<'v>, y_points_j: JDoubleArray<'v>,
                                                                                      max_x : jdouble, min_x : jdouble, max_y : jdouble, min_y : jdouble) {
    let size = env.get_array_length(&x_points_j).expect("[Rust Binding] Critical Error! Unable to read array length of points while sending geometry to rust.") as usize;

    let mut x_points= new_double_slice(size);
    let mut y_points = new_double_slice(size);
    env.get_double_array_region(x_points_j, 0, x_points.as_mut()).expect("[Rust Binding] Critical Error! Unable to read array data of x points while sending geometry to rust.");
    env.get_double_array_region(y_points_j, 0, y_points.as_mut()).expect("[Rust Binding] Critical Error! Unable to read array data of y points while sending geometry to rust.");
    let mut x_point_pos = new_pos_slice(size);
    let mut y_point_pos = new_pos_slice(size);
    for index in 0..size {
        x_point_pos[index] = x_points[index] as Pos;
        y_point_pos[index] = y_points[index] as Pos;
    }
    add_geometry(id, max_x, min_x, max_y, min_y, x_point_pos, y_point_pos);
}


#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFISolver_sendRoadNode<'v> (env: JNIEnv<'v>, _class: JClass<'v>,
                                                                                      id : jint, x_point: jdouble, y_point: jdouble,
                                                                                      speed : jdouble, node_type : jint, 
                                                                                      connections_j : JIntArray<'v>, weights_j : JDoubleArray<'v>) {
    let size = env.get_array_length(&connections_j).expect("[Rust Binding] Critical Error! Unable to read array length of points while sending geometry to rust.") as usize;

    let mut connections = new_slice(0i32, size);
    let mut weights = new_slice(0f64, size);
    env.get_int_array_region(connections_j, 0, connections.as_mut()).expect("[Rust Binding] Critical Error! Unable to read array data of connections while sending node to rust.");
    env.get_double_array_region(weights_j, 0, weights.as_mut()).expect("[Rust Binding] Critical Error! Unable to read array data of weights while sending node to rust.");
    let mut edges = new_slice(Connection {index: 0, cost:0f64 as Cost}, size);
    for i in 0..size {
        edges[i] = Connection {
            index: connections[i] as u32,
            cost: weights[i] as Cost
        }
    }
    add_node(id, x_point, y_point, speed, node_type as u8, edges);
}

#[inline]
pub fn compute(geometries : &[Geometry], traffic_lights: &[TrafficLight]) -> Vec<(jint, jint)> {
    //let block_size = geometries.len()/12;
    geometries
        .par_iter()
        //.by_uniform_blocks(block_size)
        .flat_map_iter(|geometry: &Geometry| {
            traffic_lights.iter()
                .filter(|traffic_light| geometry.is_inside(&traffic_light.position))
                .map(|traffic_light: &TrafficLight| (traffic_light.id as jint, geometry.id as jint))
        })
        .collect()
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFITraffic_compute<'l>(mut env: JNIEnv<'l>, class: JClass<'l>, debug: jboolean) {
    println!("Computing...");
    let start_time_pre = Instant::now();

    let method_id = env.get_static_method_id(&class, "receiveTrafficLight", "(II)V").expect("Something went wrong getting static method");
    let temp_geo = data::get_geometry();
    let temp_traffic = data::get_traffic_lights();
    let geometries = temp_geo.as_parallel_slice();
    let traffic_lights = temp_traffic.as_slice();
    let time_delta_init = (start_time_pre.elapsed().as_nanos() as f64) / 1e6;

    let start_time_map = Instant::now();
    let results: Vec<(jint, jint)> = compute(geometries, traffic_lights);
    let time_delta_map = (start_time_map.elapsed().as_nanos() as f64) / 1e6;

    let start_time_push = Instant::now();
    for result in results {
        unsafe {
            env.call_static_method_unchecked(
                &class,
                method_id,
                ReturnType::Primitive(Void),
                &[jvalue { i: result.0 }, jvalue { i: result.1 }]
            ).expect("");
        }
    }
    let time_delta_push = (start_time_push.elapsed().as_nanos() as f64) / 1e6;
    if debug == 1u8 {
        println!("Rust Binding - Initialization Time: {time_delta_init}ms");
        println!("Rust Binding - Map Time: {time_delta_map}ms");
        println!("Rust Binding - Push to Java Time: {time_delta_push}ms");
    }
    println!("Computing complete.");
}
