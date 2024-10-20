use std::ffi::CStr;
use std::os::raw::{c_char, c_float, c_ulonglong};
use std::panic;
use std::path::Path;

use tracing::info;

use crate::parse;

#[repr(C)]
pub struct ParsedModelData {
    vertices_ptr: *const c_float,
    vertices_len: usize,
    uvs_ptr: *const c_float,
    uvs_len: usize,
    faces_ptr: *const c_ulonglong,
    faces_len: usize,
    vertices: Vec<f32>,
    uvs: Vec<f32>,
    faces: Vec<u64>,
}

#[no_mangle]
pub extern "C" fn ffi_parse(xml_file_path: *const c_char) -> *mut ParsedModelData {
    let result = panic::catch_unwind(|| {
        let c_str = unsafe { CStr::from_ptr(xml_file_path) };
        let xml_file = Path::new(c_str.to_str().unwrap());

        let (vertices, uvs, faces) = parse(xml_file);

        let vertices_flat: Vec<f32> = vertices.into_iter().flatten().collect();
        let uvs_flat: Vec<f32> = uvs.into_iter().flatten().collect();
        let faces_flat: Vec<u64> = faces.into_iter().flatten().collect();

        info!(
            "Packing {} vertices, {} uvs, and {} faces",
            vertices_flat.len(),
            uvs_flat.len(),
            faces_flat.len()
        );

        let result = Box::new(ParsedModelData {
            vertices_ptr: vertices_flat.as_ptr() as *const c_float,
            vertices_len: vertices_flat.len(),
            uvs_ptr: uvs_flat.as_ptr() as *const c_float,
            uvs_len: uvs_flat.len(),
            faces_ptr: faces_flat.as_ptr() as *const c_ulonglong,
            faces_len: faces_flat.len(),
            vertices: vertices_flat,
            uvs: uvs_flat,
            faces: faces_flat,
        });

        Box::into_raw(result)
    });

    match result {
        Ok(ptr) => ptr,
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn ffi_free(result: *mut ParsedModelData) {
    if !result.is_null() {
        unsafe {
            drop(Box::from_raw(result));
        }
    }
}
