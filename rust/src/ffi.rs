extern crate libc;

use self::libc::{c_char, c_int, c_void, size_t};

use std::ffi::CStr;

use crate::errors::*;

type Cell = isize;

static INVALID_CELL: Cell = 0;
use networking_queue::{Queue, RequestBuilder, RequestType, Response};
use std::prelude::v1::Vec;

use cell_map::CellMap;

struct ModuleStorage {
    pub global_queue: Queue,
    pub responses_handles: CellMap<Response>,
    pub error_logger: extern "C" fn(*const c_void, *const c_char),
}

trait ResultFFIExt<T> {
    unsafe fn handle_ffi_error(self, amx: *const c_void) -> std::result::Result<T, Cell>;
}

impl<T> ResultFFIExt<T> for Result<T> {
    unsafe fn handle_ffi_error(self, amx: *const c_void) -> std::result::Result<T, Cell> {
        self.map_err(|err| {
            (get_module().error_logger)(amx, format!("{}\0", err).as_ptr() as *const c_char);
            INVALID_CELL
        })
    }
}

impl<T> ResultFFIExt<T> for Option<T> {
    unsafe fn handle_ffi_error(self, amx: *const c_void) -> std::result::Result<T, Cell> {
        self.ok_or(INVALID_CELL).map_err(|_| {
            (get_module().error_logger)(amx, "Got null pointer\0".as_ptr() as *const c_char);
            INVALID_CELL
        })
    }
}

macro_rules! try_ffi {
    ($amx:expr, $expr:expr) => {
        match $expr.handle_ffi_error($amx) {
            $crate::std::result::Result::Ok(val) => val,
            $crate::std::result::Result::Err(err) => return err,
        }
    };
    ($expr:expr,) => {
        try_ffi!($expr)
    };
}

static mut MODULE: Option<ModuleStorage> = None;

#[no_mangle]
pub unsafe extern "C" fn grip_init(error_logger: extern "C" fn(*const c_void, *const c_char)) {
    MODULE = Some(ModuleStorage {
        global_queue: Queue::new(4), // TODO: Read from config?
        responses_handles: CellMap::new(),
        error_logger,
    });
}

fn handle_null_ptr<T>(ptr: *const T) -> Option<*const T> {
    if ptr.is_null() {
        None
    } else {
        Some(ptr)
    }
}

unsafe fn get_module() -> &'static mut ModuleStorage {
    MODULE.as_mut().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn grip_deinit() {
    MODULE = None;
}

#[no_mangle]
pub unsafe extern "C" fn grip_request(
    amx: *const c_void,
    forward_id: Cell,
    uri: *const c_char,
    request_type: Cell,
    handler: Option<
        extern "C" fn(
            forward_handle: Cell,
            response_handle: Cell,
            user_data: *const Cell,
            user_data_size: Cell,
        ) -> c_void,
    >,
    user_data: *const Cell,
    user_data_size: Cell,
) -> Cell {
    let request_type = try_ffi!(
        amx,
        match request_type {
            0 => Ok(RequestType::Get),
            _ => Err(ffi_error(format!("Invalid request type {}", request_type))),
        }
    ); // TODO: Request type

    let uri = try_ffi!(
        amx,
        CStr::from_ptr(try_ffi!(
            amx,
            handle_null_ptr(uri).ok_or_else(|| ffi_error("Invalid URI."))
        )).to_str()
        .map_err(|_| ffi_error("URI is not UTF-8"))
    );

    let user_data: Vec<Cell> = std::slice::from_raw_parts(
        try_ffi!(
            amx,
            handle_null_ptr(user_data).ok_or_else(|| ffi_error("Invalid user data"))
        ),
        user_data_size as usize,
    ).to_vec();

    get_module().global_queue.send_request(
        RequestBuilder::default()
            .id(forward_id)
            .http_type(RequestType::Get)
            .uri(try_ffi!(
                amx,
                uri.parse()
                    .map_err(|_| ffi_error(format!("URI parsing error: {}", uri)))
            )).build()
            .unwrap(),
        move |response| {
            let response_id = get_module()
                .responses_handles
                .insert_with_unique_id(response);

            handler.unwrap()(
                forward_id,
                response_id,
                user_data.as_ptr(),
                user_data_size as isize,
            );

            get_module().responses_handles.remove_with_id(response_id);
        },
    );

    // TODO: Request handle
    1
}

#[no_mangle]
pub unsafe extern "C" fn grip_process_request() {
    get_module()
        .global_queue
        .execute_queue_with_limit(5, std::time::Duration::from_nanos(1000000));
}
