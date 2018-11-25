extern crate libc;

use self::libc::{c_char, c_int, c_void, size_t};
use std::borrow::Borrow;
use std::error::Error;
use std::ffi::CStr;

use crate::errors::*;

type Cell = size_t;

static INVALID_CELL: Cell = 0;
use networking_queue::{Queue, RequestBuilder, RequestType};
use std::prelude::v1::Vec;

struct ModuleStorage {
    pub global_queue: Queue,
}

trait ResultFFIExt<T> {
    fn handle_ffi_error(self) -> std::result::Result<T, Cell>;
}

impl<T> ResultFFIExt<T> for Result<T> {
    fn handle_ffi_error(self) -> std::result::Result<T, Cell> {
        self.map_err(|err| {
            error!("{}", err);
            0
        })
    }
}

impl<T> ResultFFIExt<T> for Option<T> {
    fn handle_ffi_error(self) -> std::result::Result<T, Cell> {
        self.ok_or(0).map_err(|e| {
            error!("Got null pointer");
            e
        })
    }
}

macro_rules! try_ffi {
    ($expr:expr) => {
        match $expr.handle_ffi_error() {
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
pub unsafe extern "C" fn grip_init() {
    static LOGGER_INIT: std::sync::Once = std::sync::ONCE_INIT;
    LOGGER_INIT.call_once(|| {
        pretty_env_logger::init_custom_env("GRIP_GLOBAL_LOGGER");
    });

    MODULE = Some(ModuleStorage {
        global_queue: Queue::new(4), // TODO: Read from config?
    });
}

unsafe fn get_module() -> &'static mut ModuleStorage {
    MODULE.as_mut().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn grip_deinit() {
    MODULE = None;
}

pub unsafe extern "C" fn grip_request(
    forward_id: size_t,
    uri: Option<*const c_char>,
    request_type: Cell,
    handler: Option<
        extern "C" fn(response_handle: Cell, user_data: *const Cell, user_data_size: Cell)
            -> c_void,
    >,
    user_data: Option<*const Cell>,
    user_data_size: Cell,
) -> Cell {
    let request_type = try_ffi!(match request_type {
        0 => Ok(RequestType::Get),
        _ => Err(ffi_error(format!("Invalid request type {}", request_type))),
    });

    let uri = CStr::from_ptr(try_ffi!(uri.ok_or(ffi_error("Invalid URI."))));

    let user_data: Vec<Cell> = std::slice::from_raw_parts(
        try_ffi!(user_data.ok_or(ffi_error("Invalid user data"))),
        user_data_size,
    ).to_vec();

    get_module().global_queue.send_request(
        RequestBuilder::default()
            .id(forward_id)
            .http_type(RequestType::Get)
            .uri("https://docs.rs/".parse().unwrap())
            .build()
            .unwrap(),
        move |response| {
            handler.unwrap()(forward_id, user_data.as_ptr(), user_data_size);
        },
    );

    // TODO: Request handle
    1
}
pub unsafe extern "C" fn grip_process_request() {
    use crate::std::time::Duration;
    get_module().global_queue.execute_query_with_timeout(Duration::from_millis(100000), Duration::from_nanos(0));
}