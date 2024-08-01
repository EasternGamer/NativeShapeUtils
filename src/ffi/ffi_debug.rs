use jni::JNIEnv;
use jni::objects::JClass;

use crate::debug_window::start_window;

#[no_mangle]
pub extern "system" fn Java_io_github_easterngamer_ffi_FFIDebug_launchWindow<'l>(_env: JNIEnv<'l>, _class: JClass<'l>) { 
    start_window();
}
