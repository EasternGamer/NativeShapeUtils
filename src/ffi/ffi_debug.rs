use jni::JNIEnv;
use jni::objects::JClass;
use jni::sys::jint;

use crate::debug_window::{start_search, start_window};
use crate::get_solver;

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFIDebug_launchWindowInternal<'l>(_env: JNIEnv<'l>, _class: JClass<'l>) { 
    get_solver().update_search(373729, 37887);
    start_window();
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFIDebug_updateSearch<'l>(_env: JNIEnv<'l>, _class: JClass<'l>, start_node_index : jint, end_node_index : jint) {
    get_solver().update_search(start_node_index as usize, end_node_index as usize);
    start_search();
}