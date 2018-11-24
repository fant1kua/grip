// Cell map

use networking_queue::{Queue, RequestType};


struct ModuleStorage {
    pub global_queue: Queue,
}

static mut MODULE: Option<ModuleStorage> = None;

#[no_mangle]
pub extern "C" fn grip_deinit() {
    println!("Resty deinit");
}

#[no_mangle]
pub extern "C" fn grip_init() {
    static LOGGER_INIT: std::sync::Once = std::sync::ONCE_INIT;
    LOGGER_INIT.call_once(|| {
        pretty_env_logger::init();
    });

    unsafe {
        MODULE = Some(ModuleStorage {
            global_queue: Queue::new(4)  // TODO: Read from config?
        });
    }
}

