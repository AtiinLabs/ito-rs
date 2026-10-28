use std::sync::atomic::{AtomicUsize, Ordering};

use ito_rs::{
    Error, Result,
    controls::{OptionPage, OptionRequest, SearchCapabilities, SearchSuggestion, SettingsForm},
    models::{
        Chapter, FilterItem, Listing, MediaKind, Page,
        anime::{self, Anime, Episode, Video},
        manga::{self, Manga},
        novel::{self, Novel},
    },
    provider::{
        AnimeProvider, AnimeSource, MangaProvider, MangaSource, NovelProvider, NovelSource,
        SourceProvider,
    },
};

static MANGA_CALLS: AtomicUsize = AtomicUsize::new(0);
static ANIME_CALLS: AtomicUsize = AtomicUsize::new(0);
static NOVEL_CALLS: AtomicUsize = AtomicUsize::new(0);

struct MangaPlugin;
impl MangaProvider for MangaPlugin {
    fn get_manga_list(_: Listing, _: i32) -> Result<manga::PageResult> {
        Ok(manga::PageResult {
            entries: Vec::new(),
            has_next_page: false,
        })
    }
    fn get_search_manga_list(_: &str, _: i32, _: Vec<FilterItem>) -> Result<manga::PageResult> {
        Ok(manga::PageResult {
            entries: Vec::new(),
            has_next_page: false,
        })
    }
    fn get_manga_update(value: Manga, _: bool, _: bool) -> Result<Manga> {
        Ok(value)
    }
    fn get_page_list(_: Manga, _: Chapter) -> Result<Vec<Page>> {
        Ok(Vec::new())
    }
    fn get_settings_form() -> Result<SettingsForm> {
        MANGA_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(SettingsForm::new(Vec::new()))
    }
    fn get_search_capabilities() -> Result<SearchCapabilities> {
        MANGA_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(SearchCapabilities::new())
    }
    fn get_option_page(_: OptionRequest) -> Result<OptionPage> {
        MANGA_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(OptionPage {
            options: Vec::new(),
            next_offset: None,
            total: None,
            revision: None,
        })
    }
    fn get_search_suggestions(_: &str) -> Result<Vec<SearchSuggestion>> {
        MANGA_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }
}

struct AnimePlugin;
impl AnimeProvider for AnimePlugin {
    fn get_anime_list(_: Listing, _: i32) -> Result<anime::PageResult> {
        Ok(anime::PageResult {
            entries: Vec::new(),
            has_next_page: false,
        })
    }
    fn get_search_anime_list(_: &str, _: i32, _: Vec<FilterItem>) -> Result<anime::PageResult> {
        Ok(anime::PageResult {
            entries: Vec::new(),
            has_next_page: false,
        })
    }
    fn get_anime_update(value: Anime, _: bool, _: bool) -> Result<Anime> {
        Ok(value)
    }
    fn get_video_list(_: Anime, _: Episode) -> Result<Vec<Video>> {
        Ok(Vec::new())
    }
    fn get_settings_form() -> Result<SettingsForm> {
        ANIME_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(SettingsForm::new(Vec::new()))
    }
    fn get_search_capabilities() -> Result<SearchCapabilities> {
        ANIME_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(SearchCapabilities::new())
    }
    fn get_option_page(_: OptionRequest) -> Result<OptionPage> {
        ANIME_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(OptionPage {
            options: Vec::new(),
            next_offset: None,
            total: None,
            revision: None,
        })
    }
    fn get_search_suggestions(_: &str) -> Result<Vec<SearchSuggestion>> {
        ANIME_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }
}

struct NovelPlugin;
impl NovelProvider for NovelPlugin {
    fn get_novel_list(_: Listing, _: i32) -> Result<novel::PageResult> {
        Ok(novel::PageResult {
            entries: Vec::new(),
            has_next_page: false,
        })
    }
    fn get_search_novel_list(_: &str, _: i32, _: Vec<FilterItem>) -> Result<novel::PageResult> {
        Ok(novel::PageResult {
            entries: Vec::new(),
            has_next_page: false,
        })
    }
    fn get_novel_update(value: Novel, _: bool, _: bool) -> Result<Novel> {
        Ok(value)
    }
    fn get_chapter_content(_: Novel, _: Chapter) -> Result<Vec<Page>> {
        Ok(Vec::new())
    }
    fn get_settings_form() -> Result<SettingsForm> {
        NOVEL_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(SettingsForm::new(Vec::new()))
    }
    fn get_search_capabilities() -> Result<SearchCapabilities> {
        NOVEL_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(SearchCapabilities::new())
    }
    fn get_option_page(_: OptionRequest) -> Result<OptionPage> {
        NOVEL_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(OptionPage {
            options: Vec::new(),
            next_offset: None,
            total: None,
            revision: None,
        })
    }
    fn get_search_suggestions(_: &str) -> Result<Vec<SearchSuggestion>> {
        NOVEL_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }
}

#[test]
fn single_kind_adapters_forward_matching_kinds_and_reject_other_kinds_first() {
    MANGA_CALLS.store(0, Ordering::SeqCst);
    ANIME_CALLS.store(0, Ordering::SeqCst);
    NOVEL_CALLS.store(0, Ordering::SeqCst);
    let request = OptionRequest {
        catalog: "catalog".into(),
        query: ito_rs::controls::OptionQuery::Search {
            text: String::new(),
            offset: 0,
        },
        limit: 1,
    };

    type MangaAdapter = MangaSource<MangaPlugin>;
    assert!(matches!(
        <MangaAdapter as SourceProvider>::get_search_capabilities(Some(MediaKind::Anime)),
        Err(Error::Unsupported)
    ));
    assert!(matches!(
        <MangaAdapter as SourceProvider>::get_search_suggestions("q", Some(MediaKind::Novel)),
        Err(Error::Unsupported)
    ));
    assert_eq!(
        MANGA_CALLS.load(Ordering::SeqCst),
        0,
        "wrong-kind calls must not reach manga provider"
    );
    assert!(<MangaAdapter as SourceProvider>::get_settings_form().is_ok());
    assert!(
        <MangaAdapter as SourceProvider>::get_search_capabilities(Some(MediaKind::Manga)).is_ok()
    );
    assert!(
        <MangaAdapter as SourceProvider>::get_search_suggestions("q", Some(MediaKind::Manga))
            .is_ok()
    );
    assert!(<MangaAdapter as SourceProvider>::get_search_capabilities(None).is_ok());
    assert!(<MangaAdapter as SourceProvider>::get_option_page(request.clone()).is_ok());
    assert_eq!(MANGA_CALLS.load(Ordering::SeqCst), 5);

    type AnimeAdapter = AnimeSource<AnimePlugin>;
    assert!(matches!(
        <AnimeAdapter as SourceProvider>::get_search_capabilities(Some(MediaKind::Manga)),
        Err(Error::Unsupported)
    ));
    assert!(matches!(
        <AnimeAdapter as SourceProvider>::get_search_suggestions("q", Some(MediaKind::Novel)),
        Err(Error::Unsupported)
    ));
    assert_eq!(
        ANIME_CALLS.load(Ordering::SeqCst),
        0,
        "wrong-kind calls must not reach anime provider"
    );
    assert!(<AnimeAdapter as SourceProvider>::get_settings_form().is_ok());
    assert!(
        <AnimeAdapter as SourceProvider>::get_search_capabilities(Some(MediaKind::Anime)).is_ok()
    );
    assert!(
        <AnimeAdapter as SourceProvider>::get_search_suggestions("q", Some(MediaKind::Anime))
            .is_ok()
    );
    assert!(<AnimeAdapter as SourceProvider>::get_search_capabilities(None).is_ok());
    assert!(<AnimeAdapter as SourceProvider>::get_option_page(request.clone()).is_ok());
    assert_eq!(ANIME_CALLS.load(Ordering::SeqCst), 5);

    type NovelAdapter = NovelSource<NovelPlugin>;
    assert!(matches!(
        <NovelAdapter as SourceProvider>::get_search_capabilities(Some(MediaKind::Anime)),
        Err(Error::Unsupported)
    ));
    assert!(matches!(
        <NovelAdapter as SourceProvider>::get_search_suggestions("q", Some(MediaKind::Manga)),
        Err(Error::Unsupported)
    ));
    assert_eq!(
        NOVEL_CALLS.load(Ordering::SeqCst),
        0,
        "wrong-kind calls must not reach novel provider"
    );
    assert!(<NovelAdapter as SourceProvider>::get_settings_form().is_ok());
    assert!(
        <NovelAdapter as SourceProvider>::get_search_capabilities(Some(MediaKind::Novel)).is_ok()
    );
    assert!(
        <NovelAdapter as SourceProvider>::get_search_suggestions("q", Some(MediaKind::Novel))
            .is_ok()
    );
    assert!(<NovelAdapter as SourceProvider>::get_search_capabilities(None).is_ok());
    assert!(<NovelAdapter as SourceProvider>::get_option_page(request).is_ok());
    assert_eq!(NOVEL_CALLS.load(Ordering::SeqCst), 5);
}
