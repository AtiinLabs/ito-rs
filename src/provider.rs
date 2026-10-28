//! Provider traits and the export macros that turn one into a plugin.
//!
//! A source serving several kinds implements `SourceProvider` and calls `export_plugin!`. A
//! source serving one kind may instead implement `MangaProvider`, `AnimeProvider` or
//! `NovelProvider` and call the matching `export_*_plugin!`; it is exported through the same
//! `SourceProvider` interface.
//!
//! Every plugin exports (ABI 2), besides the allocator, `ito_last_error`, the image and settings
//! diagnostics exports:
//! - `ito_abi_version() -> u32` and `ito_supported_kinds() -> u32` (a `MediaKind::mask`);
//! - `get_home`, `get_settings`, `handle_url(url)`, `handle_image(url)`;
//! - `get_listing(listing, page) -> MediaPageResult`;
//! - `search(query, page, kind, filters) -> MediaPageResult`, `kind` a `MediaKind` value or -1
//!   for every kind the source serves;
//! - optional rich-control exports: `get_settings_form`, `get_search_capabilities(kind)`,
//!   `get_option_page(request)`, and `get_search_suggestions(query, kind)`. Unsupported means absent.
//! - `get_media_update(entry, needs_details, needs_units) -> MediaEntry`;
//! - `get_chapter_pages(entry, chapter) -> Vec<Page>`;
//! - `get_episode_videos(anime, episode) -> Vec<Video>`;
//! - `mark_completed(kind, media_key, unit_key) -> i32` (1 on success).

use crate::controls::{
    OptionPage, OptionRequest, SearchCapabilities, SearchSuggestion, SettingsForm,
};
use crate::models::{
    Chapter, FilterItem, HomeLayout, Listing, MediaEntry, MediaKind, MediaPageResult, Page,
    anime::{self, Anime, Episode, Video},
    manga::{self, Manga},
    novel::{self, Novel},
};
use crate::{Error, Result};
use std::marker::PhantomData;

/// A source serving one or more kinds of media.
pub trait SourceProvider {
    /// The kinds this source serves; must not be empty. Hosts only ask for these kinds.
    const KINDS: &'static [MediaKind];

    fn get_home() -> Result<HomeLayout> {
        Ok(HomeLayout {
            components: Vec::new(),
        })
    }

    fn get_settings() -> Option<crate::models::SettingsSchema> {
        None
    }

    /// Non-destructive settings diagnostics. See MangaProvider::get_settings_actions.
    fn get_settings_actions() -> Result<Vec<crate::models::SettingsAction>> {
        Ok(Vec::new())
    }

    /// Runs a temporary settings diagnostic. See MangaProvider::run_settings_action.
    fn run_settings_action(_action_id: &str) -> Result<String> {
        Err(Error::Unsupported)
    }

    /// Advisory, local-only validation. See MangaProvider::validate_settings.
    fn validate_settings() -> Result<Vec<crate::models::SettingIssue>> {
        Err(Error::Unsupported)
    }

    /// Returns a local rich settings form; Unsupported lets hosts use `get_settings`.
    fn get_settings_form() -> Result<SettingsForm> {
        Err(Error::Unsupported)
    }
    /// Declares the search controls for one media kind, or every kind when `kind` is `None`.
    fn get_search_capabilities(_kind: Option<MediaKind>) -> Result<SearchCapabilities> {
        Err(Error::Unsupported)
    }
    /// Resolves an inline option catalog page or selected option labels.
    fn get_option_page(_request: OptionRequest) -> Result<OptionPage> {
        Err(Error::Unsupported)
    }
    /// Returns bounded query completions for the requested kind.
    fn get_search_suggestions(
        _query: &str,
        _kind: Option<MediaKind>,
    ) -> Result<Vec<SearchSuggestion>> {
        Err(Error::Unsupported)
    }
    fn get_listing(listing: Listing, page: i32) -> Result<MediaPageResult>;

    /// `kind` narrows search to a served kind; `None` searches every served kind. Hosts send
    /// canonical `FilterItem` values when rich search capabilities are available, otherwise `[]`.
    fn search(
        query: &str,
        page: i32,
        kind: Option<MediaKind>,
        filters: Vec<FilterItem>,
    ) -> Result<MediaPageResult>;

    /// Returns `entry` with its details and/or its chapters or episodes filled in. The result
    /// must be the same kind as `entry`.
    fn get_media_update(
        entry: MediaEntry,
        needs_details: bool,
        needs_units: bool,
    ) -> Result<MediaEntry>;

    /// The pages of a manga or novel chapter, read as the chapter's `kind` says.
    fn get_chapter_pages(_entry: MediaEntry, _chapter: Chapter) -> Result<Vec<Page>> {
        Err(Error::Unsupported)
    }

    fn get_episode_videos(_anime: Anime, _episode: Episode) -> Result<Vec<Video>> {
        Err(Error::Unsupported)
    }

    /// Reports completion of a chapter or episode. See MangaProvider::mark_chapter_read.
    fn mark_completed(_kind: MediaKind, _media_key: &str, _unit_key: &str) -> Result<()> {
        Err(Error::Unsupported)
    }

    /// Maps a web address on this plugin's site to a title. See MangaProvider::handle_url.
    fn handle_url(_url: &str) -> Result<crate::models::LinkValue> {
        Err(Error::Unsupported)
    }

    fn handle_image(_url: &str) -> Result<Vec<u8>> {
        Err(Error::Unsupported)
    }

    /// Configured image origins, independent of image input. See MangaProvider::get_image_origins.
    fn get_image_origins() -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// Resolves a native image request. See MangaProvider::get_image_request.
    fn get_image_request(_url: &str) -> Result<crate::models::PluginImageRequest> {
        Err(Error::Unsupported)
    }
}

pub trait MangaProvider {
    fn get_home() -> Result<HomeLayout> {
        Ok(HomeLayout {
            components: Vec::new(),
        })
    }

    fn get_settings() -> Option<crate::models::SettingsSchema> {
        None
    }

    /// Non-destructive settings diagnostics. Older hosts ignore these optional exports.
    fn get_settings_actions() -> Result<Vec<crate::models::SettingsAction>> {
        Ok(Vec::new())
    }

    /// Runs against the host's temporary settings snapshot, which may include unsaved values.
    /// May read the network, but must not change remote state or require an interactive login.
    /// Local writes are temporary. Return a user-facing sentence without setting values.
    fn run_settings_action(_action_id: &str) -> Result<String> {
        Err(crate::Error::Unsupported)
    }

    /// Advisory, local-only validation: read settings without network requests or writes.
    /// An empty list means valid; Unsupported means validation is not implemented.
    fn validate_settings() -> Result<Vec<crate::models::SettingIssue>> {
        Err(crate::Error::Unsupported)
    }

    /// Returns a rich settings form, or `Unsupported` to retain the legacy schema.
    fn get_settings_form() -> Result<SettingsForm> {
        Err(Error::Unsupported)
    }
    /// Declares search controls for this single-kind provider.
    fn get_search_capabilities() -> Result<SearchCapabilities> {
        Err(Error::Unsupported)
    }
    /// Serves an option-catalog page or selected-value labels.
    fn get_option_page(_request: OptionRequest) -> Result<OptionPage> {
        Err(Error::Unsupported)
    }
    /// Suggests completions for a search query.
    fn get_search_suggestions(_query: &str) -> Result<Vec<SearchSuggestion>> {
        Err(Error::Unsupported)
    }
    fn get_manga_list(listing: Listing, page: i32) -> Result<manga::PageResult>;

    /// `filters` carries canonical rich-control strings when supported; otherwise it is empty.
    fn get_search_manga_list(
        query: &str,
        page: i32,
        filters: Vec<FilterItem>,
    ) -> Result<manga::PageResult>;

    fn get_manga_update(manga: Manga, needs_details: bool, needs_chapters: bool) -> Result<Manga>;

    fn get_page_list(manga: Manga, chapter: Chapter) -> Result<Vec<Page>>;

    /// Reports completion of a chapter. Delivery may be retried: implementations must be
    /// idempotent and must not lower remote progress. Unsupported opts out.
    fn mark_chapter_read(_media_key: &str, _chapter_key: &str) -> Result<()> {
        Err(crate::Error::Unsupported)
    }

    /// Maps a web address on this plugin's site (the manifest `url` host or a subdomain) to
    /// the title it shows. Ito offers links pasted into Search; it opens `Manga`, `Anime` and
    /// `Novel` values, loading details from the returned key. Unsupported opts out.
    fn handle_url(_url: &str) -> Result<crate::models::LinkValue> {
        Err(crate::Error::Unsupported)
    }

    fn handle_image(_url: &str) -> Result<Vec<u8>> {
        Err(crate::Error::Unsupported)
    }

    /// Origins this plugin currently permits for native image fetching, derived from its
    /// configuration, not from a supplied image URL. Return absolute HTTP(S) origins; the host
    /// compares scheme, host and effective port and rejects cross-origin redirects.
    fn get_image_origins() -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// Resolves a plugin-owned image locator into a native request without downloading bytes.
    /// Return Unsupported for images requiring legacy `handle_image` processing. Any other
    /// failure is a refusal: the host must not fall back to another fetch path.
    fn get_image_request(_url: &str) -> Result<crate::models::PluginImageRequest> {
        Err(crate::Error::Unsupported)
    }
}

pub trait AnimeProvider {
    fn get_home() -> Result<HomeLayout> {
        Ok(HomeLayout {
            components: Vec::new(),
        })
    }

    fn get_settings() -> Option<crate::models::SettingsSchema> {
        None
    }

    /// Non-destructive settings diagnostics. See MangaProvider::get_settings_actions.
    fn get_settings_actions() -> Result<Vec<crate::models::SettingsAction>> {
        Ok(Vec::new())
    }

    /// Runs a temporary settings diagnostic. See MangaProvider::run_settings_action.
    fn run_settings_action(_action_id: &str) -> Result<String> {
        Err(crate::Error::Unsupported)
    }

    /// Advisory, local-only validation. See MangaProvider::validate_settings.
    fn validate_settings() -> Result<Vec<crate::models::SettingIssue>> {
        Err(crate::Error::Unsupported)
    }
    /// Returns a rich settings form, or `Unsupported` to retain the legacy schema.
    fn get_settings_form() -> Result<SettingsForm> {
        Err(Error::Unsupported)
    }
    /// Declares search controls for this single-kind provider.
    fn get_search_capabilities() -> Result<SearchCapabilities> {
        Err(Error::Unsupported)
    }
    /// Serves an option-catalog page or selected-value labels.
    fn get_option_page(_request: OptionRequest) -> Result<OptionPage> {
        Err(Error::Unsupported)
    }
    /// Suggests completions for a search query.
    fn get_search_suggestions(_query: &str) -> Result<Vec<SearchSuggestion>> {
        Err(Error::Unsupported)
    }

    fn get_anime_list(listing: Listing, page: i32) -> Result<anime::PageResult>;

    /// `filters` contains rich-control canonical strings when supported; otherwise it is empty.
    fn get_search_anime_list(
        query: &str,
        page: i32,
        filters: Vec<FilterItem>,
    ) -> Result<anime::PageResult>;

    fn get_anime_update(anime: Anime, needs_details: bool, needs_episodes: bool) -> Result<Anime>;

    fn get_video_list(anime: Anime, episode: Episode) -> Result<Vec<Video>>;

    /// Reports completion of an episode. Delivery may be retried: implementations must be
    /// idempotent and must not lower remote progress. Unsupported opts out.
    fn mark_episode_watched(_media_key: &str, _episode_key: &str) -> Result<()> {
        Err(crate::Error::Unsupported)
    }

    /// Maps a web address on this plugin's site to a title. See MangaProvider::handle_url.
    fn handle_url(_url: &str) -> Result<crate::models::LinkValue> {
        Err(crate::Error::Unsupported)
    }

    /// Configured image origins, independent of image input. See MangaProvider::get_image_origins.
    fn get_image_origins() -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// Resolves a native image request. See MangaProvider::get_image_request.
    fn get_image_request(_url: &str) -> Result<crate::models::PluginImageRequest> {
        Err(crate::Error::Unsupported)
    }
}

pub trait NovelProvider {
    fn get_home() -> Result<HomeLayout> {
        Ok(HomeLayout {
            components: Vec::new(),
        })
    }

    fn get_settings() -> Option<crate::models::SettingsSchema> {
        None
    }

    /// Non-destructive settings diagnostics. See MangaProvider::get_settings_actions.
    fn get_settings_actions() -> Result<Vec<crate::models::SettingsAction>> {
        Ok(Vec::new())
    }

    /// Runs a temporary settings diagnostic. See MangaProvider::run_settings_action.
    fn run_settings_action(_action_id: &str) -> Result<String> {
        Err(crate::Error::Unsupported)
    }

    /// Advisory, local-only validation. See MangaProvider::validate_settings.
    fn validate_settings() -> Result<Vec<crate::models::SettingIssue>> {
        Err(crate::Error::Unsupported)
    }
    /// Returns a rich settings form, or `Unsupported` to retain the legacy schema.
    fn get_settings_form() -> Result<SettingsForm> {
        Err(Error::Unsupported)
    }
    /// Declares search controls for this single-kind provider.
    fn get_search_capabilities() -> Result<SearchCapabilities> {
        Err(Error::Unsupported)
    }
    /// Serves an option-catalog page or selected-value labels.
    fn get_option_page(_request: OptionRequest) -> Result<OptionPage> {
        Err(Error::Unsupported)
    }
    /// Suggests completions for a search query.
    fn get_search_suggestions(_query: &str) -> Result<Vec<SearchSuggestion>> {
        Err(Error::Unsupported)
    }

    fn get_novel_list(listing: Listing, page: i32) -> Result<novel::PageResult>;

    /// `filters` contains rich-control canonical strings when supported; otherwise it is empty.
    fn get_search_novel_list(
        query: &str,
        page: i32,
        filters: Vec<FilterItem>,
    ) -> Result<novel::PageResult>;

    fn get_novel_update(novel: Novel, needs_details: bool, needs_chapters: bool) -> Result<Novel>;

    fn get_chapter_content(novel: Novel, chapter: Chapter) -> Result<Vec<Page>>;

    /// Reports completion of a chapter. See MangaProvider::mark_chapter_read.
    fn mark_chapter_read(_media_key: &str, _chapter_key: &str) -> Result<()> {
        Err(crate::Error::Unsupported)
    }

    /// Maps a web address on this plugin's site to a title. See MangaProvider::handle_url.
    fn handle_url(_url: &str) -> Result<crate::models::LinkValue> {
        Err(crate::Error::Unsupported)
    }

    /// Configured image origins, independent of image input. See MangaProvider::get_image_origins.
    fn get_image_origins() -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// Resolves a native image request. See MangaProvider::get_image_request.
    fn get_image_request(_url: &str) -> Result<crate::models::PluginImageRequest> {
        Err(crate::Error::Unsupported)
    }
}

fn empty_page() -> MediaPageResult {
    MediaPageResult {
        entries: Vec::new(),
        has_next_page: false,
    }
}

/// Exports a `MangaProvider` as a source serving manga.
pub struct MangaSource<T>(PhantomData<fn() -> T>);

impl<T: MangaProvider> SourceProvider for MangaSource<T> {
    const KINDS: &'static [MediaKind] = &[MediaKind::Manga];

    fn get_home() -> Result<HomeLayout> {
        T::get_home()
    }

    fn get_settings() -> Option<crate::models::SettingsSchema> {
        T::get_settings()
    }
    fn get_settings_form() -> Result<SettingsForm> {
        T::get_settings_form()
    }
    fn get_search_capabilities(kind: Option<MediaKind>) -> Result<SearchCapabilities> {
        if kind.is_some_and(|k| k != MediaKind::Manga) {
            Err(Error::Unsupported)
        } else {
            T::get_search_capabilities()
        }
    }
    fn get_option_page(request: OptionRequest) -> Result<OptionPage> {
        T::get_option_page(request)
    }
    fn get_search_suggestions(
        query: &str,
        kind: Option<MediaKind>,
    ) -> Result<Vec<SearchSuggestion>> {
        if kind.is_some_and(|k| k != MediaKind::Manga) {
            Err(Error::Unsupported)
        } else {
            T::get_search_suggestions(query)
        }
    }

    fn get_settings_actions() -> Result<Vec<crate::models::SettingsAction>> {
        T::get_settings_actions()
    }

    fn run_settings_action(action_id: &str) -> Result<String> {
        T::run_settings_action(action_id)
    }

    fn validate_settings() -> Result<Vec<crate::models::SettingIssue>> {
        T::validate_settings()
    }

    fn get_listing(listing: Listing, page: i32) -> Result<MediaPageResult> {
        T::get_manga_list(listing, page).map(Into::into)
    }

    fn search(
        query: &str,
        page: i32,
        kind: Option<MediaKind>,
        filters: Vec<FilterItem>,
    ) -> Result<MediaPageResult> {
        if kind.is_some_and(|kind| kind != MediaKind::Manga) {
            return Ok(empty_page());
        }
        T::get_search_manga_list(query, page, filters).map(Into::into)
    }

    fn get_media_update(
        entry: MediaEntry,
        needs_details: bool,
        needs_units: bool,
    ) -> Result<MediaEntry> {
        match entry {
            MediaEntry::Manga(manga) => {
                T::get_manga_update(manga, needs_details, needs_units).map(MediaEntry::Manga)
            }
            _ => Err(Error::Unsupported),
        }
    }

    fn get_chapter_pages(entry: MediaEntry, chapter: Chapter) -> Result<Vec<Page>> {
        match entry {
            MediaEntry::Manga(manga) => T::get_page_list(manga, chapter),
            _ => Err(Error::Unsupported),
        }
    }

    fn mark_completed(kind: MediaKind, media_key: &str, unit_key: &str) -> Result<()> {
        match kind {
            MediaKind::Manga => T::mark_chapter_read(media_key, unit_key),
            _ => Err(Error::Unsupported),
        }
    }

    fn handle_url(url: &str) -> Result<crate::models::LinkValue> {
        T::handle_url(url)
    }

    fn handle_image(url: &str) -> Result<Vec<u8>> {
        T::handle_image(url)
    }

    fn get_image_origins() -> Result<Vec<String>> {
        T::get_image_origins()
    }

    fn get_image_request(url: &str) -> Result<crate::models::PluginImageRequest> {
        T::get_image_request(url)
    }
}

/// Exports a `AnimeProvider` as a source serving anime.
pub struct AnimeSource<T>(PhantomData<fn() -> T>);

impl<T: AnimeProvider> SourceProvider for AnimeSource<T> {
    const KINDS: &'static [MediaKind] = &[MediaKind::Anime];

    fn get_home() -> Result<HomeLayout> {
        T::get_home()
    }

    fn get_settings() -> Option<crate::models::SettingsSchema> {
        T::get_settings()
    }
    fn get_settings_form() -> Result<SettingsForm> {
        T::get_settings_form()
    }
    fn get_search_capabilities(kind: Option<MediaKind>) -> Result<SearchCapabilities> {
        if kind.is_some_and(|k| k != MediaKind::Anime) {
            Err(Error::Unsupported)
        } else {
            T::get_search_capabilities()
        }
    }
    fn get_option_page(request: OptionRequest) -> Result<OptionPage> {
        T::get_option_page(request)
    }
    fn get_search_suggestions(
        query: &str,
        kind: Option<MediaKind>,
    ) -> Result<Vec<SearchSuggestion>> {
        if kind.is_some_and(|k| k != MediaKind::Anime) {
            Err(Error::Unsupported)
        } else {
            T::get_search_suggestions(query)
        }
    }

    fn get_settings_actions() -> Result<Vec<crate::models::SettingsAction>> {
        T::get_settings_actions()
    }

    fn run_settings_action(action_id: &str) -> Result<String> {
        T::run_settings_action(action_id)
    }

    fn validate_settings() -> Result<Vec<crate::models::SettingIssue>> {
        T::validate_settings()
    }

    fn get_listing(listing: Listing, page: i32) -> Result<MediaPageResult> {
        T::get_anime_list(listing, page).map(Into::into)
    }

    fn search(
        query: &str,
        page: i32,
        kind: Option<MediaKind>,
        filters: Vec<FilterItem>,
    ) -> Result<MediaPageResult> {
        if kind.is_some_and(|kind| kind != MediaKind::Anime) {
            return Ok(empty_page());
        }
        T::get_search_anime_list(query, page, filters).map(Into::into)
    }

    fn get_media_update(
        entry: MediaEntry,
        needs_details: bool,
        needs_units: bool,
    ) -> Result<MediaEntry> {
        match entry {
            MediaEntry::Anime(anime) => {
                T::get_anime_update(anime, needs_details, needs_units).map(MediaEntry::Anime)
            }
            _ => Err(Error::Unsupported),
        }
    }

    fn get_episode_videos(anime: Anime, episode: Episode) -> Result<Vec<Video>> {
        T::get_video_list(anime, episode)
    }

    fn mark_completed(kind: MediaKind, media_key: &str, unit_key: &str) -> Result<()> {
        match kind {
            MediaKind::Anime => T::mark_episode_watched(media_key, unit_key),
            _ => Err(Error::Unsupported),
        }
    }

    fn handle_url(url: &str) -> Result<crate::models::LinkValue> {
        T::handle_url(url)
    }

    fn get_image_origins() -> Result<Vec<String>> {
        T::get_image_origins()
    }

    fn get_image_request(url: &str) -> Result<crate::models::PluginImageRequest> {
        T::get_image_request(url)
    }
}

/// Exports a `NovelProvider` as a source serving novels.
pub struct NovelSource<T>(PhantomData<fn() -> T>);

impl<T: NovelProvider> SourceProvider for NovelSource<T> {
    const KINDS: &'static [MediaKind] = &[MediaKind::Novel];

    fn get_home() -> Result<HomeLayout> {
        T::get_home()
    }

    fn get_settings() -> Option<crate::models::SettingsSchema> {
        T::get_settings()
    }
    fn get_settings_form() -> Result<SettingsForm> {
        T::get_settings_form()
    }
    fn get_search_capabilities(kind: Option<MediaKind>) -> Result<SearchCapabilities> {
        if kind.is_some_and(|k| k != MediaKind::Novel) {
            Err(Error::Unsupported)
        } else {
            T::get_search_capabilities()
        }
    }
    fn get_option_page(request: OptionRequest) -> Result<OptionPage> {
        T::get_option_page(request)
    }
    fn get_search_suggestions(
        query: &str,
        kind: Option<MediaKind>,
    ) -> Result<Vec<SearchSuggestion>> {
        if kind.is_some_and(|k| k != MediaKind::Novel) {
            Err(Error::Unsupported)
        } else {
            T::get_search_suggestions(query)
        }
    }

    fn get_settings_actions() -> Result<Vec<crate::models::SettingsAction>> {
        T::get_settings_actions()
    }

    fn run_settings_action(action_id: &str) -> Result<String> {
        T::run_settings_action(action_id)
    }

    fn validate_settings() -> Result<Vec<crate::models::SettingIssue>> {
        T::validate_settings()
    }

    fn get_listing(listing: Listing, page: i32) -> Result<MediaPageResult> {
        T::get_novel_list(listing, page).map(Into::into)
    }

    fn search(
        query: &str,
        page: i32,
        kind: Option<MediaKind>,
        filters: Vec<FilterItem>,
    ) -> Result<MediaPageResult> {
        if kind.is_some_and(|kind| kind != MediaKind::Novel) {
            return Ok(empty_page());
        }
        T::get_search_novel_list(query, page, filters).map(Into::into)
    }

    fn get_media_update(
        entry: MediaEntry,
        needs_details: bool,
        needs_units: bool,
    ) -> Result<MediaEntry> {
        match entry {
            MediaEntry::Novel(novel) => {
                T::get_novel_update(novel, needs_details, needs_units).map(MediaEntry::Novel)
            }
            _ => Err(Error::Unsupported),
        }
    }

    fn get_chapter_pages(entry: MediaEntry, chapter: Chapter) -> Result<Vec<Page>> {
        match entry {
            MediaEntry::Novel(novel) => T::get_chapter_content(novel, chapter),
            _ => Err(Error::Unsupported),
        }
    }

    fn mark_completed(kind: MediaKind, media_key: &str, unit_key: &str) -> Result<()> {
        match kind {
            MediaKind::Novel => T::mark_chapter_read(media_key, unit_key),
            _ => Err(Error::Unsupported),
        }
    }

    fn handle_url(url: &str) -> Result<crate::models::LinkValue> {
        T::handle_url(url)
    }

    fn get_image_origins() -> Result<Vec<String>> {
        T::get_image_origins()
    }

    fn get_image_request(url: &str) -> Result<crate::models::PluginImageRequest> {
        T::get_image_request(url)
    }
}

#[macro_export]
macro_rules! export_manga_plugin {
    ($type:ty) => {
        $crate::export_plugin!($crate::provider::MangaSource<$type>);
    };
}

#[macro_export]
macro_rules! export_anime_plugin {
    ($type:ty) => {
        $crate::export_plugin!($crate::provider::AnimeSource<$type>);
    };
}

#[macro_export]
macro_rules! export_novel_plugin {
    ($type:ty) => {
        $crate::export_plugin!($crate::provider::NovelSource<$type>);
    };
}

/// Exports a `SourceProvider`. Call it once per plugin.
#[macro_export]
macro_rules! export_plugin {
    ($type:ty) => {
        $crate::__export_abi_version!();
        $crate::__export_last_error!();
        $crate::__export_image_requests!($type, $crate::provider::SourceProvider);
        $crate::__export_settings_actions!($type, $crate::provider::SourceProvider);
        $crate::__export_controls!($type, $crate::provider::SourceProvider);

        #[unsafe(no_mangle)]
        pub extern "C" fn ito_supported_kinds() -> u32 {
            const MASK: u32 =
                $crate::models::MediaKind::mask(<$type as $crate::provider::SourceProvider>::KINDS);
            const _: () = assert!(MASK != 0, "a source must serve at least one kind");
            MASK
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_home() -> i64 {
            $crate::image::__pack_result(
                "get_home",
                <$type as $crate::provider::SourceProvider>::get_home(),
            )
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_settings() -> i64 {
            match <$type as $crate::provider::SourceProvider>::get_settings() {
                Some(settings) => $crate::image::__pack_result("get_settings", Ok(settings)),
                None => 0,
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn handle_url(url_ptr: i32, url_len: i32) -> i64 {
            let url = $crate::export::__text(unsafe { $crate::export::__bytes(url_ptr, url_len) });
            $crate::image::__pack_result(
                "handle_url",
                <$type as $crate::provider::SourceProvider>::handle_url(url.as_ref()),
            )
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn handle_image(url_ptr: i32, url_len: i32) -> i64 {
            let url = $crate::export::__text(unsafe { $crate::export::__bytes(url_ptr, url_len) });
            $crate::export::__pack_bytes(
                "handle_image",
                <$type as $crate::provider::SourceProvider>::handle_image(url.as_ref()),
            )
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_listing(listing_ptr: i32, listing_len: i32, page: i32) -> i64 {
            let listing = unsafe { $crate::export::__bytes(listing_ptr, listing_len) };
            $crate::image::__pack_result(
                "get_listing",
                $crate::export::__listing::<$type>(listing, page),
            )
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn search(
            query_ptr: i32,
            query_len: i32,
            page: i32,
            kind: i32,
            filters_ptr: i32,
            filters_len: i32,
        ) -> i64 {
            let query = unsafe { $crate::export::__bytes(query_ptr, query_len) };
            let filters = unsafe { $crate::export::__bytes(filters_ptr, filters_len) };
            $crate::image::__pack_result(
                "search",
                $crate::export::__search::<$type>(query, page, kind, filters),
            )
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_media_update(
            entry_ptr: i32,
            entry_len: i32,
            needs_details: i32,
            needs_units: i32,
        ) -> i64 {
            let entry = unsafe { $crate::export::__bytes(entry_ptr, entry_len) };
            $crate::image::__pack_result(
                "get_media_update",
                $crate::export::__media_update::<$type>(entry, needs_details, needs_units),
            )
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_chapter_pages(
            entry_ptr: i32,
            entry_len: i32,
            chapter_ptr: i32,
            chapter_len: i32,
        ) -> i64 {
            let entry = unsafe { $crate::export::__bytes(entry_ptr, entry_len) };
            let chapter = unsafe { $crate::export::__bytes(chapter_ptr, chapter_len) };
            $crate::image::__pack_result(
                "get_chapter_pages",
                $crate::export::__chapter_pages::<$type>(entry, chapter),
            )
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_episode_videos(
            anime_ptr: i32,
            anime_len: i32,
            episode_ptr: i32,
            episode_len: i32,
        ) -> i64 {
            let anime = unsafe { $crate::export::__bytes(anime_ptr, anime_len) };
            let episode = unsafe { $crate::export::__bytes(episode_ptr, episode_len) };
            $crate::image::__pack_result(
                "get_episode_videos",
                $crate::export::__episode_videos::<$type>(anime, episode),
            )
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn mark_completed(
            kind: i32,
            media_key_ptr: i32,
            media_key_len: i32,
            unit_key_ptr: i32,
            unit_key_len: i32,
        ) -> i32 {
            let media_key = unsafe { $crate::export::__bytes(media_key_ptr, media_key_len) };
            let unit_key = unsafe { $crate::export::__bytes(unit_key_ptr, unit_key_len) };
            $crate::completion::__ack_result(
                "mark_completed",
                $crate::export::__mark_completed::<$type>(kind, media_key, unit_key),
            )
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        ContentKind,
        manga::{ContentRating, Status, Viewer},
    };
    use std::cell::RefCell;

    thread_local! {
        static CALLS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn record(call: impl Into<String>) {
        CALLS.with(|calls| calls.borrow_mut().push(call.into()));
    }

    fn calls() -> Vec<String> {
        CALLS.with(|calls| calls.borrow_mut().drain(..).collect())
    }

    fn manga(key: &str) -> Manga {
        Manga {
            key: key.into(),
            title: key.into(),
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
        }
    }

    fn chapter(kind: ContentKind) -> Chapter {
        Chapter {
            key: "c1".into(),
            title: None,
            volume: None,
            chapter: None,
            date_updated: None,
            scanlator: None,
            url: None,
            lang: None,
            paywalled: None,
            kind,
        }
    }

    struct Shelf;

    impl MangaProvider for Shelf {
        fn get_manga_list(_listing: Listing, _page: i32) -> Result<manga::PageResult> {
            Ok(manga::PageResult {
                entries: vec![manga("listed")],
                has_next_page: true,
            })
        }

        fn get_search_manga_list(
            query: &str,
            _page: i32,
            _filters: Vec<FilterItem>,
        ) -> Result<manga::PageResult> {
            record(format!("search {query}"));
            Ok(manga::PageResult {
                entries: vec![manga(query)],
                has_next_page: false,
            })
        }

        fn get_manga_update(
            mut manga: Manga,
            _needs_details: bool,
            _needs_chapters: bool,
        ) -> Result<Manga> {
            manga.title = "updated".into();
            Ok(manga)
        }

        fn get_page_list(manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
            record(format!(
                "pages {} {} {:?}",
                manga.key, chapter.key, chapter.kind
            ));
            Ok(Vec::new())
        }

        fn mark_chapter_read(media_key: &str, chapter_key: &str) -> Result<()> {
            record(format!("read {media_key} {chapter_key}"));
            Ok(())
        }
    }

    type Source = MangaSource<Shelf>;

    #[test]
    fn a_single_kind_source_answers_only_for_its_own_kind() {
        assert_eq!(MediaKind::mask(Source::KINDS), 0b001);

        let novels = Source::search("q", 1, Some(MediaKind::Novel), Vec::new()).unwrap();
        assert!(novels.entries.is_empty() && !novels.has_next_page);
        assert!(
            calls().is_empty(),
            "a search for another kind must not reach the plugin"
        );

        let any = Source::search("q", 1, None, Vec::new()).unwrap();
        let manga = Source::search("m", 1, Some(MediaKind::Manga), Vec::new()).unwrap();
        assert_eq!(calls(), ["search q", "search m"]);
        assert!(matches!(&any.entries[..], [MediaEntry::Manga(found)] if found.key == "q"));
        assert!(matches!(&manga.entries[..], [MediaEntry::Manga(found)] if found.key == "m"));

        assert!(matches!(
            Source::mark_completed(MediaKind::Anime, "s", "c"),
            Err(Error::Unsupported)
        ));
        Source::mark_completed(MediaKind::Manga, "s", "c").unwrap();
        assert_eq!(calls(), ["read s c"]);
    }

    #[test]
    fn a_single_kind_source_passes_its_own_entries_and_chapters_through() {
        let listed = Source::get_listing(
            Listing {
                id: "l".into(),
                name: "L".into(),
                kind: 0,
            },
            1,
        )
        .unwrap();
        assert!(listed.has_next_page);
        assert!(matches!(&listed.entries[..], [MediaEntry::Manga(found)] if found.key == "listed"));

        let updated = Source::get_media_update(MediaEntry::Manga(manga("s")), true, true).unwrap();
        assert!(
            matches!(updated, MediaEntry::Manga(found) if found.key == "s" && found.title == "updated")
        );

        // A text chapter of a manga reaches the manga plugin with its kind intact.
        Source::get_chapter_pages(MediaEntry::Manga(manga("s")), chapter(ContentKind::Text))
            .unwrap();
        assert_eq!(calls(), ["pages s c1 Text"]);

        let novel = Novel {
            key: "n".into(),
            title: "N".into(),
            authors: None,
            artist: None,
            description: None,
            tags: None,
            cover: None,
            url: None,
            status: novel::Status::Unknown,
            content_rating: novel::ContentRating::Safe,
            nsfw: 0,
            chapters: None,
        };
        assert!(matches!(
            Source::get_media_update(MediaEntry::Novel(novel.clone()), true, true),
            Err(Error::Unsupported)
        ));
        assert!(matches!(
            Source::get_chapter_pages(MediaEntry::Novel(novel), chapter(ContentKind::Text)),
            Err(Error::Unsupported)
        ));
        assert!(calls().is_empty());
    }

    struct Shows;

    impl AnimeProvider for Shows {
        fn get_anime_list(_listing: Listing, _page: i32) -> Result<anime::PageResult> {
            Ok(anime::PageResult {
                entries: Vec::new(),
                has_next_page: false,
            })
        }

        fn get_search_anime_list(
            query: &str,
            _page: i32,
            _filters: Vec<FilterItem>,
        ) -> Result<anime::PageResult> {
            record(format!("search {query}"));
            Ok(anime::PageResult {
                entries: Vec::new(),
                has_next_page: false,
            })
        }

        fn get_anime_update(
            anime: Anime,
            _needs_details: bool,
            _needs_episodes: bool,
        ) -> Result<Anime> {
            Ok(anime)
        }

        fn get_video_list(anime: Anime, episode: Episode) -> Result<Vec<Video>> {
            record(format!("videos {} {}", anime.key, episode.key));
            Ok(Vec::new())
        }

        fn mark_episode_watched(media_key: &str, episode_key: &str) -> Result<()> {
            record(format!("watched {media_key} {episode_key}"));
            Ok(())
        }
    }

    #[test]
    fn an_anime_source_routes_episodes_and_its_own_completions() {
        type Source = AnimeSource<Shows>;
        assert_eq!(MediaKind::mask(Source::KINDS), 0b010);
        Source::search("q", 1, Some(MediaKind::Manga), Vec::new()).unwrap();
        Source::search("q", 1, Some(MediaKind::Anime), Vec::new()).unwrap();
        let anime = Anime {
            key: "a".into(),
            title: "A".into(),
            studios: None,
            description: None,
            tags: None,
            cover: None,
            url: None,
            status: anime::Status::Unknown,
            content_rating: anime::ContentRating::Safe,
            nsfw: 0,
            episodes: None,
            seasons: None,
        };
        let episode = Episode {
            key: "e".into(),
            title: None,
            episode: None,
            date_updated: None,
            url: None,
            lang: None,
            paywalled: None,
        };
        Source::get_episode_videos(anime.clone(), episode).unwrap();
        assert!(matches!(
            Source::get_chapter_pages(MediaEntry::Anime(anime), chapter(ContentKind::Images)),
            Err(Error::Unsupported)
        ));
        Source::mark_completed(MediaKind::Anime, "a", "e").unwrap();
        assert!(matches!(
            Source::mark_completed(MediaKind::Manga, "a", "e"),
            Err(Error::Unsupported)
        ));
        assert_eq!(calls(), ["search q", "videos a e", "watched a e"]);
    }

    struct Library;

    impl NovelProvider for Library {
        fn get_novel_list(_listing: Listing, _page: i32) -> Result<novel::PageResult> {
            Ok(novel::PageResult {
                entries: Vec::new(),
                has_next_page: false,
            })
        }

        fn get_search_novel_list(
            _query: &str,
            _page: i32,
            _filters: Vec<FilterItem>,
        ) -> Result<novel::PageResult> {
            Ok(novel::PageResult {
                entries: Vec::new(),
                has_next_page: false,
            })
        }

        fn get_novel_update(
            novel: Novel,
            _needs_details: bool,
            _needs_chapters: bool,
        ) -> Result<Novel> {
            Ok(novel)
        }

        fn get_chapter_content(novel: Novel, chapter: Chapter) -> Result<Vec<Page>> {
            record(format!(
                "content {} {} {:?}",
                novel.key, chapter.key, chapter.kind
            ));
            Ok(Vec::new())
        }

        fn mark_chapter_read(media_key: &str, chapter_key: &str) -> Result<()> {
            record(format!("read {media_key} {chapter_key}"));
            Ok(())
        }
    }

    #[test]
    fn a_novel_source_reads_its_chapters_and_completions() {
        type Source = NovelSource<Library>;
        assert_eq!(MediaKind::mask(Source::KINDS), 0b100);
        let novel = Novel {
            key: "n".into(),
            title: "N".into(),
            authors: None,
            artist: None,
            description: None,
            tags: None,
            cover: None,
            url: None,
            status: novel::Status::Unknown,
            content_rating: novel::ContentRating::Safe,
            nsfw: 0,
            chapters: None,
        };
        Source::get_chapter_pages(MediaEntry::Novel(novel), chapter(ContentKind::Text)).unwrap();
        assert!(matches!(
            Source::get_chapter_pages(MediaEntry::Manga(manga("s")), chapter(ContentKind::Text)),
            Err(Error::Unsupported)
        ));
        Source::mark_completed(MediaKind::Novel, "n", "c1").unwrap();
        assert!(matches!(
            Source::mark_completed(MediaKind::Manga, "n", "c1"),
            Err(Error::Unsupported)
        ));
        assert_eq!(calls(), ["content n c1 Text", "read n c1"]);
    }
}
