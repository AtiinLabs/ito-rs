//! Optional, idempotent completion notifications emitted by the provider macros.
//!
//! Exports borrow two caller-owned UTF-8 key buffers and return an `i32` acknowledgement:
//! `1` means success; `0` means failure, reported through `ito_last_error`. Genuine Unsupported
//! uses its explicit protocol-2 category. Older hosts ignore these optional exports.

use crate::Result;

/// Acknowledges without serializing or allocating a unit response.
#[doc(hidden)]
pub fn __ack_result(export: &str, result: Result<()>) -> i32 {
    match result {
        Ok(()) => 1,
        Err(error) => {
            crate::error::__report_failure(export, &error);
            0
        }
    }
}

/// Emits one provider's optional completion export. The caller retains both argument buffers.
#[doc(hidden)]
#[macro_export]
macro_rules! __export_completion {
    ($type:ty, $provider:path, $export:ident) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $export(
            media_key_ptr: i32,
            media_key_len: i32,
            unit_key_ptr: i32,
            unit_key_len: i32,
        ) -> i32 {
            let media_bytes = if media_key_ptr == 0 || media_key_len <= 0 {
                &[]
            } else {
                unsafe {
                    ::core::slice::from_raw_parts(
                        media_key_ptr as *const u8,
                        media_key_len as usize,
                    )
                }
            };
            let unit_bytes = if unit_key_ptr == 0 || unit_key_len <= 0 {
                &[]
            } else {
                unsafe {
                    ::core::slice::from_raw_parts(unit_key_ptr as *const u8, unit_key_len as usize)
                }
            };
            let media_key = ::std::string::String::from_utf8_lossy(media_bytes);
            let unit_key = ::std::string::String::from_utf8_lossy(unit_bytes);
            $crate::completion::__ack_result(
                ::core::stringify!($export),
                <$type as $provider>::$export(media_key.as_ref(), unit_key.as_ref()),
            )
        }
    };
}
