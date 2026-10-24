//! Optional settings diagnostics, separate from the ABI-1 settings schema.
//!
//! `get_settings_actions` and `validate_settings` return packed postcard vectors;
//! `run_settings_action` borrows a caller-owned UTF-8 id and returns a packed postcard String.
//! Hosts free returned buffers with `dealloc` and retain ownership of the input buffer.
//! A successful empty vector or string still has an encoded length byte; zero means failure
//! reported through `ito_last_error`. Unsupported is optional non-participation, not validation
//! success. Hosts must not execute actions automatically or persist their temporary settings.
//! Older hosts ignore these exports; Setting and SettingsSchema retain their ABI-1 layout.

/// Emits the settings discovery, diagnostic action, and local validation exports.
/// Uses the existing packed-result protocol, including nonzero empty successful results.
#[doc(hidden)]
#[macro_export]
macro_rules! __export_settings_actions {
    ($type:ty, $provider:path) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn get_settings_actions() -> i64 {
            $crate::image::__pack_result(
                "get_settings_actions",
                <$type as $provider>::get_settings_actions(),
            )
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn run_settings_action(id_ptr: i32, id_len: i32) -> i64 {
            let bytes = if id_ptr == 0 || id_len <= 0 {
                &[]
            } else {
                unsafe { core::slice::from_raw_parts(id_ptr as *const u8, id_len as usize) }
            };
            let id = String::from_utf8_lossy(bytes);
            $crate::image::__pack_result(
                "run_settings_action",
                <$type as $provider>::run_settings_action(id.as_ref()),
            )
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn validate_settings() -> i64 {
            $crate::image::__pack_result(
                "validate_settings",
                <$type as $provider>::validate_settings(),
            )
        }
    };
}
