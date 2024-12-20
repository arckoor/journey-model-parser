use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_float, c_uint};
use std::path::Path;
use std::sync::Once;

use tracing::info;

use crate::entity::Entity;
use crate::parse;

static mut VERSION: *const c_char = std::ptr::null();
static INIT: Once = Once::new();

#[repr(C)]
pub struct ParsedModelData {
    object_count: usize,
    vertices_ptr: *const c_float,
    uvs_ptr: *const c_float,
    faces_ptr: *const c_uint,
    vertices_len_ptr: *const usize,
    uvs_len_ptr: *const usize,
    faces_len_ptr: *const usize,
    translation_ptr: *const c_float,
    vertices: Vec<f32>,
    uvs: Vec<f32>,
    faces: Vec<u32>,
    vertices_len: Vec<usize>,
    uvs_len: Vec<usize>,
    faces_len: Vec<usize>,
    translation: Vec<f32>,
}

impl Entity {
    pub fn to_ffi(&self) -> Box<ParsedModelData> {
        let mut vertices_flat = Vec::new();
        let mut uvs_flat = Vec::new();
        let mut faces_flat = Vec::new();

        let mut vertices_len = Vec::new();
        let mut uvs_len = Vec::new();
        let mut faces_len = Vec::new();

        let translation = self.translation.to_vec();

        for object in &self.objects {
            let object_vertices: Vec<f32> = object.vertices.iter().copied().flatten().collect();
            let object_uvs: Vec<f32> = object.uvs.iter().copied().flatten().collect();
            let object_faces: Vec<u32> = object.faces.iter().copied().flatten().collect();

            vertices_len.push(object_vertices.len());
            uvs_len.push(object_uvs.len());
            faces_len.push(object_faces.len());

            vertices_flat.extend(object_vertices);
            uvs_flat.extend(object_uvs);
            faces_flat.extend(object_faces);
        }

        let object_count = self.objects.len();

        info!(
            "Packing {} vertices, {} uvs, {} faces from {} object(s)",
            vertices_flat.len(),
            uvs_flat.len(),
            faces_flat.len(),
            object_count
        );

        Box::new(ParsedModelData {
            vertices_ptr: vertices_flat.as_ptr(),
            uvs_ptr: uvs_flat.as_ptr(),
            faces_ptr: faces_flat.as_ptr(),
            object_count,
            vertices_len_ptr: vertices_len.as_ptr(),
            uvs_len_ptr: uvs_len.as_ptr(),
            faces_len_ptr: faces_len.as_ptr(),
            translation_ptr: translation.as_ptr(),
            vertices: vertices_flat,
            uvs: uvs_flat,
            faces: faces_flat,
            vertices_len,
            uvs_len,
            faces_len,
            translation,
        })
    }
}

#[no_mangle]
pub extern "C" fn ffi_version() -> *const c_char {
    unsafe {
        INIT.call_once(|| {
            let version = env!("CARGO_PKG_VERSION");
            let version = CString::new(version).unwrap();
            VERSION = version.into_raw();
        });
        VERSION
    }
}

#[no_mangle]
pub extern "C" fn ffi_parse(xml_file_path: *const c_char) -> *mut ParsedModelData {
    let c_str = unsafe { CStr::from_ptr(xml_file_path) };
    let xml_file = Path::new(c_str.to_str().unwrap());
    let entity = parse(xml_file);

    if let Err(e) = entity {
        info!("Failed to parse file: {:?}", e);
        return std::ptr::null_mut();
    }

    let result = entity.unwrap().to_ffi();

    Box::into_raw(result)
}

#[no_mangle]
pub extern "C" fn ffi_free(result: *mut ParsedModelData) {
    if !result.is_null() {
        unsafe {
            drop(Box::from_raw(result));
        }
    }
}
