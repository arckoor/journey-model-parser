use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Once;

use crate::error::ParseError;
use crate::{initialize_tracing, parse, Options};

static mut VERSION: *const c_char = std::ptr::null();
static INIT: Once = Once::new();

macro_rules! FFI_VISIT {
    ($ok:ty, $err:expr, $body:block) => {{
        let f = || -> Result<$ok, ParseError> { $body };
        match f() {
            Ok(val) => val,
            Err(e) => {
                tracing::error!("Error: {}", e);
                $err
            }
        }
    }};
}

#[no_mangle]
pub extern "C" fn ffi_init() {
    initialize_tracing();
}

#[no_mangle]
pub extern "C" fn ffi_version() -> *const c_char {
    FFI_VISIT!(*const c_char, std::ptr::null(), {
        unsafe {
            INIT.call_once(|| {
                let version = env!("CARGO_PKG_VERSION");
                let version = CString::new(version).unwrap();
                VERSION = version.into_raw();
            });
            Ok(VERSION)
        }
    })
}

#[no_mangle]
pub extern "C" fn ffi_parse(options: *const c_char) -> *mut c_char {
    FFI_VISIT!(*mut c_char, std::ptr::null_mut(), {
        let options = (|| {
            let opts = (unsafe { CStr::from_ptr(options) })
                .to_str()
                .map_err(|e| ParseError::new(&format!("{e}")))?;
            let opts = serde_json::from_str::<Options>(opts)
                .map_err(|e| ParseError::new(&format!("{e}")))?;
            Ok::<_, ParseError>(opts)
        })()?;

        let entity = parse(&options)?;

        Ok(CString::new(entity.to_json(false)).unwrap().into_raw())
    })
}

#[no_mangle]
pub extern "C" fn ffi_free(result: *mut c_char) {
    FFI_VISIT!((), (), {
        if !result.is_null() {
            unsafe {
                drop(CString::from_raw(result));
            }
        }
        Ok(())
    })
}
