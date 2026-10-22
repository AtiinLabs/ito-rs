pub mod defaults;
pub mod env;
pub mod error;
pub mod ffi_alloc;
pub mod host;
pub mod html;
#[doc(hidden)]
pub mod image;
pub mod models;
pub mod net;
pub mod provider;
pub mod ui;
pub mod webview;

pub use error::{Error, Result};
pub use postcard;

/// The version of the binary contract between plugins and hosts: how models are laid out on the
/// wire, which values a host must decode, and how exports pass and return them. Plugins export it
/// as `ito_abi_version` and hosts refuse plugins built for a version they don't support. Bump it
/// when a host must understand something new to run plugins built with this SDK; a new host
/// function needs no bump, since hosts check a plugin's imports before loading it.
pub const ABI_VERSION: u32 = 1;

/// Exports `ito_abi_version`. Used by the `export_*_plugin!` macros.
#[doc(hidden)]
#[macro_export]
macro_rules! __export_abi_version {
    () => {
        #[unsafe(no_mangle)]
        pub extern "C" fn ito_abi_version() -> u32 {
            $crate::ABI_VERSION
        }
    };
}
