//! Optional, idempotent completion notifications (`mark_completed`).
//!
//! The export borrows two caller-owned UTF-8 key buffers and returns an `i32` acknowledgement:
//! `1` means success; `0` means failure, reported through `ito_last_error`. Genuine Unsupported
//! uses its explicit protocol-2 category.

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
