use jni::JNIEnv;
use jni::objects::{JClass, JIntArray};
use jni::sys::{jdouble, jint};
use crate::{add_node, get_nodes, new_slice, SOLVER};
use crate::objects::solver::connection::Connection;
use crate::objects::solver::solver::Solver;
use crate::types::Cost;

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