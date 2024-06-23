#![allow(non_snake_case)]

use std::ffi::{c_char, c_void, CStr, CString};

use curseofrust::state::{BasicOpts, State, UI};

#[repr(C)]
pub struct CORParseOptionsReturn {
    first: *const c_void,
    second: *const c_void,
}
/// # Params
/// - optStringPtr: A pointer to options string.
///
/// # Returns
/// Always return a `CORParseOptionsReturnRef`.
/// ## On success
/// - first: A `CORBasicOptsRef`.
/// - second: A `CORMultiplayerOptsRef`.
/// ## On error
/// - first: A `nil` pointer.
/// - second: Error msg in `char*` form.
#[no_mangle]
pub extern "C" fn CORParseOptions(optStringPtr: *const c_char) -> CORParseOptionsReturn {
    let opt_string =
        String::from_utf8_lossy(unsafe { CStr::from_ptr(optStringPtr) }.to_bytes()).to_string();
    cli_parser::parse(opt_string.split_whitespace()).map_or_else(
        |e| CORParseOptionsReturn {
            first: 0 as _,
            second: CString::new(e.to_string())
                .expect("CString::new failure in CORParseOptions()")
                .into_raw()
                .cast(),
        },
        |(basic_opts, multiplayer_opts)| CORParseOptionsReturn {
            first: Box::into_raw(Box::new(basic_opts)).cast(),
            second: Box::into_raw(Box::new(multiplayer_opts)).cast(),
        },
    )
}

/// # Usage
/// Release the error string returned by [`CORParseOptions`], [`CORMakeState`].
#[no_mangle]
pub extern "C" fn CORReleaseErrorString(errorStringPtr: *const c_char) {
    unsafe { drop(CString::from_raw(errorStringPtr as *mut _)) }
}

/// # Returns
/// ## On success
/// A `CORStateRef`.
/// ## On error
/// Error msg in `char*` form.
#[no_mangle]
pub extern "C" fn CORMakeState(basicOptsPtr: *const c_void) -> *const c_void {
    let basic_opts = unsafe { Box::from_raw(basicOptsPtr.cast::<BasicOpts>() as *mut _) };
    State::new(*basic_opts).map_or_else(
        |e| {
            CString::new(e.to_string())
                .expect("CString::new failure in CORMakeState()")
                .into_raw()
                .cast()
        },
        |v| Box::into_raw(Box::new(v)).cast(),
    )
}

#[no_mangle]
pub extern "C" fn CORMakeUI(statePtr: *const c_void) -> *const c_void {
    let state = unsafe { Box::from_raw(statePtr.cast::<State>() as *mut _) };
    Box::into_raw(Box::new(UI::new(&state))).cast()
}
