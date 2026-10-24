use serde::{Deserialize, Serialize};

pub mod anime;
pub mod manga;
pub mod novel;

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
#[non_exhaustive]
pub enum FilterValue {
    Boolean(bool),
    Int(i64),
    Float(f64),
    String(String),
}

/// A search filter value. Reserved: hosts currently always send an empty filter list.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FilterItem {
    pub type_name: String, // swift `type`
    pub name: String,
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
    pub chapter: manga::Chapter,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnimeWithEpisode {
    pub anime: anime::Anime,
    pub episode: anime::Episode,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NovelWithChapter {
    pub novel: novel::Novel,
    pub chapter: novel::Chapter,
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
            SettingIssue { key: "url".into(), message: "Missing".into() },
            SettingIssue { key: String::new(), message: "Setup".into() },
        ];
        assert_eq!(
            postcard::to_allocvec(&issues).unwrap(),
            b"\x02\x03url\x07Missing\x00\x05Setup"
        );
    }

    // Hosts decode settings by postcard variant index, so existing variants must keep
    // their indices and new ones are appended.
    #[test]
    fn setting_variants_keep_their_wire_indices() {
        let schema = SettingsSchema {
            settings: vec![
                Setting::Toggle { id: "t".into(), name: "T".into(), summary: None, default_value: true },
                Setting::Text { id: "x".into(), name: "X".into(), summary: None, default_value: String::new() },
                Setting::Picker {
                    id: "p".into(),
                    name: "P".into(),
                    summary: None,
                    options: vec!["a".into()],
                    default_value: "a".into(),
                },
                Setting::Secret { id: "s".into(), name: "S".into(), summary: Some("hint".into()) },
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
