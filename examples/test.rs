#![feature(portable_simd)]
#![feature(duration_millis_float)]
#![feature(slice_pattern)]
#![feature(new_uninit)]
#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_slice)]

use std::time::Instant;

use jni::sys::jint;
use crate::lib::*;
use crate::lib::debug_window::start_window;
use crate::lib::debug_window::start_search;
use crate::lib::objects::pathing::solver::Solver;
use crate::lib::objects::util::stop_watch::StopWatch;
use crate::lib::objects::pathing::node_type::SearchMethod;
use crate::loader::*;

#[path = "../src/lib.rs"]
mod lib;

pub fn computation() {
    println!("Computing...");
    let mut stop_watch = StopWatch::start();

    add_traffic_lights(FileLoader::new("cache\\traffic.dat").load().unwrap());
    stop_watch.elapsed_store("Traffic Data to Memory");
    add_suburbs(FileLoader::new("cache\\suburb.dat").load().unwrap());
    stop_watch.elapsed_store("Suburb Data to Memory");
    add_nodes(FileLoader::new("cache\\nodes.dat").load_parallel().unwrap());
    stop_watch.elapsed_store("Node Data To Memory");
    
    println!("Completed reading nodes");
    let temp_geo = get_suburbs();
    let temp_traffic = get_traffic_lights();
    let geometries = temp_geo.as_slice();
    let traffic_lights = temp_traffic.as_slice();
    stop_watch.elapsed_store("Memory Read Data");

    let start_time_map = Instant::now();
    let _results: Vec<(jint, jint)> = compute(geometries, traffic_lights);
    stop_watch.elapsed_store("Computation Total Time");
    let nanos = start_time_map.elapsed().as_nanos() as f64;
    let nano_seconds_per_op = nanos / ((geometries.len() * traffic_lights.len()) as f64);
    let per_core_average = nano_seconds_per_op * 24f64;
    stop_watch.print_prefixed("Rust Binding");
    println!("Rust Binding - Check Average: {nano_seconds_per_op} ns/op");
    println!("Rust Binding - Per Core Average: {per_core_average} ns/op");
    add_solver(Solver::new(get_nodes().get_slice(), 373729, 37887, 100_000, SearchMethod::SHORTEST));
    build_node_tree();
    associate_traffic_lights_to_nodes();
}

fn main() {
    computation();
    start_search();
    start_window();
}