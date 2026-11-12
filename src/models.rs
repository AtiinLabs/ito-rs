use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub mod anime;
pub mod manga;
pub mod novel;

/// What a catalog entry is. One source may serve several kinds; it declares them with
/// `SourceProvider::KINDS`.
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum MediaKind {
    Manga = 0,
    Anime = 1,
    Novel = 2,
}

impl MediaKind {
    /// The bitmask hosts read from `ito_supported_kinds`: bit `n` is the kind with value `n`.
    pub const fn mask(kinds: &[MediaKind]) -> u32 {
        let mut mask = 0;
        let mut index = 0;
        while index < kinds.len() {
            mask |= 1 << kinds[index] as u32;
            index += 1;
        }
        mask
    }
}

/// How a chapter is read, whatever its entry's kind: `Images` opens the page reader on its
/// pages; `Text` opens the text reader, which lays out text pages and shows image pages inline.
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ContentKind {
    Images = 0,
    Text = 1,
}

/// The scanlation group that uploaded a chapter.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Scanlator {
    /// A stable identifier of the group within this source, when the source has one. `None`
    /// means unknown; never invent an identifier from the display name.
    pub id: Option<String>,
    /// The group's display name. Renaming a group never renames the upload it made.
    pub name: String,
}

impl Scanlator {
    /// A group known only by its display name.
    pub fn named(name: impl Into<String>) -> Self {
        Scanlator {
            id: None,
            name: name.into(),
        }
    }

    /// A group with a stable source-scoped identifier.
    pub fn with_id(id: impl Into<String>, name: impl Into<String>) -> Self {
        Scanlator {
            id: Some(id.into()),
            name: name.into(),
        }
    }
}

/// A chapter of a manga or novel: one exact upload.
///
/// Build it with [`Chapter::new`] and functional update syntax, so a field added by a later
/// release is a recompile and not a source edit:
/// `Chapter { title, chapter, ..Chapter::new(key, kind) }`.
///
/// Limits (see [`crate::catalog`]): numbers are finite and bounded in magnitude; strings are
/// bounded and compared as exact UTF-8 bytes; keys are unique within one chapter list.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Chapter {
    /// The stable identifier of this exact upload within its entry, used to fetch its pages and
    /// to remember reading and downloads. It is not a content hash and is not unique across
    /// sources or entries.
    pub key: String,
    pub title: Option<String>,
    /// Display and ordering number of the volume. Equal numbers never prove two chapters are
    /// the same reading position.
    pub volume: Option<f64>,
    /// Display and ordering number of the chapter. Equal numbers never prove two chapters are
    /// the same reading position.
    pub chapter: Option<f64>,
    /// When the upload was published, in seconds since 1970-01-01 UTC. Finite and at most
    /// 1e11 in magnitude.
    pub date_updated: Option<f64>,
    pub scanlator: Option<Scanlator>,
    pub url: Option<String>,
    /// The language of this upload, independent of the scanlator.
    pub lang: Option<String>,
    /// Locked for the signed-in account or session. `None` means unknown.
    pub paywalled: Option<bool>,
    /// Chosen per chapter, so one series can mix page volumes with text volumes.
    pub kind: ContentKind,
    /// Identifies the reading position this upload belongs to: every upload of one entry that
    /// the source itself declares to be the same position carries the same id. The id is scoped
    /// to the entry, stays stable across refreshes, and is never reused for a different
    /// position. Emit it only with credible source-side evidence: the same number, title,
    /// volume, scanlator or date alone is not enough. `None` means unknown.
    pub logical_unit_id: Option<String>,
    /// Orders reading positions within one response: lower values are read earlier, values are
    /// comparable only with others in the same response, and gaps are allowed. Equal values
    /// mean the same reading position (for example alternative uploads of one chapter). It is
    /// an ordering hint for unnumbered or irregular releases, independent of the order of the
    /// list, and not a durable identity.
    pub reading_order: Option<i64>,
    /// A token that changes exactly when this upload's content changes. Set it only when the
    /// source provides one tied to the content; never use a timestamp. When present it is not
    /// empty. `None` means unknown.
    pub revision: Option<String>,
}

impl Chapter {
    /// A chapter with only its required fields; every optional field is unknown.
    pub fn new(key: impl Into<String>, kind: ContentKind) -> Self {
        Chapter {
            key: key.into(),
            title: None,
            volume: None,
            chapter: None,
            date_updated: None,
            scanlator: None,
            url: None,
            lang: None,
            paywalled: None,
            kind,
            logical_unit_id: None,
            reading_order: None,
            revision: None,
        }
    }
}

/// Which chapters a returned list covers. Encoded like the other model enums, as an i32
/// discriminant; the values are the wire format.
///
/// Evolution rule: a discriminant a reader does not know decodes to `Unknown`, so a later
/// release can add scopes without a new ABI.
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CatalogScope {
    #[serde(other)]
    Unknown = 0,
    /// What the source shows by default, which may collapse alternative uploads.
    Visible = 1,
    /// Every eligible alternative upload is listed.
    AllEligible = 2,
}

/// Whether a chapter list holds everything within its [`CatalogScope`], for the signed-in
/// account and the source settings in effect. A discriminant a reader does not know decodes to
/// `Unknown`, like [`CatalogScope`].
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CatalogCompleteness {
    #[serde(other)]
    Unknown = 0,
    Partial = 1,
    Complete = 2,
}

/// What an entry's chapter list covers. An entry without one has an unknown scope and
/// completeness. A catalog requires the entry's `chapters` to be present: set it whenever you
/// set `chapters`. The entry passed to `get_media_update` never carries a catalog.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChapterCatalog {
    pub scope: CatalogScope,
    pub completeness: CatalogCompleteness,
}

/// The request of the optional `get_chapter_editions` export: one page of every alternative
/// upload of a manga or novel's chapters. See [`crate::editions`] for the rules.
///
/// `entry` is the title as the host holds it, with its chapter list and catalog removed before
/// the source sees it; its key is exact. `cursor` is `None` on the first page and otherwise the
/// opaque `next_cursor` of the previous page, byte for byte. `limit` is the most chapters the
/// source may return, 1 to 100.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChapterEditionsRequest {
    pub entry: MediaEntry,
    pub cursor: Option<String>,
    pub limit: u32,
}

/// One page of alternative uploads, answering a [`ChapterEditionsRequest`].
///
/// Editions are ordinary [`Chapter`] values in the ABI 3 layout. `scope` and `completeness` use
/// the same enums as [`ChapterCatalog`], but `scope` stays constant through a traversal and
/// `completeness` does not: every page that carries a `next_cursor` is `Partial`, and the last
/// page, which has none, may be `Unknown`, `Partial` or `Complete`. `Complete` on the last page
/// says the traversal covered everything within `scope`, no more.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ChapterEditionsPage {
    pub editions: Vec<Chapter>,
    pub next_cursor: Option<String>,
    pub scope: CatalogScope,
    pub completeness: CatalogCompleteness,
}

/// A catalog entry of any kind.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MediaEntry {
    Manga(manga::Manga),
    Anime(anime::Anime),
    Novel(novel::Novel),
}

impl MediaEntry {
    pub fn kind(&self) -> MediaKind {
        match self {
            MediaEntry::Manga(_) => MediaKind::Manga,
            MediaEntry::Anime(_) => MediaKind::Anime,
            MediaEntry::Novel(_) => MediaKind::Novel,
        }
    }

    pub fn key(&self) -> &str {
        match self {
            MediaEntry::Manga(manga) => &manga.key,
            MediaEntry::Anime(anime) => &anime.key,
            MediaEntry::Novel(novel) => &novel.key,
        }
    }
}

/// One page of catalog entries; a source serving several kinds may mix them.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaPageResult {
    pub entries: Vec<MediaEntry>,
    pub has_next_page: bool,
}

impl From<manga::PageResult> for MediaPageResult {
    fn from(page: manga::PageResult) -> Self {
        MediaPageResult {
            entries: page.entries.into_iter().map(MediaEntry::Manga).collect(),
            has_next_page: page.has_next_page,
        }
    }
}

impl From<anime::PageResult> for MediaPageResult {
    fn from(page: anime::PageResult) -> Self {
        MediaPageResult {
            entries: page.entries.into_iter().map(MediaEntry::Anime).collect(),
            has_next_page: page.has_next_page,
        }
    }
}

impl From<novel::PageResult> for MediaPageResult {
    fn from(page: novel::PageResult) -> Self {
        MediaPageResult {
            entries: page.entries.into_iter().map(MediaEntry::Novel).collect(),
            has_next_page: page.has_next_page,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[non_exhaustive]
pub enum PageContent {
    Url(String),
    Text(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Page {
    pub index: i32,
    pub content: PageContent,
    pub has_description: bool,
    pub description: Option<String>,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// An image the host can fetch without holding the plugin's execution lane.
///
/// `url` is an absolute HTTP(S) URL. The host only fetches it when its origin matches one
/// returned by the provider's inputless `get_image_origins`. Headers are private request
/// material, never part of the persisted image locator or logs. Cross-origin redirects are
/// refused, so servers must expose the image at the declared origin.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PluginImageRequest {
    pub url: String,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Listing {
    pub id: String,
    pub name: String,
    /// Reserved; hosts pass it back unchanged. Set 0.
    pub kind: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// A value carried by a search filter. Rich controls use canonical strings in `String`.
#[non_exhaustive]
pub enum FilterValue {
    Boolean(bool),
    Int(i64),
    Float(f64),
    String(String),
}

/// A named search control value. Rich-capable hosts send dense canonical-string filters.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FilterItem {
    /// Rich-control kind wire name, or the caller's existing filter type.
    pub type_name: String, // swift `type`
    /// Stable id of the search control.
    pub name: String,
    /// Canonical rich-control value or an existing filter value.
    pub value: FilterValue,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[non_exhaustive]
pub enum LinkValue {
    Url(String),
    Manga(manga::Manga),
    Anime(anime::Anime),
    Novel(novel::Novel),
    Listing(Listing),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Link {
    pub title: String,
    pub value: Option<LinkValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MangaWithChapter {
    pub manga: manga::Manga,
    pub chapter: Chapter,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnimeWithEpisode {
    pub anime: anime::Anime,
    pub episode: anime::Episode,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NovelWithChapter {
    pub novel: novel::Novel,
    pub chapter: Chapter,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[non_exhaustive]
pub enum HomeComponentValue {
    Scroller(Vec<manga::Manga>, Option<Listing>),
    MangaList(bool, Option<i32>, Vec<manga::Manga>, Option<Listing>),
    MangaChapterList(Option<i32>, Vec<MangaWithChapter>, Option<Listing>),
    AnimeScroller(Vec<anime::Anime>, Option<Listing>),
    AnimeList(bool, Option<i32>, Vec<anime::Anime>, Option<Listing>),
    AnimeEpisodeList(Option<i32>, Vec<AnimeWithEpisode>, Option<Listing>),
    BigScroller(Vec<manga::Manga>, Option<f32>),
    AnimeBigScroller(Vec<anime::Anime>, Option<f32>),
    NovelScroller(Vec<novel::Novel>, Option<Listing>),
    NovelList(bool, Option<i32>, Vec<novel::Novel>, Option<Listing>),
    NovelChapterList(Option<i32>, Vec<NovelWithChapter>, Option<Listing>),
    NovelBigScroller(Vec<novel::Novel>, Option<f32>),
    Filters(Vec<FilterItem>),
    Links(Vec<Link>),
    /// Entries of any kind, for sources serving several.
    MediaScroller(Vec<MediaEntry>, Option<Listing>),
    MediaList(bool, Option<i32>, Vec<MediaEntry>, Option<Listing>),
    MediaBigScroller(Vec<MediaEntry>, Option<f32>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HomeComponent {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub value: HomeComponentValue,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HomeLayout {
    pub components: Vec<HomeComponent>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[non_exhaustive]
pub enum Setting {
    Toggle {
        id: String,
        name: String,
        summary: Option<String>,
        default_value: bool,
    },
    Text {
        id: String,
        name: String,
        summary: Option<String>,
        default_value: String,
    },
    Picker {
        id: String,
        name: String,
        summary: Option<String>,
        options: Vec<String>,
        default_value: String,
    },
    /// A credential such as an API key or password. The host keeps it out of plain
    /// storage and backups, shows it as a masked field, and never displays the saved
    /// value. Plugins read it with `defaults::get(id)`, which returns `None` until the
    /// user enters one; writing it from the plugin is not supported.
    Secret {
        id: String,
        name: String,
        summary: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SettingsSchema {
    pub settings: Vec<Setting>,
}

/// A non-destructive diagnostic offered by a plugin's settings screen.
/// Kept separate from SettingsSchema so older hosts can still decode that schema.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SettingsAction {
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
}

/// An advisory settings problem. `key` is a Setting id, or empty for a form-level issue.
/// Messages must not include credentials, URLs, or other setting values.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SettingIssue {
    pub key: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_payloads_follow_the_separate_host_wire_contract() {
        let actions = vec![SettingsAction {
            id: "probe".into(),
            title: "Test".into(),
            summary: Some("Hi".into()),
        }];
        assert_eq!(
            postcard::to_allocvec(&actions).unwrap(),
            b"\x01\x05probe\x04Test\x01\x02Hi"
        );
        let issues = vec![
            SettingIssue {
                key: "url".into(),
                message: "Missing".into(),
            },
            SettingIssue {
                key: String::new(),
                message: "Setup".into(),
            },
        ];
        assert_eq!(
            postcard::to_allocvec(&issues).unwrap(),
            b"\x02\x03url\x07Missing\x00\x05Setup"
        );
    }

    // Hosts decode these positionally, so values, field order and variant indices are the wire.
    #[test]
    fn kinds_and_chapters_have_a_fixed_wire_layout() {
        // i32 discriminants travel as zigzag varints.
        assert_eq!(postcard::to_allocvec(&MediaKind::Manga).unwrap(), [0]);
        assert_eq!(postcard::to_allocvec(&MediaKind::Anime).unwrap(), [2]);
        assert_eq!(postcard::to_allocvec(&MediaKind::Novel).unwrap(), [4]);
        assert_eq!(postcard::to_allocvec(&ContentKind::Images).unwrap(), [0]);
        assert_eq!(postcard::to_allocvec(&ContentKind::Text).unwrap(), [2]);
        assert!(postcard::from_bytes::<ContentKind>(&[4]).is_err());
        assert_eq!(
            MediaKind::mask(&[MediaKind::Novel, MediaKind::Manga]),
            0b101
        );

        let chapter = Chapter::new("k", ContentKind::Text);
        // Key, eight absent options, the kind, then the three ABI 3 options.
        assert_eq!(
            postcard::to_allocvec(&chapter).unwrap(),
            [1, b'k', 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0]
        );

        // f64 numbers are eight little-endian bytes behind a presence byte; the scanlator is an
        // option of (optional id, name); reading order is a zigzag varint.
        let chapter = Chapter {
            chapter: Some(1.5),
            scanlator: Some(Scanlator::with_id("g", "G")),
            logical_unit_id: Some("u".into()),
            reading_order: Some(-1),
            revision: Some("r".into()),
            ..Chapter::new("k", ContentKind::Images)
        };
        let mut expected = vec![1, b'k', 0, 0, 1];
        expected.extend_from_slice(&1.5f64.to_le_bytes());
        expected.extend_from_slice(&[0, 1, 1, 1, b'g', 1, b'G', 0, 0, 0, 0]);
        expected.extend_from_slice(&[1, 1, b'u', 1, 1, 1, 1, b'r']);
        assert_eq!(postcard::to_allocvec(&chapter).unwrap(), expected);
        assert_eq!(postcard::from_bytes::<Chapter>(&expected).unwrap(), chapter);

        // Scope and completeness are i32 discriminants too.
        let catalog = ChapterCatalog {
            scope: CatalogScope::AllEligible,
            completeness: CatalogCompleteness::Partial,
        };
        assert_eq!(postcard::to_allocvec(&catalog).unwrap(), [4, 2]);
        assert_eq!(
            postcard::from_bytes::<CatalogScope>(&[6]).unwrap(),
            CatalogScope::Unknown
        );
        assert_eq!(
            postcard::from_bytes::<CatalogCompleteness>(&[6]).unwrap(),
            CatalogCompleteness::Unknown
        );
    }

    #[test]
    fn mixed_entries_and_home_components_keep_their_variant_indices() {
        let novel = novel::Novel {
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
            catalog: None,
        };
        let entry = postcard::to_allocvec(&MediaEntry::Novel(novel.clone())).unwrap();
        assert_eq!(entry[0], 2);
        let decoded: MediaEntry = postcard::from_bytes(&entry).unwrap();
        assert_eq!(decoded.kind(), MediaKind::Novel);
        assert_eq!(decoded.key(), "n");

        let index = |value: HomeComponentValue| postcard::to_allocvec(&value).unwrap()[0];
        assert_eq!(index(HomeComponentValue::Links(Vec::new())), 13);
        assert_eq!(
            index(HomeComponentValue::MediaScroller(
                vec![MediaEntry::Novel(novel)],
                None
            )),
            14
        );
        assert_eq!(
            index(HomeComponentValue::MediaList(false, None, Vec::new(), None)),
            15
        );
        assert_eq!(
            index(HomeComponentValue::MediaBigScroller(Vec::new(), None)),
            16
        );
    }

    // Hosts decode settings by postcard variant index, so existing variants must keep
    // their indices and new ones are appended.
    #[test]
    fn setting_variants_keep_their_wire_indices() {
        let schema = SettingsSchema {
            settings: vec![
                Setting::Toggle {
                    id: "t".into(),
                    name: "T".into(),
                    summary: None,
                    default_value: true,
                },
                Setting::Text {
                    id: "x".into(),
                    name: "X".into(),
                    summary: None,
                    default_value: String::new(),
                },
                Setting::Picker {
                    id: "p".into(),
                    name: "P".into(),
                    summary: None,
                    options: vec!["a".into()],
                    default_value: "a".into(),
                },
                Setting::Secret {
                    id: "s".into(),
                    name: "S".into(),
                    summary: Some("hint".into()),
                },
            ],
        };
        let bytes = postcard::to_allocvec(&schema).unwrap();

        // Vec length 4, then each setting starts with its variant index.
        assert_eq!(bytes[0], 4);
        assert_eq!(bytes[1], 0);
        let secret_start = bytes.len() - postcard::to_allocvec(&schema.settings[3]).unwrap().len();
        assert_eq!(bytes[secret_start], 3);

        let decoded: SettingsSchema = postcard::from_bytes(&bytes).unwrap();
        match &decoded.settings[3] {
            Setting::Secret { id, name, summary } => {
                assert_eq!(id, "s");
                assert_eq!(name, "S");
                assert_eq!(summary.as_deref(), Some("hint"));
            }
            other => panic!("expected secret, got {other:?}"),
        }
    }
}
