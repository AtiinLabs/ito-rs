use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone)]
#[repr(i32)]
pub enum Status {
    Unknown = 0,
    Ongoing = 1,
    Completed = 2,
    Cancelled = 3,
    Hiatus = 4,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone)]
#[repr(i32)]
pub enum ContentRating {
    Safe = 0,
    Suggestive = 1,
    Nsfw = 2,
}

pub use super::{Chapter, ChapterCatalog};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Novel {
    pub key: String,
    pub title: String,
    pub authors: Option<Vec<String>>,
    pub artist: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub cover: Option<String>,
    pub url: Option<String>,
    pub status: Status,
    pub content_rating: ContentRating,
    pub nsfw: i32,
    pub chapters: Option<Vec<Chapter>>,
    /// What `chapters` covers. `None` means unknown, and it must be `None` when `chapters` is.
    /// Set it whenever you set `chapters`; the entry passed to `get_media_update` never
    /// carries a catalog.
    pub catalog: Option<ChapterCatalog>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PageResult {
    pub entries: Vec<Novel>,
    pub has_next_page: bool,
}
