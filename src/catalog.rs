//! The chapter catalog contract: shared limits and the validation both sides apply.
//!
//! The export macros validate every entry a plugin returns before encoding it, and hosts apply
//! the same rules to what they decode, with the same numbers (pinned by `fixtures/catalog-v3.json`).
//! A violation fails the export with `Error::Contract`, whose report has category
//! `ContractViolation` and the fixed rule code as its message, never plugin text. Plugins can
//! call the `validate_*` functions in their own tests to see a violation early.
//!
//! Strings are compared as exact UTF-8 bytes, with no normalization, and every length is in
//! UTF-8 bytes.
//!
//! Order of checks for a chapter list: a catalog without chapters, the chapter count, each
//! chapter in order (key, title, scanlator name, scanlator id, logical unit id, revision, url,
//! lang, volume, chapter, date), duplicate keys, then scanlator identifiers that carry two
//! names. Duplicate `reading_order` values are valid: equal values mean the same reading
//! position.

use crate::models::{
    Chapter, HomeComponentValue, HomeLayout, LinkValue, MediaEntry, MediaPageResult, manga::Manga,
    novel::Novel,
};
use std::{collections::HashMap, collections::HashSet, fmt};

/// Chapters in one entry's list.
pub const MAX_CHAPTERS: usize = 20_000;
/// Bytes of a chapter key; keys are never empty.
pub const MAX_KEY_BYTES: usize = 512;
pub const MAX_TITLE_BYTES: usize = 1_024;
/// Bytes of a scanlator display name; names are never empty.
pub const MAX_SCANLATOR_NAME_BYTES: usize = 256;
/// Bytes of a scanlator or logical unit identifier; present identifiers are never empty.
pub const MAX_ID_BYTES: usize = 256;
/// Bytes of a revision token; a present revision is never empty.
pub const MAX_REVISION_BYTES: usize = 256;
pub const MAX_URL_BYTES: usize = 4_096;
pub const MAX_LANG_BYTES: usize = 35;
/// Largest magnitude of a volume or chapter number: finite as a `Float` and safe to convert
/// to an `Int`.
pub const MAX_ABS_NUMBER: f64 = 1e9;
/// Largest magnitude of `date_updated`, in seconds since 1970-01-01 UTC (about the year 5138).
pub const MAX_ABS_DATE: f64 = 1e11;
/// Encoded size of a `get_media_update` or `handle_url` response.
pub const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

macro_rules! rules {
    ($($variant:ident => $code:literal),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum Rule { $($variant),+ }
        impl Rule {
            /// Every rule, in declaration order.
            pub const ALL: &'static [Rule] = &[$(Self::$variant),+];

            pub const fn code(self) -> &'static str {
                match self { $(Self::$variant => $code),+ }
            }
        }
    };
}
rules! {
    CatalogWithoutChapters => "catalog.without_chapters",
    CountChapters => "count.chapters",
    LengthKey => "length.key",
    LengthTitle => "length.title",
    LengthScanlatorName => "length.scanlator_name",
    LengthScanlatorId => "length.scanlator_id",
    LengthLogicalUnitId => "length.logical_unit_id",
    LengthRevision => "length.revision",
    LengthUrl => "length.url",
    LengthLang => "length.lang",
    NumberRange => "number.range",
    NumberDate => "number.date",
    DuplicateKey => "duplicate.key",
    ScanlatorConflict => "scanlator.conflict",
    ResponseTooLarge => "response.too_large",
}

/// A broken catalog rule. It names the rule only, never the offending value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Violation {
    pub rule: Rule,
}

impl Violation {
    pub const fn new(rule: Rule) -> Self {
        Violation { rule }
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.rule.code())
    }
}

impl std::error::Error for Violation {}

type Check = Result<(), Violation>;

fn fail(rule: Rule) -> Check {
    Err(Violation::new(rule))
}

fn bytes_within(value: &str, min: usize, max: usize, rule: Rule) -> Check {
    if (min..=max).contains(&value.len()) {
        Ok(())
    } else {
        fail(rule)
    }
}

fn optional_within(value: Option<&str>, min: usize, max: usize, rule: Rule) -> Check {
    value.map_or(Ok(()), |value| bytes_within(value, min, max, rule))
}

fn number(value: Option<f64>) -> Check {
    match value {
        Some(value) if !(value.is_finite() && value.abs() <= MAX_ABS_NUMBER) => {
            fail(Rule::NumberRange)
        }
        _ => Ok(()),
    }
}

fn date(value: Option<f64>) -> Check {
    match value {
        Some(value) if !(value.is_finite() && value.abs() <= MAX_ABS_DATE) => {
            fail(Rule::NumberDate)
        }
        _ => Ok(()),
    }
}

/// Checks one chapter on its own; list-wide rules are in [`validate_chapters`].
pub fn validate_chapter(chapter: &Chapter) -> Check {
    bytes_within(&chapter.key, 1, MAX_KEY_BYTES, Rule::LengthKey)?;
    optional_within(
        chapter.title.as_deref(),
        0,
        MAX_TITLE_BYTES,
        Rule::LengthTitle,
    )?;
    if let Some(scanlator) = &chapter.scanlator {
        bytes_within(
            &scanlator.name,
            1,
            MAX_SCANLATOR_NAME_BYTES,
            Rule::LengthScanlatorName,
        )?;
        optional_within(
            scanlator.id.as_deref(),
            1,
            MAX_ID_BYTES,
            Rule::LengthScanlatorId,
        )?;
    }
    optional_within(
        chapter.logical_unit_id.as_deref(),
        1,
        MAX_ID_BYTES,
        Rule::LengthLogicalUnitId,
    )?;
    optional_within(
        chapter.revision.as_deref(),
        1,
        MAX_REVISION_BYTES,
        Rule::LengthRevision,
    )?;
    optional_within(chapter.url.as_deref(), 0, MAX_URL_BYTES, Rule::LengthUrl)?;
    optional_within(chapter.lang.as_deref(), 0, MAX_LANG_BYTES, Rule::LengthLang)?;
    number(chapter.volume)?;
    number(chapter.chapter)?;
    date(chapter.date_updated)
}

/// Checks one chapter list: its size, each chapter, unique keys and consistent group names.
pub fn validate_chapters(chapters: &[Chapter]) -> Check {
    if chapters.len() > MAX_CHAPTERS {
        return fail(Rule::CountChapters);
    }
    for chapter in chapters {
        validate_chapter(chapter)?;
    }
    let mut keys = HashSet::with_capacity(chapters.len());
    if !chapters
        .iter()
        .all(|chapter| keys.insert(chapter.key.as_str()))
    {
        return fail(Rule::DuplicateKey);
    }
    let mut groups: HashMap<&str, &str> = HashMap::new();
    for scanlator in chapters
        .iter()
        .filter_map(|chapter| chapter.scanlator.as_ref())
    {
        if let Some(id) = scanlator.id.as_deref()
            && *groups.entry(id).or_insert(&scanlator.name) != scanlator.name
        {
            return fail(Rule::ScanlatorConflict);
        }
    }
    Ok(())
}

fn validate_list(chapters: Option<&[Chapter]>, has_catalog: bool) -> Check {
    match chapters {
        None if has_catalog => fail(Rule::CatalogWithoutChapters),
        None => Ok(()),
        Some(chapters) => validate_chapters(chapters),
    }
}

pub fn validate_manga(manga: &Manga) -> Check {
    validate_list(manga.chapters.as_deref(), manga.catalog.is_some())
}

pub fn validate_novel(novel: &Novel) -> Check {
    validate_list(novel.chapters.as_deref(), novel.catalog.is_some())
}

/// Checks the chapter list an entry carries; anime has none.
pub fn validate_entry(entry: &MediaEntry) -> Check {
    match entry {
        MediaEntry::Manga(manga) => validate_manga(manga),
        MediaEntry::Novel(novel) => validate_novel(novel),
        MediaEntry::Anime(_) => Ok(()),
    }
}

pub fn validate_page_result(page: &MediaPageResult) -> Check {
    page.entries.iter().try_for_each(validate_entry)
}

pub fn validate_link(link: &LinkValue) -> Check {
    match link {
        LinkValue::Manga(manga) => validate_manga(manga),
        LinkValue::Novel(novel) => validate_novel(novel),
        LinkValue::Url(_) | LinkValue::Anime(_) | LinkValue::Listing(_) => Ok(()),
    }
}

/// Checks every entry and chapter a home layout carries.
pub fn validate_home(home: &HomeLayout) -> Check {
    for component in &home.components {
        match &component.value {
            HomeComponentValue::Scroller(entries, _)
            | HomeComponentValue::MangaList(_, _, entries, _)
            | HomeComponentValue::BigScroller(entries, _) => {
                entries.iter().try_for_each(validate_manga)?
            }
            HomeComponentValue::NovelScroller(entries, _)
            | HomeComponentValue::NovelList(_, _, entries, _)
            | HomeComponentValue::NovelBigScroller(entries, _) => {
                entries.iter().try_for_each(validate_novel)?
            }
            HomeComponentValue::MangaChapterList(_, items, _) => {
                for item in items {
                    validate_manga(&item.manga)?;
                    validate_chapter(&item.chapter)?;
                }
            }
            HomeComponentValue::NovelChapterList(_, items, _) => {
                for item in items {
                    validate_novel(&item.novel)?;
                    validate_chapter(&item.chapter)?;
                }
            }
            HomeComponentValue::MediaScroller(entries, _)
            | HomeComponentValue::MediaList(_, _, entries, _)
            | HomeComponentValue::MediaBigScroller(entries, _) => {
                entries.iter().try_for_each(validate_entry)?
            }
            HomeComponentValue::AnimeScroller(..)
            | HomeComponentValue::AnimeList(..)
            | HomeComponentValue::AnimeEpisodeList(..)
            | HomeComponentValue::AnimeBigScroller(..)
            | HomeComponentValue::Filters(_)
            | HomeComponentValue::Links(_) => {}
        }
    }
    Ok(())
}
