//
// Created by alik on 21.11.18.
//

#ifndef RESTRY_FFI_H
#define RESTRY_FFI_H


extern "C" {
//    typedef (*void) RestryRequest;
//    RestryRequest restry_create_post_request();
//    RestryRequest destroy_request();

    void grip_init();
    void grip_deinit();
    void grip_process_request();
    //pub unsafe extern "C" fn grip_process_request() {

    //pub unsafe extern "C" fn grip_request(
    //        forward_id: size_t,
    //uri: Option<*const c_char>,
    //request_type: Cell,
    //handler: Option<
    //extern "C" fn(response_handle: Cell, user_data: *const Cell, user_data_size: Cell)
    //-> c_void,
    //>,
}

#endif //RESTRY_FFI_H
