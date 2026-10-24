//! Plumbing for `export_plugin!`: borrowing host buffers, decoding arguments, dispatching to the
//! provider, packing results. Hosts keep ownership of argument buffers and free returned ones with
//! `dealloc`. The macro only turns pointers into slices; the rest lives here.

use crate::models::{
    anime::Video, Chapter, MediaEntry, MediaKind, MediaPageResult, Page,
};
use crate::provider::SourceProvider;
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

#[doc(hidden)]
pub fn __listing<T: SourceProvider>(listing: &[u8], page: i32) -> Result<MediaPageResult> {
    T::get_listing(__decode(listing)?, page)
}

#[doc(hidden)]
pub fn __search<T: SourceProvider>(
    query: &[u8],
    page: i32,
    kind: i32,
    filters: &[u8],
) -> Result<MediaPageResult> {
    let kind = __search_kind(kind)?;
    let filters = if filters.is_empty() { Vec::new() } else { __decode(filters)? };
    T::search(__text(query).as_ref(), page, kind, filters)
}

#[doc(hidden)]
pub fn __media_update<T: SourceProvider>(
    entry: &[u8],
    needs_details: i32,
    needs_units: i32,
) -> Result<MediaEntry> {
    T::get_media_update(__decode(entry)?, needs_details != 0, needs_units != 0)
}

#[doc(hidden)]
pub fn __chapter_pages<T: SourceProvider>(entry: &[u8], chapter: &[u8]) -> Result<Vec<Page>> {
    let entry: MediaEntry = __decode(entry)?;
    let chapter: Chapter = __decode(chapter)?;
    T::get_chapter_pages(entry, chapter)
}

#[doc(hidden)]
pub fn __episode_videos<T: SourceProvider>(anime: &[u8], episode: &[u8]) -> Result<Vec<Video>> {
    T::get_episode_videos(__decode(anime)?, __decode(episode)?)
}

#[doc(hidden)]
pub fn __mark_completed<T: SourceProvider>(kind: i32, media_key: &[u8], unit_key: &[u8]) -> Result<()> {
    T::mark_completed(__media_kind(kind)?, __text(media_key).as_ref(), __text(unit_key).as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{ErrorCategory, ErrorReport};
    use crate::models::{
        manga::{ContentRating, Manga, Status, Viewer},
        ContentKind, FilterItem, Listing,
    };
    use std::cell::RefCell;

    thread_local! {
        static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn seen() -> Vec<String> {
        SEEN.with(|seen| seen.borrow_mut().drain(..).collect())
    }

    fn last_report() -> Option<ErrorReport> {
        crate::error::take_last_report().map(|bytes| postcard::from_bytes(&bytes).unwrap())
    }

    /// A source serving every kind that records what reaches it.
    struct Everything;

    impl SourceProvider for Everything {
        const KINDS: &'static [MediaKind] = &[MediaKind::Manga, MediaKind::Anime, MediaKind::Novel];

        fn get_listing(listing: Listing, page: i32) -> Result<MediaPageResult> {
            SEEN.with(|seen| seen.borrow_mut().push(format!("listing {} {page}", listing.id)));
            Ok(MediaPageResult { entries: Vec::new(), has_next_page: false })
        }

        fn search(query: &str, _page: i32, kind: Option<MediaKind>, _filters: Vec<FilterItem>) -> Result<MediaPageResult> {
            SEEN.with(|seen| seen.borrow_mut().push(format!("search {query} {kind:?}")));
            Ok(MediaPageResult { entries: Vec::new(), has_next_page: false })
        }

        fn get_media_update(entry: MediaEntry, _needs_details: bool, _needs_units: bool) -> Result<MediaEntry> {
            Ok(entry)
        }

        fn get_chapter_pages(entry: MediaEntry, chapter: Chapter) -> Result<Vec<Page>> {
            SEEN.with(|seen| seen.borrow_mut().push(format!("pages {} {:?}", entry.key(), chapter.kind)));
            Ok(Vec::new())
        }

        fn mark_completed(kind: MediaKind, media_key: &str, unit_key: &str) -> Result<()> {
            SEEN.with(|seen| seen.borrow_mut().push(format!("completed {kind:?} {media_key} {unit_key}")));
            Ok(())
        }
    }

    crate::export_plugin!(Everything);

    fn manga() -> MediaEntry {
        MediaEntry::Manga(Manga {
            key: "s".into(),
            title: "S".into(),
            authors: None,
            artist: None,
            description: None,
            tags: None,
            cover: None,
            url: None,
            status: Status::Unknown,
            content_rating: ContentRating::Safe,
            nsfw: 0,
            viewer: Viewer::Default,
            chapters: None,
        })
    }

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

    #[test]
    fn exported_kinds_and_abi_are_what_hosts_read() {
        assert_eq!(ito_supported_kinds(), 0b111);
        assert_eq!(crate::ABI_VERSION, 2);
    }

    #[test]
    fn exports_pass_decoded_arguments_through() {
        __search::<Everything>(b"q", 1, -1, &[]).unwrap();
        __search::<Everything>("七".as_bytes(), 1, MediaKind::Novel as i32, &[]).unwrap();
        let listing = postcard::to_allocvec(&Listing { id: "top".into(), name: "Top".into(), kind: 0 }).unwrap();
        __listing::<Everything>(&listing, 3).unwrap();
        let entry = postcard::to_allocvec(&manga()).unwrap();
        let chapter = postcard::to_allocvec(&Chapter {
            key: "c".into(),
            title: None,
            volume: None,
            chapter: None,
            date_updated: None,
            scanlator: None,
            url: None,
            lang: None,
            paywalled: None,
            kind: ContentKind::Text,
        })
        .unwrap();
        __chapter_pages::<Everything>(&entry, &chapter).unwrap();
        assert_eq!(seen(), ["search q None", "search 七 Some(Novel)", "listing top 3", "pages s Text"]);

        assert_eq!(mark_completed(MediaKind::Anime as i32, 0, 0, 0, 0), 1);
        assert_eq!(seen(), ["completed Anime  "]);
    }

    #[test]
    fn a_rejected_argument_is_reported_to_the_host_once() {
        // An ABI 1 chapter has no kind, so an ABI 2 plugin can't read it.
        let entry = postcard::to_allocvec(&manga()).unwrap();
        let legacy_chapter = [1, b'c', 0, 0, 0, 0, 0, 0, 0, 0];
        let failure = __chapter_pages::<Everything>(&entry, &legacy_chapter).unwrap_err();
        assert_eq!(crate::image::__pack_result::<Vec<Page>>("get_chapter_pages", Err(failure)), 0);
        assert_eq!(last_report().unwrap().category, ErrorCategory::Other);
        assert!(last_report().is_none());

        assert_eq!(mark_completed(3, 0, 0, 0, 0), 0);
        assert_eq!(last_report().unwrap().category, ErrorCategory::Other);
        assert_eq!(search(0, 0, 1, 7, 0, 0), 0);
        assert_eq!(last_report().unwrap().category, ErrorCategory::Other);
        assert!(seen().is_empty(), "a rejected argument must not reach the plugin");
    }

    #[test]
    fn an_unsupported_export_is_reported_as_unsupported() {
        let anime = postcard::to_allocvec(&crate::models::anime::Anime {
            key: "a".into(),
            title: "A".into(),
            studios: None,
            description: None,
            tags: None,
            cover: None,
            url: None,
            status: crate::models::anime::Status::Unknown,
            content_rating: crate::models::anime::ContentRating::Safe,
            nsfw: 0,
            episodes: None,
            seasons: None,
        })
        .unwrap();
        let episode = postcard::to_allocvec(&crate::models::anime::Episode {
            key: "e".into(),
            title: None,
            episode: None,
            date_updated: None,
            url: None,
            lang: None,
            paywalled: None,
        })
        .unwrap();
        let unsupported = __episode_videos::<Everything>(&anime, &episode).unwrap_err();
        assert_eq!(crate::image::__pack_result::<Vec<Video>>("get_episode_videos", Err(unsupported)), 0);
        assert_eq!(last_report().unwrap().category, ErrorCategory::Unsupported);
    }
}
