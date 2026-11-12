//! Pins the `get_chapter_editions` wire layouts, bounds and dispatch to
//! `fixtures/chapter-editions-v1.json`. The fixture bytes are written independently of the SDK's
//! encoder; entries inside requests reuse the bytes of `fixtures/catalog-v3.json`.

use ito_rs::{
    Error, Result,
    catalog,
    editions::{self, MAX_CURSOR_BYTES, MAX_LIMIT, Violation},
    error::ErrorCategory,
    export,
    models::{
        CatalogCompleteness as Completeness, CatalogScope as Scope, Chapter, ChapterEditionsPage,
        ChapterEditionsRequest, ContentKind, FilterItem, Listing, MediaEntry, MediaKind,
        MediaPageResult, Page, Scanlator,
        anime::{self, Anime, Episode, Video},
        manga::{self, Manga},
        novel::{self, Novel},
    },
    provider::{
        AnimeProvider, AnimeSource, MangaProvider, MangaSource, NovelProvider, NovelSource,
        SourceProvider,
    },
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{cell::RefCell, fmt::Debug};

const EXPORT: &str = "get_chapter_editions";

fn fixture() -> Value {
    serde_json::from_str(include_str!("../fixtures/chapter-editions-v1.json"))
        .expect("valid editions fixture JSON")
}

fn catalog_fixture() -> Value {
    serde_json::from_str(include_str!("../fixtures/catalog-v3.json"))
        .expect("valid catalog fixture JSON")
}

fn wire_bytes(case: &Value) -> Vec<u8> {
    let part = |key: &str| hex::decode(case.get(key).and_then(Value::as_str).unwrap_or("")).unwrap();
    let mut bytes = part("hex");
    if let Some(count) = case.get("repeat_count") {
        let unit = part("repeat_hex");
        for _ in 0..count.as_u64().unwrap() {
            bytes.extend_from_slice(&unit);
        }
        bytes.extend_from_slice(&part("suffix_hex"));
    }
    bytes
}

/// The entry bytes `fixtures/catalog-v3.json` pins under `name`, decoded.
fn catalog_entry(name: &str) -> MediaEntry {
    let fixture = catalog_fixture();
    let case = fixture["wire"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == name)
        .unwrap_or_else(|| panic!("catalog fixture {name}"));
    ito_rs::postcard::from_bytes(&wire_bytes(case)).expect("catalog fixture entry decodes")
}

fn strict<T: DeserializeOwned>(bytes: &[u8]) -> Option<T> {
    let (value, rest) = ito_rs::postcard::take_from_bytes(bytes).ok()?;
    rest.is_empty().then_some(value)
}

fn outcome(result: std::result::Result<(), Violation>) -> String {
    match result {
        Ok(()) => "ok".into(),
        Err(violation) => format!("violation:{}", violation.code()),
    }
}

fn chapter_a() -> Chapter {
    Chapter {
        title: Some("Prologue".into()),
        chapter: Some(1.0),
        scanlator: Some(Scanlator::with_id("g-1", "Alpha Scans")),
        lang: Some("en".into()),
        reading_order: Some(0),
        ..Chapter::new("t1", ContentKind::Text)
    }
}

fn chapter_b() -> Chapter {
    Chapter {
        title: Some("Gallery".into()),
        chapter: Some(2.0),
        scanlator: Some(Scanlator::with_id("g-2", "Beta Scans")),
        lang: Some("en".into()),
        reading_order: Some(1),
        revision: Some("r1".into()),
        ..Chapter::new("i1", ContentKind::Images)
    }
}

fn grouped(key: &str, name: &str) -> Chapter {
    Chapter {
        scanlator: Some(Scanlator::with_id("g-1", name)),
        ..Chapter::new(key, ContentKind::Text)
    }
}

fn page_of(
    editions: Vec<Chapter>,
    next_cursor: Option<&str>,
    scope: Scope,
    completeness: Completeness,
) -> ChapterEditionsPage {
    ChapterEditionsPage {
        editions,
        next_cursor: next_cursor.map(str::to_owned),
        scope,
        completeness,
    }
}

fn built_request(name: &str) -> ChapterEditionsRequest {
    let (entry, cursor, limit): (_, Option<String>, u32) = match name {
        "request.manga_first_page" => ("entry.manga.minimal", None, 100),
        "request.manga_with_cursor" => ("entry.manga.minimal", Some("cur-1".into()), 20),
        "request.novel_with_cursor" => ("entry.novel.minimal", Some("cur-1".into()), 1),
        "request.manga_with_chapters_and_catalog" => {
            ("entry.manga.catalog_visible_partial", Some("cur-1".into()), 10)
        }
        "request.anime_entry" => ("entry.anime.with_episode", None, 10),
        "request.limit_zero" => ("entry.manga.minimal", None, 0),
        "request.limit_101" => ("entry.manga.minimal", None, 101),
        "request.limit_max_u32" => ("entry.manga.minimal", None, u32::MAX),
        "request.cursor_empty" => ("entry.manga.minimal", Some(String::new()), 10),
        "request.cursor_at_limit" => ("entry.manga.minimal", Some("a".repeat(4096)), 10),
        "request.cursor_oversized" => ("entry.manga.minimal", Some("a".repeat(4097)), 10),
        "request.cursor_multibyte_oversized" => {
            ("entry.manga.minimal", Some("\u{20ac}".repeat(1366)), 10)
        }
        "request.cursor_multibyte_at_limit" => {
            ("entry.manga.minimal", Some("\u{20ac}".repeat(1365)), 10)
        }
        other => panic!("unhandled request fixture {other}"),
    };
    ChapterEditionsRequest {
        entry: catalog_entry(entry),
        cursor,
        limit,
    }
}

fn built_page(name: &str) -> ChapterEditionsPage {
    use Completeness::{Complete, Partial};
    use Scope::{AllEligible, Visible};
    let unknown = (Scope::Unknown, Completeness::Unknown);
    match name {
        "page.mixed_nonterminal" => {
            page_of(vec![chapter_a(), chapter_b()], Some("cur-2"), Visible, Partial)
        }
        "page.terminal_complete" => {
            page_of(vec![chapter_a(), chapter_b()], None, AllEligible, Complete)
        }
        "page.terminal_visible_complete" => page_of(vec![chapter_a()], None, Visible, Complete),
        "page.terminal_partial" => page_of(vec![chapter_a()], None, Visible, Partial),
        "page.terminal_unknown" => page_of(vec![chapter_a()], None, unknown.0, unknown.1),
        "page.terminal_empty" => page_of(Vec::new(), None, unknown.0, unknown.1),
        "page.empty_with_cursor" => page_of(Vec::new(), Some("c"), Visible, Partial),
        "page.cursor_complete" => page_of(vec![chapter_a()], Some("c"), Visible, Complete),
        "page.cursor_unknown_completeness" => {
            page_of(vec![chapter_a()], Some("c"), Visible, Completeness::Unknown)
        }
        "page.cursor_empty" => page_of(vec![chapter_a()], Some(""), Visible, Partial),
        "page.cursor_at_limit" => {
            page_of(vec![chapter_a()], Some(&"a".repeat(4096)), Visible, Partial)
        }
        "page.cursor_oversized" => {
            page_of(vec![chapter_a()], Some(&"a".repeat(4097)), Visible, Partial)
        }
        "page.over_limit" => page_of(vec![chapter_a(), chapter_b()], Some("c"), Visible, Partial),
        "page.over_limit_terminal" => {
            page_of(vec![chapter_a(), chapter_b()], None, Visible, Complete)
        }
        "page.duplicate_keys" => page_of(
            vec![chapter_a(), Chapter::new("t1", ContentKind::Text)],
            None,
            Visible,
            Partial,
        ),
        "page.scanlator_conflict" => page_of(
            vec![grouped("c1", "Alpha Scans"), grouped("c2", "Alpha Renamed")],
            None,
            Visible,
            Partial,
        ),
        "page.key_empty" => page_of(
            vec![Chapter::new("", ContentKind::Text)],
            None,
            Visible,
            Partial,
        ),
        "page.count_before_completeness" => {
            page_of(vec![chapter_a(), chapter_b()], Some("c"), Visible, Complete)
        }
        "page.unknown_enum_values_decode_to_unknown" => {
            page_of(vec![chapter_a()], None, unknown.0, unknown.1)
        }
        other => panic!("unhandled page fixture {other}"),
    }
}

/// Checks the built value against the fixture bytes. A case with canonical bytes is accepted as
/// written but decodes to the built value, which encodes to the canonical bytes.
fn check<T: Serialize + DeserializeOwned + Debug>(
    expected: &T,
    bytes: &[u8],
    canonical: Option<&[u8]>,
    name: &str,
) {
    let canonical_bytes = canonical.unwrap_or(bytes);
    assert_eq!(
        ito_rs::postcard::to_allocvec(expected).unwrap(),
        canonical_bytes,
        "exact fixture encoding {name}"
    );
    let decoded: T = strict(bytes).unwrap_or_else(|| panic!("strict decode {name}"));
    assert_eq!(
        format!("{decoded:?}"),
        format!("{expected:?}"),
        "decoded fixture {name}"
    );
    assert_eq!(
        ito_rs::postcard::to_allocvec(&decoded).unwrap(),
        canonical_bytes,
        "re-encoded fixture {name}"
    );
}

/// What the recorder provider saw, in call order.
#[derive(Debug, PartialEq, Eq)]
struct Seen {
    kind: MediaKind,
    key: String,
    had_chapters: bool,
    had_catalog: bool,
    cursor: Option<String>,
    limit: u32,
}

thread_local! {
    static SEEN: RefCell<Vec<Seen>> = const { RefCell::new(Vec::new()) };
    static REPLY: RefCell<Option<Result<ChapterEditionsPage>>> = const { RefCell::new(None) };
}

fn record(entry: &MediaEntry, cursor: Option<String>, limit: u32) {
    let (had_chapters, had_catalog) = match entry {
        MediaEntry::Manga(manga) => (manga.chapters.is_some(), manga.catalog.is_some()),
        MediaEntry::Novel(novel) => (novel.chapters.is_some(), novel.catalog.is_some()),
        MediaEntry::Anime(_) => (false, false),
    };
    SEEN.with_borrow_mut(|seen| {
        seen.push(Seen {
            kind: entry.kind(),
            key: entry.key().to_owned(),
            had_chapters,
            had_catalog,
            cursor,
            limit,
        });
    });
}

fn reply() -> Result<ChapterEditionsPage> {
    REPLY
        .with_borrow_mut(Option::take)
        .expect("the test queued a reply")
}

fn empty_page() -> MediaPageResult {
    MediaPageResult {
        entries: Vec::new(),
        has_next_page: false,
    }
}

/// Implements the export directly, over every kind.
struct Recorder;

impl SourceProvider for Recorder {
    const KINDS: &'static [MediaKind] = &[MediaKind::Manga, MediaKind::Novel];

    fn get_listing(_: Listing, _: i32) -> Result<MediaPageResult> {
        Ok(empty_page())
    }

    fn search(
        _: &str,
        _: i32,
        _: Option<MediaKind>,
        _: Vec<FilterItem>,
    ) -> Result<MediaPageResult> {
        Ok(empty_page())
    }

    fn get_media_update(entry: MediaEntry, _: bool, _: bool) -> Result<MediaEntry> {
        Ok(entry)
    }

    fn get_chapter_editions(request: ChapterEditionsRequest) -> Result<ChapterEditionsPage> {
        record(&request.entry, request.cursor, request.limit);
        reply()
    }
}

/// Leaves the export unimplemented.
struct Silent;

impl SourceProvider for Silent {
    const KINDS: &'static [MediaKind] = &[MediaKind::Manga];

    fn get_listing(_: Listing, _: i32) -> Result<MediaPageResult> {
        Ok(empty_page())
    }

    fn search(
        _: &str,
        _: i32,
        _: Option<MediaKind>,
        _: Vec<FilterItem>,
    ) -> Result<MediaPageResult> {
        Ok(empty_page())
    }

    fn get_media_update(entry: MediaEntry, _: bool, _: bool) -> Result<MediaEntry> {
        Ok(entry)
    }
}

struct Manual;

impl MangaProvider for Manual {
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
    fn get_manga_update(manga: Manga, _: bool, _: bool) -> Result<Manga> {
        Ok(manga)
    }
    fn get_page_list(_: Manga, _: Chapter) -> Result<Vec<Page>> {
        Ok(Vec::new())
    }
    fn get_chapter_editions(
        manga: Manga,
        cursor: Option<String>,
        limit: u32,
    ) -> Result<ChapterEditionsPage> {
        record(&MediaEntry::Manga(manga), cursor, limit);
        reply()
    }
}

/// A single-kind provider that does not implement the export.
struct PlainManga;

impl MangaProvider for PlainManga {
    fn get_manga_list(l: Listing, p: i32) -> Result<manga::PageResult> {
        Manual::get_manga_list(l, p)
    }
    fn get_search_manga_list(q: &str, p: i32, f: Vec<FilterItem>) -> Result<manga::PageResult> {
        Manual::get_search_manga_list(q, p, f)
    }
    fn get_manga_update(manga: Manga, d: bool, c: bool) -> Result<Manga> {
        Manual::get_manga_update(manga, d, c)
    }
    fn get_page_list(m: Manga, c: Chapter) -> Result<Vec<Page>> {
        Manual::get_page_list(m, c)
    }
}

struct ManualNovel;

impl NovelProvider for ManualNovel {
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
    fn get_novel_update(novel: Novel, _: bool, _: bool) -> Result<Novel> {
        Ok(novel)
    }
    fn get_chapter_content(_: Novel, _: Chapter) -> Result<Vec<Page>> {
        Ok(Vec::new())
    }
    fn get_chapter_editions(
        novel: Novel,
        cursor: Option<String>,
        limit: u32,
    ) -> Result<ChapterEditionsPage> {
        record(&MediaEntry::Novel(novel), cursor, limit);
        reply()
    }
}

struct PlainAnime;

impl AnimeProvider for PlainAnime {
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
    fn get_anime_update(anime: Anime, _: bool, _: bool) -> Result<Anime> {
        Ok(anime)
    }
    fn get_video_list(_: Anime, _: Episode) -> Result<Vec<Video>> {
        Ok(Vec::new())
    }
}

fn encode(request: &ChapterEditionsRequest) -> Vec<u8> {
    ito_rs::postcard::to_allocvec(request).expect("a request encodes")
}

/// Runs the export's dispatch on an encoded request, with `reply` queued for the provider.
fn dispatch<T: SourceProvider>(
    bytes: &[u8],
    queued: Option<Result<ChapterEditionsPage>>,
) -> Result<Vec<u8>> {
    SEEN.with_borrow_mut(Vec::clear);
    REPLY.with_borrow_mut(|slot| *slot = queued);
    export::__chapter_editions::<T>(bytes)
}

fn calls() -> usize {
    SEEN.with_borrow(Vec::len)
}

fn request_of(entry: &str, cursor: Option<&str>, limit: u32) -> ChapterEditionsRequest {
    ChapterEditionsRequest {
        entry: catalog_entry(entry),
        cursor: cursor.map(str::to_owned),
        limit,
    }
}

fn assert_invalid_request(result: Result<Vec<u8>>, context: &str) {
    let error = result.expect_err(context);
    assert!(
        matches!(&error, Error::Host(message) if message == "request.invalid"),
        "{context}: {error}"
    );
    assert_eq!(error.category(), ErrorCategory::Other, "{context}");
    assert_eq!(calls(), 0, "{context}: the source must not run");
}

#[test]
fn every_request_fixture_has_exact_structure_and_validation() {
    let fixture = fixture();
    let cases = fixture["requests"].as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let name = case["name"].as_str().unwrap();
        assert_eq!(case["type"], "ChapterEditionsRequest", "{name}");
        let bytes = wire_bytes(case);
        let expected = case["expect"].as_str().unwrap();
        if expected == "decode_error" {
            assert!(
                strict::<ChapterEditionsRequest>(&bytes).is_none(),
                "the SDK must reject {name}"
            );
            assert_invalid_request(dispatch::<Recorder>(&bytes, None), name);
            continue;
        }
        let request = built_request(name);
        let canonical = case
            .get("canonical_hex")
            .map(|hex| hex::decode(hex.as_str().unwrap()).unwrap());
        check(&request, &bytes, canonical.as_deref(), name);
        assert_eq!(outcome(editions::validate_request(&request)), expected, "{name}");
        if expected != "ok" {
            assert_invalid_request(dispatch::<Recorder>(&bytes, None), name);
        }
    }
}

#[test]
fn every_page_fixture_has_exact_structure_and_validation() {
    let fixture = fixture();
    let cases = fixture["pages"].as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let name = case["name"].as_str().unwrap();
        assert_eq!(case["type"], "ChapterEditionsPage", "{name}");
        let limit = u32::try_from(case["limit"].as_u64().unwrap()).unwrap();
        let bytes = wire_bytes(case);
        let canonical = case
            .get("canonical_hex")
            .map(|hex| hex::decode(hex.as_str().unwrap()).unwrap());
        let page = built_page(name);
        check(&page, &bytes, canonical.as_deref(), name);
        let expected = case["expect"].as_str().unwrap();
        assert_eq!(outcome(editions::validate_page(&page, limit)), expected, "{name}");

        let packed = editions::__encode_page(EXPORT, &page, limit);
        match expected.strip_prefix("violation:") {
            None => assert_eq!(
                packed.expect(name),
                canonical.as_deref().unwrap_or(&bytes),
                "{name}"
            ),
            Some(code) => {
                let error = packed.expect_err(name);
                assert_eq!(error.category(), ErrorCategory::ContractViolation, "{name}");
                assert_eq!(error.message(), code, "{name}");
                assert!(
                    matches!(&error, Error::Contract { export, .. } if export == EXPORT),
                    "{name}"
                );
            }
        }
    }
}

#[test]
fn the_mixed_text_and_image_page_keeps_each_chapters_reading_mode() {
    let fixture = fixture();
    let case = fixture["pages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "page.mixed_nonterminal")
        .unwrap();
    let page: ChapterEditionsPage = strict(&wire_bytes(case)).expect("golden page decodes");
    let kinds: Vec<(&str, ContentKind)> = page
        .editions
        .iter()
        .map(|chapter| (chapter.key.as_str(), chapter.kind))
        .collect();
    assert_eq!(
        kinds,
        [("t1", ContentKind::Text), ("i1", ContentKind::Images)]
    );
    assert_eq!(page.next_cursor.as_deref(), Some("cur-2"));
    assert_eq!(page.scope, Scope::Visible);
    assert_eq!(page.completeness, Completeness::Partial);
    let groups: Vec<_> = page
        .editions
        .iter()
        .map(|chapter| chapter.scanlator.as_ref().expect("a group"))
        .map(|group| (group.id.as_deref(), group.name.as_str()))
        .collect();
    assert_eq!(
        groups,
        [(Some("g-1"), "Alpha Scans"), (Some("g-2"), "Beta Scans")]
    );
}

#[test]
fn the_fixture_limits_codes_and_run_caps_equal_the_sdk() {
    let fixture = fixture();
    let limits = &fixture["limits"];
    assert_eq!(limits["max_limit"].as_u64().unwrap(), u64::from(MAX_LIMIT));
    assert_eq!(
        limits["max_cursor_bytes"].as_u64().unwrap() as usize,
        MAX_CURSOR_BYTES
    );
    assert_eq!(
        limits["max_response_bytes"].as_u64().unwrap() as usize,
        catalog::MAX_RESPONSE_BYTES
    );
    let run = &fixture["run_limits"];
    assert_eq!(
        run["max_rows"].as_u64().unwrap() as usize,
        catalog::MAX_CHAPTERS
    );
    assert_eq!(
        run["max_cumulative_bytes"].as_u64().unwrap() as usize,
        catalog::MAX_RESPONSE_BYTES
    );
    assert_eq!(run["max_pages"], 256);
    assert_eq!(fixture["contract"], "ito-chapter-editions");
    assert_eq!(fixture["version"], 1);

    let mut listed: Vec<&str> = fixture["codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|code| code.as_str().unwrap())
        .collect();
    let mut known = vec![
        Violation::RequestEntryKind.code(),
        Violation::RequestLimit.code(),
        Violation::RequestCursor.code(),
        Violation::PageCursor.code(),
        Violation::PageCount.code(),
        Violation::PageCompleteness.code(),
        Violation::PageNoProgress.code(),
    ];
    listed.sort_unstable();
    known.sort_unstable();
    assert_eq!(listed, known);

    let pinned: Vec<&str> = fixture["requests"]
        .as_array()
        .unwrap()
        .iter()
        .chain(fixture["pages"].as_array().unwrap())
        .filter_map(|case| case["expect"].as_str()?.strip_prefix("violation:"))
        .collect();
    for code in known {
        assert!(pinned.contains(&code), "no fixture case for {code}");
    }
}

#[test]
fn the_source_sees_the_exact_request_without_chapters_or_catalog() {
    let page = built_page("page.terminal_complete");
    let expected_bytes = ito_rs::postcard::to_allocvec(&page).unwrap();
    for (entry, kind, key) in [
        ("entry.manga.catalog_visible_partial", MediaKind::Manga, "m"),
        ("entry.novel.catalog_all_eligible_complete", MediaKind::Novel, "n"),
    ] {
        let request = request_of(entry, Some("cur-1"), 7);
        let (MediaEntry::Manga(Manga { chapters: Some(_), .. })
        | MediaEntry::Novel(Novel { chapters: Some(_), .. })) = request.entry
        else {
            panic!("{entry} carries chapters on the wire");
        };
        let bytes = dispatch::<Recorder>(&encode(&request), Some(Ok(page.clone())))
            .unwrap_or_else(|error| panic!("{entry}: {error}"));
        assert_eq!(bytes, expected_bytes, "{entry}");
        assert_eq!(
            SEEN.with_borrow(|seen| seen
                .iter()
                .map(|s| (s.kind, s.key.clone(), s.had_chapters, s.had_catalog))
                .collect::<Vec<_>>()),
            [(kind, key.to_owned(), false, false)],
            "{entry}"
        );
        assert_eq!(
            SEEN.with_borrow(|seen| (seen[0].cursor.clone(), seen[0].limit)),
            (Some("cur-1".to_owned()), 7),
            "{entry}"
        );
    }
}

#[test]
fn boundary_requests_reach_the_source_unchanged() {
    for (cursor, limit) in [
        (None, 1),
        (None, MAX_LIMIT),
        (Some("a".repeat(MAX_CURSOR_BYTES)), 50),
        (Some("\u{1F600}".repeat(MAX_CURSOR_BYTES / 4)), 50),
    ] {
        let request = request_of("entry.manga.minimal", cursor.as_deref(), limit);
        dispatch::<Recorder>(
            &encode(&request),
            Some(Ok(built_page("page.terminal_empty"))),
        )
        .expect("a boundary request is valid");
        assert_eq!(
            SEEN.with_borrow(|seen| (seen[0].cursor.clone(), seen[0].limit)),
            (cursor, limit)
        );
    }
}

#[test]
fn requests_that_break_the_contract_never_reach_the_source() {
    let queued = || Some(Ok(built_page("page.terminal_empty")));
    for (name, request) in [
        ("anime", request_of("entry.anime.with_episode", None, 10)),
        ("limit zero", request_of("entry.manga.minimal", None, 0)),
        ("limit too large", request_of("entry.manga.minimal", None, MAX_LIMIT + 1)),
        ("empty cursor", request_of("entry.manga.minimal", Some(""), 10)),
        (
            "oversized cursor",
            request_of(
                "entry.manga.minimal",
                Some(&"a".repeat(MAX_CURSOR_BYTES + 1)),
                10,
            ),
        ),
    ] {
        assert_invalid_request(dispatch::<Recorder>(&encode(&request), queued()), name);
    }
    let mut trailing = encode(&request_of("entry.manga.minimal", None, 10));
    trailing.push(0);
    assert_invalid_request(dispatch::<Recorder>(&trailing, queued()), "trailing byte");
    assert_invalid_request(dispatch::<Recorder>(&[], queued()), "empty buffer");
}

#[test]
fn unsupported_means_absent_and_only_unsupported_does() {
    let manga = encode(&request_of("entry.manga.minimal", None, 10));
    let novel = encode(&request_of("entry.novel.minimal", None, 10));
    let is_unsupported = |result: Result<Vec<u8>>, context: &str| {
        let error = result.expect_err(context);
        assert!(matches!(error, Error::Unsupported), "{context}: {error}");
        assert_eq!(error.category(), ErrorCategory::Unsupported, "{context}");
        assert_eq!(calls(), 0, "{context}");
    };
    is_unsupported(dispatch::<Silent>(&manga, None), "default provider");
    is_unsupported(
        dispatch::<MangaSource<PlainManga>>(&manga, None),
        "manga provider default",
    );
    is_unsupported(
        dispatch::<AnimeSource<PlainAnime>>(&manga, None),
        "anime provider default",
    );
    is_unsupported(
        dispatch::<MangaSource<Manual>>(&novel, None),
        "manga source asked for a novel",
    );
    is_unsupported(
        dispatch::<NovelSource<ManualNovel>>(&manga, None),
        "novel source asked for a manga",
    );

    // A source that opts out itself is reported as unsupported too, not as an empty page.
    let error = dispatch::<Recorder>(&manga, Some(Err(Error::Unsupported)))
        .expect_err("an explicit opt-out");
    assert!(matches!(error, Error::Unsupported));
    assert_eq!(calls(), 1);
}

#[test]
fn single_kind_providers_receive_their_typed_entry_cursor_and_limit() {
    let page = built_page("page.mixed_nonterminal");
    let manga = encode(&request_of("entry.manga.catalog_visible_partial", Some("c-9"), 33));
    dispatch::<MangaSource<Manual>>(&manga, Some(Ok(page.clone()))).expect("manga provider");
    assert_eq!(
        SEEN.with_borrow(|seen| seen.iter().map(|s| (s.kind, s.had_chapters, s.had_catalog)).collect::<Vec<_>>()),
        [(MediaKind::Manga, false, false)]
    );
    assert_eq!(
        SEEN.with_borrow(|seen| (seen[0].cursor.clone(), seen[0].limit)),
        (Some("c-9".to_owned()), 33)
    );

    let novel = encode(&request_of("entry.novel.catalog_all_eligible_complete", None, 5));
    dispatch::<NovelSource<ManualNovel>>(&novel, Some(Ok(page))).expect("novel provider");
    assert_eq!(
        SEEN.with_borrow(|seen| seen.iter().map(|s| (s.kind, s.had_chapters, s.had_catalog)).collect::<Vec<_>>()),
        [(MediaKind::Novel, false, false)]
    );
    assert_eq!(SEEN.with_borrow(|seen| seen[0].cursor.clone()), None);
}

#[test]
fn a_source_failure_is_never_turned_into_unsupported_or_an_empty_page() {
    let bytes = encode(&request_of("entry.manga.minimal", None, 10));
    let failures = [
        (Error::Net("offline".into()), ErrorCategory::Network),
        (Error::Server("busy".into()), ErrorCategory::Server),
        (Error::NotFound("gone".into()), ErrorCategory::NotFound),
        (Error::Unauthorized("denied".into()), ErrorCategory::Unauthorized),
        (Error::NeedsSetup("setup".into()), ErrorCategory::NeedsSetup),
        (Error::NeedsLogin("login".into()), ErrorCategory::NeedsLogin),
        // A refusal that merely says "Unsupported operation" is not the typed opt-out.
        (Error::Host("Unsupported operation".into()), ErrorCategory::Other),
        (
            Error::Postcard(ito_rs::postcard::Error::DeserializeUnexpectedEnd),
            ErrorCategory::Other,
        ),
    ];
    for (failure, category) in failures {
        let message = failure.message();
        let error = dispatch::<Recorder>(&bytes, Some(Err(failure))).expect_err(&message);
        assert_eq!(error.category(), category, "{message}");
        assert_eq!(error.message(), message);
        assert_eq!(calls(), 1, "{message}");
    }
}

#[test]
fn a_page_that_breaks_the_contract_fails_the_export_with_its_code() {
    let fixture = fixture();
    for case in fixture["pages"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let Some(code) = case["expect"].as_str().unwrap().strip_prefix("violation:") else {
            continue;
        };
        let limit = u32::try_from(case["limit"].as_u64().unwrap()).unwrap();
        let bytes = encode(&request_of("entry.manga.minimal", None, limit));
        let error = dispatch::<Recorder>(&bytes, Some(Ok(built_page(name))))
            .expect_err(name);
        assert_eq!(error.category(), ErrorCategory::ContractViolation, "{name}");
        assert_eq!(error.message(), code, "{name}");
        assert!(
            matches!(&error, Error::Contract { export, .. } if export == EXPORT),
            "{name}"
        );
        assert_eq!(calls(), 1, "{name}");
    }
}

/// A labelled edit that breaks one chapter rule, and the rule code it must report.
type ChapterRuleCase = (&'static str, fn(&mut Chapter), &'static str);

#[test]
fn chapter_rules_apply_to_every_edition() {
    let with_edit = |edit: fn(&mut Chapter)| {
        let mut chapter = chapter_a();
        edit(&mut chapter);
        page_of(vec![chapter], None, Scope::Visible, Completeness::Partial)
    };
    let cases: [ChapterRuleCase; 6] = [
        ("key too long", |c| c.key = "k".repeat(513), "length.key"),
        ("title too long", |c| c.title = Some("t".repeat(1025)), "length.title"),
        ("revision empty", |c| c.revision = Some(String::new()), "length.revision"),
        ("number NaN", |c| c.chapter = Some(f64::NAN), "number.range"),
        ("date infinite", |c| c.date_updated = Some(f64::INFINITY), "number.date"),
        ("url too long", |c| c.url = Some("u".repeat(4097)), "length.url"),
    ];
    for (name, edit, code) in cases {
        let page = with_edit(edit);
        let error = editions::__encode_page(EXPORT, &page, MAX_LIMIT).expect_err(name);
        assert_eq!(error.message(), code, "{name}");
        assert_eq!(
            editions::validate_page(&page, MAX_LIMIT),
            Err(Violation::Catalog(catalog::Violation::new(
                catalog::Rule::ALL
                    .iter()
                    .copied()
                    .find(|rule| rule.code() == code)
                    .expect("a catalog rule")
            ))),
            "{name}"
        );
    }
}

#[test]
fn the_largest_valid_page_is_far_below_the_response_bound() {
    let big = |index: usize| Chapter {
        title: Some("t".repeat(catalog::MAX_TITLE_BYTES)),
        volume: Some(catalog::MAX_ABS_NUMBER),
        chapter: Some(-catalog::MAX_ABS_NUMBER),
        date_updated: Some(catalog::MAX_ABS_DATE),
        scanlator: Some(Scanlator::with_id(
            "i".repeat(catalog::MAX_ID_BYTES),
            "n".repeat(catalog::MAX_SCANLATOR_NAME_BYTES),
        )),
        url: Some("u".repeat(catalog::MAX_URL_BYTES)),
        lang: Some("l".repeat(catalog::MAX_LANG_BYTES)),
        paywalled: Some(true),
        logical_unit_id: Some("u".repeat(catalog::MAX_ID_BYTES)),
        reading_order: Some(i64::MIN),
        revision: Some("r".repeat(catalog::MAX_REVISION_BYTES)),
        ..Chapter::new(
            format!("{index:03}{}", "k".repeat(catalog::MAX_KEY_BYTES - 3)),
            ContentKind::Images,
        )
    };
    let page = page_of(
        (0..MAX_LIMIT as usize).map(big).collect(),
        Some(&"c".repeat(MAX_CURSOR_BYTES)),
        Scope::AllEligible,
        Completeness::Partial,
    );
    let bytes = editions::__encode_page(EXPORT, &page, MAX_LIMIT).expect("the largest page");
    assert!(bytes.len() < catalog::MAX_RESPONSE_BYTES / 8, "{}", bytes.len());
    let decoded: ChapterEditionsPage = strict(&bytes).expect("round trip");
    assert_eq!(decoded, page);
}

#[test]
fn an_edition_page_reuses_the_chapter_wire_layout() {
    // A page of one chapter is the count, that chapter's bytes, then the page fields.
    let chapter = chapter_a();
    let chapter_bytes = ito_rs::postcard::to_allocvec(&chapter).unwrap();
    let page = page_of(vec![chapter], None, Scope::Visible, Completeness::Complete);
    let mut expected = vec![1];
    expected.extend_from_slice(&chapter_bytes);
    expected.extend_from_slice(&[0, 2, 4]);
    assert_eq!(ito_rs::postcard::to_allocvec(&page).unwrap(), expected);
}
