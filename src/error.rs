use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::cell::RefCell;
use std::fmt;

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The service couldn't be reached (no connection, the host cancelled or timed out the request).
    Net(String),
    Html(String),
    Postcard(postcard::Error),
    Utf8(std::string::FromUtf8Error),
    Host(String),
    Unsupported,
    /// The plugin needs settings it doesn't have, such as a server address. Hosts offer to open
    /// the plugin's settings.
    NeedsSetup(String),
    /// The service rejected the plugin's credentials. Hosts offer to open the plugin's settings.
    Unauthorized(String),
    /// The service answered with an error of its own.
    Server(String),
    /// What was asked for doesn't exist.
    NotFound(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Net(msg) => write!(f, "Network error: {}", msg),
            Error::Html(msg) => write!(f, "HTML parsing error: {}", msg),
            Error::Postcard(e) => write!(f, "Serialization error: {}", e),
            Error::Utf8(e) => write!(f, "UTF-8 error: {}", e),
            Error::Host(msg) => write!(f, "Host error: {}", msg),
            Error::Unsupported => write!(f, "Unsupported operation"),
            Error::NeedsSetup(msg)
            | Error::Unauthorized(msg)
            | Error::Server(msg)
            | Error::NotFound(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<postcard::Error> for Error {
    fn from(e: postcard::Error) -> Self {
        Error::Postcard(e)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Error::Utf8(e)
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// What kind of failure an error is, so the host can offer the right way out (open the plugin's
/// settings, check the connection, try again). Hosts read it through `ito_last_error`. Encoded
/// like the other model enums, as an i32 discriminant; the values are the wire format.
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ErrorCategory {
    NeedsSetup = 0,
    Unauthorized = 1,
    Network = 2,
    Server = 3,
    NotFound = 4,
    Other = 5,
    /// The provider explicitly does not participate in this optional operation.
    Unsupported = 6,
}

/// A failure as hosts read it from `ito_last_error`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ErrorReport {
    pub category: ErrorCategory,
    pub message: String,
}

/// Version 2 distinguishes Unsupported from a refusal whose message happens to say
/// "Unsupported operation". Hosts before this export use the legacy message convention.
pub const ERROR_PROTOCOL_VERSION: u32 = 2;

impl Error {
    pub fn category(&self) -> ErrorCategory {
        match self {
            Error::NeedsSetup(_) => ErrorCategory::NeedsSetup,
            Error::Unauthorized(_) => ErrorCategory::Unauthorized,
            Error::Net(_) => ErrorCategory::Network,
            Error::Server(_) => ErrorCategory::Server,
            Error::NotFound(_) => ErrorCategory::NotFound,
            Error::Unsupported => ErrorCategory::Unsupported,
            Error::Html(_) | Error::Postcard(_) | Error::Utf8(_) | Error::Host(_) => {
                ErrorCategory::Other
            }
        }
    }

    /// The text a host shows: the plugin's own sentence where it gave one, without the prefixes
    /// `Display` adds for logs.
    pub fn message(&self) -> String {
        match self {
            Error::Net(msg)
            | Error::Host(msg)
            | Error::NeedsSetup(msg)
            | Error::Unauthorized(msg)
            | Error::Server(msg)
            | Error::NotFound(msg) => msg.clone(),
            _ => self.to_string(),
        }
    }

    pub fn report(&self) -> ErrorReport {
        ErrorReport {
            category: self.category(),
            message: self.message(),
        }
    }
}

thread_local! {
    /// The report of the last failed export, until the host takes it.
    static LAST_ERROR: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
}

/// Reports `error` as the failure of `export`: kept for the host's `ito_last_error` call, and
/// printed as "Error in <export>: <error>" for hosts that read failures from the plugin's output.
/// Used by the `export_*_plugin!` macros.
#[doc(hidden)]
pub fn __report_failure(export: &str, error: &Error) {
    let report = postcard::to_allocvec(&error.report()).ok();
    LAST_ERROR.with(|last| *last.borrow_mut() = report);
    crate::host::print(&format!("Error in {}: {}", export, error));
}

/// Hands the last report to the host once, packed as `(pointer << 32) | length`, or 0 when there
/// is none. The host frees it with `dealloc`, like any export result.
#[doc(hidden)]
pub fn __take_last_error() -> i64 {
    LAST_ERROR.with(|last| match last.borrow_mut().take() {
        Some(bytes) => {
            let bytes = bytes.into_boxed_slice();
            let ptr = bytes.as_ptr() as u64;
            let len = bytes.len() as u64;
            let _ = Box::into_raw(bytes);
            ((ptr << 32) | len) as i64
        }
        None => 0,
    })
}

/// Exports `ito_last_error` and its protocol version. Used by the `export_*_plugin!` macros.
#[doc(hidden)]
#[macro_export]
macro_rules! __export_last_error {
    () => {
        #[unsafe(no_mangle)]
        pub extern "C" fn ito_last_error() -> i64 {
            $crate::error::__take_last_error()
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn ito_error_version() -> u32 {
            $crate::error::ERROR_PROTOCOL_VERSION
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_report_the_category_hosts_act_on_and_the_plugins_own_words() {
        let cases = [
            (Error::NeedsSetup("Add your server URL".into()), ErrorCategory::NeedsSetup, "Add your server URL"),
            (Error::Unauthorized("Wrong API key".into()), ErrorCategory::Unauthorized, "Wrong API key"),
            (Error::Net("No connection".into()), ErrorCategory::Network, "No connection"),
            (Error::Server("HTTP 503".into()), ErrorCategory::Server, "HTTP 503"),
            (Error::NotFound("No such book".into()), ErrorCategory::NotFound, "No such book"),
            (Error::Host("Something else".into()), ErrorCategory::Other, "Something else"),
            (Error::Host("Unsupported operation".into()), ErrorCategory::Other, "Unsupported operation"),
            (Error::Unauthorized("Unsupported operation".into()), ErrorCategory::Unauthorized, "Unsupported operation"),
            (Error::Unsupported, ErrorCategory::Unsupported, "Unsupported operation"),
        ];
        for (error, category, message) in cases {
            assert_eq!(error.report(), ErrorReport { category, message: message.into() });
        }
    }

    #[test]
    fn the_report_wire_format_is_the_zigzag_category_then_the_message() {
        let bytes = postcard::to_allocvec(&Error::NeedsSetup("ab".into()).report()).unwrap_or_default();
        assert_eq!(bytes, vec![0, 2, b'a', b'b']);
        // Postcard writes i32 as zigzag varints: 2 → 4, 5 → 10. Hosts decode it the same way.
        let bytes = postcard::to_allocvec(&Error::Net("x".into()).report()).unwrap_or_default();
        assert_eq!(bytes[0], 4);
        let bytes = postcard::to_allocvec(&Error::Host("x".into()).report()).unwrap_or_default();
        assert_eq!(bytes[0], 10);
        let bytes = postcard::to_allocvec(&Error::Unsupported.report()).unwrap_or_default();
        assert_eq!(bytes[0], 12);
    }
}
