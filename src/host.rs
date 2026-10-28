#[link(wasm_import_module = "ito:core/net")]
unsafe extern "C" {
    pub fn fetch(ptr: i32, len: i32) -> i32;
    pub fn fetch_v2(req_ptr: i32, req_len: i32, opt_ptr: i32, opt_len: i32) -> i32;
    pub fn fetch_read(ptr: i32);
}

#[link(wasm_import_module = "ito:core/html")]
unsafe extern "C" {
    pub fn parse(ptr: i32, len: i32) -> i32;
    pub fn select(id: i32, ptr: i32, len: i32) -> i64;
    pub fn text(id: i32) -> i64;
    pub fn attr(id: i32, ptr: i32, len: i32) -> i64;
    // A native test binary must not bind this handle-based import to the C allocator.
    #[cfg_attr(not(target_arch = "wasm32"), link_name = "ito_html_free")]
    pub fn free(id: i32);
    pub fn html_content(id: i32) -> i64;
    pub fn outer_html(id: i32) -> i64;
    pub fn own_text(id: i32) -> i64;
}

#[link(wasm_import_module = "ito:core/std")]
unsafe extern "C" {
    #[link_name = "print"]
    pub fn sys_print(msg_ptr: i32, msg_len: i32);
}

#[link(wasm_import_module = "ito:core/defaults")]
unsafe extern "C" {
    pub fn set(key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32);
    pub fn get(key_ptr: i32, key_len: i32) -> i64;
    pub fn remove(key_ptr: i32, key_len: i32);
    pub fn set_secret(key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32);
}

#[link(wasm_import_module = "ito:core/webview")]
unsafe extern "C" {
    pub fn webview_load_url(ptr: i32, len: i32) -> i32;
    pub fn webview_execute_js(ptr: i32, len: i32) -> i32;
    pub fn webview_read_result(ptr: i32);
}

/// Prints to the host's log. Off wasm (the SDK's own tests) there is no host, so nothing is printed.
pub fn print(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let msg_bytes = msg.as_bytes();
        unsafe {
            sys_print(msg_bytes.as_ptr() as i32, msg_bytes.len() as i32);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = msg;
}
