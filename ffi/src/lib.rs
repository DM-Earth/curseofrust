#![allow(non_snake_case)]

use std::{
    ffi::{c_char, c_void, CStr, CString},
    ptr::null,
};

use curseofrust::{
    state::{BasicOpts, State, UI},
    Pos,
};

/// Helper struct for functions that:
/// - has two return values.
/// - involves error handling.
#[repr(C)]
pub struct CORFunctionReturn {
    first: *const c_void,
    second: *const c_void,
}

/// Parse Curse of Rust options from C string.
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
pub extern "C" fn CORParseOptions(optStringPtr: *const c_char) -> CORFunctionReturn {
    let opt_string =
        String::from_utf8_lossy(unsafe { CStr::from_ptr(optStringPtr) }.to_bytes()).to_string();
    cli_parser::parse(opt_string.split_whitespace()).map_or_else(
        |e| CORFunctionReturn {
            first: null(),
            second: CString::new(e.to_string())
                .expect("CString::new failure in CORParseOptions()")
                .into_raw()
                .cast(),
        },
        |(basic_opts, multiplayer_opts)| CORFunctionReturn {
            first: Box::into_raw(Box::new(basic_opts)).cast(),
            second: Box::into_raw(Box::new(multiplayer_opts)).cast(),
        },
    )
}

/// Release the error string returned by [`CORParseOptions`], [`CORMakeState`].
#[no_mangle]
pub extern "C" fn CORReleaseErrorString(errorStringPtr: *const c_char) {
    unsafe { drop(CString::from_raw(errorStringPtr as *mut _)) }
}

/// Generate initial [`State`] from [`BasicOpts`].
/// # Returns
/// ## On success
/// first: A `CORStateRef`.
/// second: A `nil` pointer.
/// ## On error
/// first: A `nil` pointer.
/// second: Error msg in `char*` form.
///
/// # Extra info
/// The param `basicOptsPtr` will be consumed.
#[no_mangle]
pub extern "C" fn CORMakeState(basicOptsPtr: *const c_void) -> CORFunctionReturn {
    let basic_opts = unsafe { Box::from_raw(basicOptsPtr.cast::<BasicOpts>() as *mut _) };
    State::new(*basic_opts).map_or_else(
        |e| CORFunctionReturn {
            first: CString::new(e.to_string())
                .expect("CString::new failure in CORMakeState()")
                .into_raw()
                .cast(),
            second: null(),
        },
        |v| CORFunctionReturn {
            first: null(),
            second: Box::into_raw(Box::new(v)).cast(),
        },
    )
}

/// C struct of [`Pos`].
#[repr(C)]
pub struct CORPosition {
    /// Horizontal axis.
    pub x: i32,
    /// Vertical axis.
    pub y: i32,
}

impl From<Pos> for CORPosition {
    fn from(value: Pos) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}

impl Into<Pos> for CORPosition {
    fn into(self) -> Pos {
        Pos(self.x, self.y)
    }
}

/// C struct of [`UI`].
#[repr(C)]
pub struct CORInterface {
    pub cursor: CORPosition,
    /// Number of tiles to skip in the beginning of
    /// every line.
    pub xskip: u16,
    /// Total max number of tiles in horizontal direction.
    pub xlen: u16,
}

impl From<UI> for CORInterface {
    fn from(value: UI) -> Self {
        Self {
            cursor: value.cursor.into(),
            xskip: value.xskip,
            xlen: value.xlen,
        }
    }
}

/// Generate [`UI`] from [`State`].
#[no_mangle]
pub extern "C" fn CORMakeInterface(statePtr: *const c_void) -> CORInterface {
    let state = unsafe { Box::from_raw(statePtr.cast::<State>() as *mut _) };
    let ui = UI::new(&state).into();
    Box::leak(state);
    ui
}

/// Extract game seed from [`State`].
#[no_mangle]
pub extern "C" fn CORGetSeed(statePtr: *const c_void) -> u64 {
    let state: Box<State> = unsafe { Box::from_raw(statePtr.cast::<State>() as *mut _) };
    let seed = state.seed;
    Box::leak(state);
    seed
}

/// Extract grid height form [`State`].
#[no_mangle]
pub extern "C" fn CORGetGridHeight(statePtr: *const c_void) -> u32 {
    let state: Box<State> = unsafe { Box::from_raw(statePtr.cast::<State>() as *mut _) };
    let height = state.grid.height();
    Box::leak(state);
    height
}

/// Extract grid width form [`State`].
#[no_mangle]
pub extern "C" fn CORGetGridWidth(statePtr: *const c_void) -> u32 {
    let state: Box<State> = unsafe { Box::from_raw(statePtr.cast::<State>() as *mut _) };
    let width = state.grid.width();
    Box::leak(state);
    width
}

#[no_mangle]
pub extern "C" fn CORKingsMove(statePtr: *const c_void) {
    let mut state: Box<State> = unsafe { Box::from_raw(statePtr.cast::<State>() as *mut _) };
    state.kings_move();
    Box::leak(state);
}

#[no_mangle]
pub extern "C" fn CORSimulate(statePtr: *const c_void) {
    let mut state: Box<State> = unsafe { Box::from_raw(statePtr.cast::<State>() as *mut _) };
    state.simulate();
    Box::leak(state);
}

#[no_mangle]
pub extern "C" fn CORGetTile(statePtr: *const c_void, pos: CORPosition) {
    let state: Box<State> = unsafe { Box::from_raw(statePtr.cast::<State>() as *mut _) };
    let tile = state.grid.tile(pos.into());
    todo!()
    // What should it return?
    // C191239 2024-06-25 11:44:32 +0800
}
