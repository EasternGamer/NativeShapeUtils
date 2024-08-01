use std::simd::Simd;
use std::time::Instant;
use jni::JNIEnv;
use jni::objects::{AsJArrayRaw, JByteArray, JClass};
use jni::signature::Primitive::Void;
use jni::signature::ReturnType;
use jni::sys::{jboolean, jdouble, jint, jintArray, jsize, jvalue};
use rayon::prelude::ParallelSlice;
use crate::{add_geometry, add_traffic_lights, compute, get_geometry, get_traffic_lights, new_double_slice, new_pos_slice};
use crate::loader::load_from_bytes;
use crate::objects::boundary::Boundary;
use crate::types::Pos;

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFITraffic_sendTrafficLights<'l>(env: JNIEnv<'l>, _class: JClass<'l>, data : JByteArray<'l>) {
    let bytes = env.convert_byte_array(&data).expect("Failed to load byte array for traffic lights");
    add_traffic_lights(load_from_bytes(bytes.as_slice()))
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFITraffic_sendSuburbs<'l> (env: JNIEnv<'l>, _class: JClass<'l>, data : JByteArray<'l>) {
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
pub extern "system" fn Java_io_github_easterngamer_ffi_FFITraffic_getSuburbsInBounds<'l>(env: JNIEnv<'l>, _class: JClass<'l>,
                                                                                                 max_x : jdouble, min_x : jdouble,
                                                                                                 max_y : jdouble, min_y : jdouble,
                                                                                                 limit : jint, debug : jboolean) -> jintArray {
    let start_time = Instant::now();
    let result = get_geometry();
    let geometries = result.as_slice();
    let boundary = Boundary {
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
    let result = get_traffic_lights();
    let traffic_lights = result.as_slice();
    let boundary = Boundary {
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
pub extern "system" fn Java_io_github_easterngamer_ffi_FFITraffic_compute<'l>(mut env: JNIEnv<'l>, class: JClass<'l>, debug: jboolean) {
    println!("Computing...");
    let start_time_pre = Instant::now();

    let method_id = env.get_static_method_id(&class, "receiveTrafficLight", "(II)V").expect("Something went wrong getting static method");
    let temp_geo = get_geometry();
    let temp_traffic = get_traffic_lights();
    let geometries = temp_geo.as_slice().as_parallel_slice();
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