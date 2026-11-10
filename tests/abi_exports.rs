//! ABI 3 changes payload layouts only: every export keeps the name and signature hosts call.
//! Assigning each to an explicitly typed function pointer fails to compile on any change.

use ito_rs::{
    Result,
    models::{Chapter, FilterItem, Listing, MediaEntry, MediaKind, MediaPageResult, Page},
    provider::SourceProvider,
};

struct Provider;

impl SourceProvider for Provider {
    const KINDS: &'static [MediaKind] = &[MediaKind::Manga];

    fn get_listing(_: Listing, _: i32) -> Result<MediaPageResult> {
        Ok(MediaPageResult {
            entries: Vec::new(),
            has_next_page: false,
        })
    }

    fn search(
        _: &str,
        _: i32,
        _: Option<MediaKind>,
        _: Vec<FilterItem>,
    ) -> Result<MediaPageResult> {
        Ok(MediaPageResult {
            entries: Vec::new(),
            has_next_page: false,
        })
    }

    fn get_media_update(entry: MediaEntry, _: bool, _: bool) -> Result<MediaEntry> {
        Ok(entry)
    }

    fn get_chapter_pages(_: MediaEntry, _: Chapter) -> Result<Vec<Page>> {
        Ok(Vec::new())
    }
}

ito_rs::export_plugin!(Provider);

#[test]
fn every_export_keeps_its_name_and_signature() {
    let _: extern "C" fn() -> u32 = ito_abi_version;
    let _: extern "C" fn() -> u32 = ito_supported_kinds;
    let _: extern "C" fn() -> u32 = ito_error_version;
    let _: extern "C" fn() -> i64 = ito_last_error;
    let _: extern "C" fn() -> i64 = get_home;
    let _: extern "C" fn() -> i64 = get_settings;
    let _: extern "C" fn() -> i64 = get_settings_actions;
    let _: extern "C" fn(i32, i32) -> i64 = run_settings_action;
    let _: extern "C" fn() -> i64 = validate_settings;
    let _: extern "C" fn() -> i64 = get_settings_form;
    let _: extern "C" fn(i32) -> i64 = get_search_capabilities;
    let _: extern "C" fn(i32, i32) -> i64 = get_option_page;
    let _: extern "C" fn(i32, i32, i32) -> i64 = get_search_suggestions;
    let _: extern "C" fn(i32, i32) -> i64 = handle_url;
    let _: extern "C" fn(i32, i32) -> i64 = handle_image;
    let _: extern "C" fn() -> i64 = get_image_origins;
    let _: extern "C" fn(i32, i32) -> i64 = get_image_request;
    let _: extern "C" fn(i32, i32, i32) -> i64 = get_listing;
    let _: extern "C" fn(i32, i32, i32, i32, i32, i32) -> i64 = search;
    let _: extern "C" fn(i32, i32, i32, i32) -> i64 = get_media_update;
    let _: extern "C" fn(i32, i32, i32, i32) -> i64 = get_chapter_pages;
    let _: extern "C" fn(i32, i32, i32, i32) -> i64 = get_episode_videos;
    let _: extern "C" fn(i32, i32, i32, i32, i32) -> i32 = mark_completed;
    let _: extern "C" fn(usize) -> *mut u8 = ito_rs::ffi_alloc::alloc;
    let _: unsafe extern "C" fn(*mut u8, usize) = ito_rs::ffi_alloc::dealloc;
}

#[test]
fn the_exported_abi_version_is_three() {
    assert_eq!(ito_abi_version(), 3);
    assert_eq!(ito_rs::ABI_VERSION, 3);
}
