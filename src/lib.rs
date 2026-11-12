//! Source-provider SDK for Ito plugins.
//!
//! Version 0.8 is ABI 3: chapters carry `f64` numbers, a scanlator with an optional stable
//! identifier, and optional `logical_unit_id`, `reading_order` and `revision` fields, and
//! manga and novel entries describe what their chapter list covers with a `ChapterCatalog`.
//! Plugins built for ABI 1 or 2 must be ported; see `docs/ABI3-MIGRATION.md`. Rich settings,
//! paged option catalogs and search capabilities (see [`controls`]) are unchanged.
//! Providers may retain legacy settings for older hosts; the rich-control exports opt out
//! with [`Error::Unsupported`] unless implemented. Sources that can list every alternative upload
//! of a chapter, page by page, may implement the optional `get_chapter_editions` export (see
//! [`editions`]); it adds no ABI version.

pub mod catalog;
pub mod controls;
pub mod editions;

#[doc(hidden)]
pub mod completion;
pub mod defaults;
pub mod env;
pub mod error;
#[doc(hidden)]
pub mod export;
pub mod ffi_alloc;
pub mod host;
pub mod html;
#[doc(hidden)]
pub mod image;
pub mod models;
pub mod net;
pub mod provider;
#[doc(hidden)]
pub mod settings_action;
pub mod webview;

pub use error::{Error, Result};
pub use postcard;

/// The version of the binary contract between plugins and hosts: how models are laid out on the
/// wire, which values a host must decode, and how exports pass and return them. Plugins export it
/// as `ito_abi_version` and hosts refuse plugins built for a version they don't support. Bump it
/// when a host must understand something new to run plugins built with this SDK; a new host
/// function needs no bump, since hosts check a plugin's imports before loading it.
///
/// Version 2: entries carry a `MediaKind` and chapters a `ContentKind`; plugins export
/// `ito_supported_kinds` and one kind-neutral set of exports (see `provider`).
///
/// Version 3: `Chapter` numbers are `f64`, its scanlator is an optional `Scanlator` (stable
/// identifier plus display name) and it gains `logical_unit_id`, `reading_order` and
/// `revision`; `Manga` and `Novel` gain an optional `ChapterCatalog`. Export signatures are
/// unchanged. Hosts supporting ABI 3 refuse older plugins, and the SDK validates returned
/// chapter lists against the limits in [`catalog`].
pub const ABI_VERSION: u32 = 3;

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
