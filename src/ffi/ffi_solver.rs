use std::simd::Simd;
use std::thread::spawn;

use jni::JNIEnv;
use jni::objects::{AsJArrayRaw, JByteArray, JClass, JIntArray};
use jni::sys::{jdouble, jint, jintArray, jsize};

use crate::{add_nodes, add_solver, associate_traffic_lights_to_nodes, build_node_tree, get_closest_node, get_nodes, get_solver, get_traffic_lights, new_slice};
use crate::loader::load_from_bytes;
use crate::objects::pathing::solver::Solver;
use crate::types::{Flag, Pos};

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFISolver_sendNodes<'l> (env: JNIEnv<'l>, _class: JClass<'l>, data : JByteArray<'l>) {
    let bytes = env.convert_byte_array(&data).expect("Failed to load byte array for traffic lights");
    add_nodes(load_from_bytes(bytes.as_slice()))
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFISolver_updateTrafficLightFlags<'l> (env: JNIEnv<'l>, _class: JClass<'l>, data : JIntArray<'l>) {
    let mut flags = new_slice(0i32, env.get_array_length(&data).expect("") as usize);
    env.get_int_array_region(&data, 0, &mut flags).expect("Failed to load byte array for traffic lights");
    let traffic_lights = get_traffic_lights().get_slice_mut();
    for (index, flag) in flags.iter().enumerate() {
        traffic_lights[index].get_mut().flag = *flag as Flag;
    }
    associate_traffic_lights_to_nodes();
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFISolver_buildSolver<'l>(_env: JNIEnv<'l>, _class: JClass<'l>) {
    add_solver(Solver::new(get_nodes().get_slice(), 0, 0, 100_000_000));
    build_node_tree();
    associate_traffic_lights_to_nodes();
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFISolver_findPath<'l>(env: JNIEnv<'l>, _class: JClass<'l>,
                                                                               source_x : jdouble, source_y : jdouble,
                                                                               destination_x : jdouble, destination_y : jdouble) -> jintArray {
    println!("Starting path...");
    let start_pos = Simd::from_array([source_x as Pos, source_y as Pos]);
    let end_pos = Simd::from_array([destination_x as Pos, destination_y as Pos]);
    let closest_to_start = spawn(move || {get_closest_node(&start_pos)});
    let closest_to_end = spawn(move || {get_closest_node(&end_pos)});
    println!("Finding path...");
    let solver = get_solver();
    println!("Getting solver...");
    solver.update_search(closest_to_start.join().expect("Unable to find closest start"), closest_to_end.join().expect("Unable to find closest end"));
    println!("Updating search...");
    solver.update_search_speed(100_000_000);
    println!("Prefind...");
    solver.compute_pre_find();
    println!("Path Found...");
    let indices : Vec<jint> = solver.get_path_as_indices()
        .as_ref()
        .map(|t| {&t.0})
        .unwrap_or(&Vec::new().into_boxed_slice())
        .iter()
        .map(|x| *x as jint)
        .collect();
    let indexes = &env.new_int_array(indices.len() as jsize).unwrap();
    println!("Path Indices...");
    env.set_int_array_region(indexes, 0, indices.as_slice()).expect("TODO: panic message");
    indexes.as_jarray_raw()
}