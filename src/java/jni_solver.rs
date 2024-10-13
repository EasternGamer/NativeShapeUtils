use std::simd::Simd;
use std::thread::spawn;

use jni::objects::{JByteArray, JClass, JIntArray, JValue};
use jni::sys::{jdouble, jint, jobject, jsize};
use jni::JNIEnv;

use crate::loader::load_from_bytes;
use crate::objects::pathing::node_type::SearchMethod;
use crate::objects::pathing::solver::Solver;
use crate::types::{Flag, Pos};
use crate::{add_nodes, add_solver, associate_traffic_lights_to_nodes, build_node_tree, get_closest_node, get_nodes, get_solver, get_traffic_lights, new_slice, remove_solver};

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_jni_JNISolver_sendNodes<'l> (env: JNIEnv<'l>, _class: JClass<'l>, data : JByteArray<'l>) {
    let bytes = env.convert_byte_array(&data).expect("Failed to load byte array for traffic lights");
    add_nodes(load_from_bytes(bytes.as_slice()));
    build_node_tree();
}
#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_jni_JNISolver_associateTrafficLightsToNodes<'l> (_env: JNIEnv<'l>, _class: JClass<'l>) {
    associate_traffic_lights_to_nodes();
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_jni_JNISolver_destroySolver<'l>(_env: JNIEnv<'l>, _class: JClass<'l>) {
    remove_solver()
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_jni_JNISolver_updateTrafficLightFlags<'l> (env: JNIEnv<'l>, _class: JClass<'l>, data : JIntArray<'l>) {
    let mut flags = new_slice(0i32, env.get_array_length(&data).expect("") as usize);
    env.get_int_array_region(&data, 0, &mut flags).expect("Failed to load byte array for traffic lights");
    let traffic_lights = get_traffic_lights().get_slice_mut();
    for (index, flag) in flags.iter().enumerate() {
        traffic_lights[index].get_mut().flag = *flag as Flag;
    }
    associate_traffic_lights_to_nodes();
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_jni_JNISolver_buildSolver<'l>(_env: JNIEnv<'l>, _class: JClass<'l>) -> jint {
    add_solver(Solver::new(get_nodes().get_slice(), 0, 0, 100_000_000, SearchMethod::FASTEST)) as jint
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_jni_JNISolver_setSearchMethod<'l>(_env: JNIEnv<'l>, _class: JClass<'l>, index : jint, search_method: jint) {
    get_solver(index as usize).search_method = match search_method {
        0 => SearchMethod::FASTEST,
        1 => SearchMethod::SHORTEST,
        _ => SearchMethod::AVOID
    }
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_jni_JNISolver_findPath<'l>(mut env: JNIEnv<'l>, _class: JClass<'l>,
                                                                              index: jint,
                                                                              source_x : jdouble, source_y : jdouble,
                                                                              destination_x : jdouble, destination_y : jdouble) -> jobject {
    let start_pos = Simd::from_array([source_x as Pos, source_y as Pos]);
    let end_pos = Simd::from_array([destination_x as Pos, destination_y as Pos]);
    let closest_to_start = spawn(move || {get_closest_node(&start_pos)});
    let closest_to_end = spawn(move || {get_closest_node(&end_pos)});
    let path_class = env.find_class("io/github/easterngamer/jni/JNISolver$Path").unwrap();
    let init_method = env.get_method_id(&path_class, "<init>", "([IDD)V").unwrap();
    let solver = get_solver(index as usize);
    
    match (closest_to_start.join().expect("Unable to find closest start"), closest_to_end.join().expect("Unable to find closest end")) {
        (Some(start), Some(end)) => {
            solver.update_search(start, end);
            solver.update_search_speed(100_000_000);
            while !solver.fully_searched() {
                solver.compute();
            }
            if let Some((path_data, cost, distance)) = solver.get_path_as_indices().as_ref() {
                let indices: Vec<jint> = path_data
                    .iter()
                    .map(|x| *x as jint)
                    .collect();
                let indexes = &env.new_int_array(indices.len() as jsize).unwrap();
                env.set_int_array_region(indexes, 0, indices.as_slice()).expect("TODO: panic message");
                let array = JValue::from(indexes).as_jni();
                let cost = JValue::from(*cost as f64).as_jni();
                let distance = JValue::from(*distance as f64).as_jni();

                unsafe { env.new_object_unchecked(path_class, init_method, &[array, cost, distance]).expect("Unable to create object").as_raw() }
            } else {
                let array = JValue::from(&env.new_int_array(0 as jsize).unwrap()).as_jni();
                let zero = JValue::from(0.0f64).as_jni();
                unsafe { env.new_object_unchecked(path_class, init_method, &[array, zero, zero]).expect("Unable to create object").as_raw() }
            }
        }
        _ => {
            let array = JValue::from(&env.new_int_array(0 as jsize).unwrap()).as_jni();
            let zero = JValue::from(0.0f64).as_jni();
            unsafe { env.new_object_unchecked(path_class, init_method, &[array, zero, zero]).expect("Unable to create object").as_raw() }
        }
    }
    
    
}