//
// Created by alik on 21.11.18.
//

#ifndef RESTRY_FFI_H
#define RESTRY_FFI_H

#include "amxxmodule.h"

extern "C" {
//    typedef (*void) RestryRequest;
//    RestryRequest restry_create_post_request();
//    RestryRequest destroy_request();

    typedef void (*GripErrorLogger)(const void* amx, const char* str);

    void grip_init(GripErrorLogger logger);
    void grip_deinit();
    void grip_process_request();

    // void handler(cell forward_handle, cell response_handle, const cell *user_data, cell user_data_size);
    typedef void (*GripHandler)(cell, cell, const cell*, cell);


    cell grip_request(cell forward_id, const char *url, cell request_type, GripHandler handler,  const cell *user_data, cell user_data_size);

//    pub unsafe extern "C" fn grip_request(
//            forward_id: Cell,
//    uri: Option<*const c_char>,
//    request_type: Cell,
//    handler: Option<
//    extern "C" fn(
//            forward_handle: Cell,
        //    response_handle: Cell,
        //    user_data: *const Cell,
        //    user_data_size: Cell,
//    ) -> c_void,
//    >,
//    user_data: Option<*const Cell>,
//    user_data_size: Cell,
//    ) -> Cell {
}

#endif //RESTRY_FFI_H
