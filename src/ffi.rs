use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Once;

use tracing::info;

use crate::error::ParseError;
use crate::{parse, Options};

static mut VERSION: *const c_char = std::ptr::null();
static INIT: Once = Once::new();

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
pub extern "C" fn ffi_parse(options: *const c_char) -> *mut c_char {
    let options = match (|| {
        let opts = (unsafe { CStr::from_ptr(options) })
            .to_str()
            .map_err(|e| ParseError::new(&format!("{}", e)))?;
        let opts = serde_json::from_str::<Options>(opts)
            .map_err(|e| ParseError::new(&format!("{}", e)))?;
        Ok::<_, ParseError>(opts)
    })() {
        Ok(opts) => opts,
        Err(e) => {
            info!("Failed to parse options: {:?}", e);
            return std::ptr::null_mut();
        }
    };

    let entity = parse(&options);

    if let Err(e) = entity {
        info!("Failed to parse file: {:?}", e);
        return std::ptr::null_mut();
    }

    let serialized = serde_json::to_string(&entity.unwrap()).unwrap();
    CString::new(serialized).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn ffi_free(result: *mut c_char) {
    if !result.is_null() {
        unsafe {
            drop(CString::from_raw(result));
        }
    }
}
