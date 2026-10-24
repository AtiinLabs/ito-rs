//! Raw plumbing for `export_plugin!`: borrowing host buffers, decoding arguments, packing
//! results. Hosts keep ownership of argument buffers and free returned ones with `dealloc`.

use crate::models::MediaKind;
use crate::{Error, Result};
use serde::de::DeserializeOwned;
use std::borrow::Cow;

/// Borrows a host-owned argument buffer; a null pointer or non-positive length is empty.
///
/// # Safety
/// `ptr` must address `len` readable bytes that outlive the returned slice.
#[doc(hidden)]
pub unsafe fn __bytes<'a>(ptr: i32, len: i32) -> &'a [u8] {
    if ptr == 0 || len <= 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(ptr as *const u8, len as usize) }
    }
}

#[doc(hidden)]
pub fn __text(bytes: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

#[doc(hidden)]
pub fn __decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    postcard::from_bytes(bytes).map_err(Error::from)
}

/// Packs raw bytes, such as `handle_image` output. Unsupported returns 0 without a report, so
/// hosts fall back to fetching the image themselves.
#[doc(hidden)]
pub fn __pack_bytes(export: &str, result: Result<Vec<u8>>) -> i64 {
    match result {
        Ok(bytes) => {
            let bytes = bytes.into_boxed_slice();
            let ptr = bytes.as_ptr() as u64;
            let len = bytes.len() as u64;
            let _ = Box::into_raw(bytes);
            ((ptr << 32) | len) as i64
        }
        Err(Error::Unsupported) => 0,
        Err(error) => {
            crate::error::__report_failure(export, &error);
            0
        }
    }
}

/// A host's `MediaKind` argument.
#[doc(hidden)]
pub fn __media_kind(value: i32) -> Result<MediaKind> {
    match value {
        0 => Ok(MediaKind::Manga),
        1 => Ok(MediaKind::Anime),
        2 => Ok(MediaKind::Novel),
        _ => Err(Error::Host(format!("Unknown media kind {value}"))),
    }
}

/// A host's search kind: -1 for every kind the source serves.
#[doc(hidden)]
pub fn __search_kind(value: i32) -> Result<Option<MediaKind>> {
    if value == -1 { Ok(None) } else { __media_kind(value).map(Some) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_kind_arguments_use_the_wire_values() {
        assert_eq!(__media_kind(MediaKind::Manga as i32).unwrap(), MediaKind::Manga);
        assert_eq!(__media_kind(MediaKind::Anime as i32).unwrap(), MediaKind::Anime);
        assert_eq!(__media_kind(MediaKind::Novel as i32).unwrap(), MediaKind::Novel);
        assert!(matches!(__media_kind(3), Err(Error::Host(_))));
        assert!(matches!(__media_kind(-1), Err(Error::Host(_))));
        assert_eq!(__search_kind(-1).unwrap(), None);
        assert_eq!(__search_kind(2).unwrap(), Some(MediaKind::Novel));
        assert!(__search_kind(-2).is_err());
    }
}
