use ffi::*;

pub trait ResultFFIExt<T> {
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

pub fn handle_null_ptr<T>(ptr: *const T) -> Option<*const T> {
    if ptr.is_null() {
        None
    } else {
        Some(ptr)
    }
}
