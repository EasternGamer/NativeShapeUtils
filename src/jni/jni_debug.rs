use jni::JNIEnv;
use jni::objects::JClass;
use jni::sys::jint;

use crate::debug_window::{start_search, start_window};
use crate::get_solver;
use crate::types::Index;

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_jni_JNIDebug_launchWindowInternal<'l>(_env: JNIEnv<'l>, _class: JClass<'l>) {
    get_solver(0).update_search(373729, 37887);
    start_window();
}

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_jni_JNIDebug_updateSearch<'l>(_env: JNIEnv<'l>, _class: JClass<'l>, start_node_index : jint, end_node_index : jint) {
    get_solver(0).update_search(start_node_index as Index, end_node_index as Index);
    start_search();
}