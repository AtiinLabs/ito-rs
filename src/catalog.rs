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
//!
//! Order of checks for a home layout: components in order. A featured component checks its item
//! count, then its `auto_advance`, then each item in order (its entry, banner and logo lengths,
//! banner and logo schemes, tagline, summary, badge label, badge rank, chip count, each chip's text
//! and then number, accent), then duplicate `(kind, key)` pairs. The first violation wins.

use crate::models::{
    Chapter, HomeComponentValue, HomeLayout, LinkValue, MediaEntry, MediaPageResult,
    featured::{Featured, FeaturedChip, FeaturedItem},
    manga::Manga,
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
/// Encoded size of a `get_media_update`, `handle_url` or `get_chapter_editions` response.
pub const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

/// Items in one featured home component.
pub const MAX_FEATURED_ITEMS: usize = 20;
/// Chips on one featured item.
pub const MAX_FEATURED_CHIPS: usize = 6;
pub const MAX_TAGLINE_BYTES: usize = 256;
pub const MAX_SUMMARY_BYTES: usize = 2_048;
pub const MAX_BADGE_LABEL_BYTES: usize = 64;
pub const MAX_CHIP_TEXT_BYTES: usize = 64;
/// Largest rank a badge shows; ranks start at 1.
pub const MAX_BADGE_RANK: i32 = 999;
/// Bounds of a featured component's `auto_advance`, in seconds.
pub const MIN_AUTO_ADVANCE: f32 = 2.0;
pub const MAX_AUTO_ADVANCE: f32 = 60.0;
/// Largest accent colour, `0xRRGGBB`.
pub const MAX_ACCENT: u32 = 0xFF_FFFF;
/// Years a year chip may show.
pub const MIN_CHIP_YEAR: f64 = 1_800.0;
pub const MAX_CHIP_YEAR: f64 = 2_200.0;
/// Largest value of a rating chip on its normalized scale; the smallest is 0.
pub const MAX_CHIP_RATING: f64 = 10.0;
/// Largest count a units chip may show.
pub const MAX_CHIP_UNITS: f64 = 1_000_000.0;

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
    CountFeatured => "count.featured",
    CountChips => "count.chips",
    LengthText => "length.text",
    UrlScheme => "url.scheme",
    ColorRange => "color.range",
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
            HomeComponentValue::Featured(featured) => validate_featured(featured)?,
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

/// Checks one featured component: its item count, `auto_advance`, each item in order, then
/// duplicate `(kind, key)` pairs.
fn validate_featured(featured: &Featured) -> Check {
    if !(1..=MAX_FEATURED_ITEMS).contains(&featured.items.len()) {
        return fail(Rule::CountFeatured);
    }
    if featured
        .auto_advance
        .is_some_and(|seconds| !(MIN_AUTO_ADVANCE..=MAX_AUTO_ADVANCE).contains(&seconds))
    {
        return fail(Rule::NumberRange);
    }
    featured.items.iter().try_for_each(validate_featured_item)?;
    let mut keys = HashSet::new();
    for item in &featured.items {
        if !keys.insert((item.entry.kind(), item.entry.key())) {
            return fail(Rule::DuplicateKey);
        }
    }
    Ok(())
}

/// Checks one featured item. Both artwork lengths come before both schemes, as the shared order
/// requires.
fn validate_featured_item(item: &FeaturedItem) -> Check {
    validate_entry(&item.entry)?;
    let artwork = [item.banner.as_deref(), item.logo.as_deref()];
    for url in artwork {
        optional_within(url, 1, MAX_URL_BYTES, Rule::LengthUrl)?;
    }
    for url in artwork {
        optional_web_url(url)?;
    }
    optional_within(
        item.tagline.as_deref(),
        1,
        MAX_TAGLINE_BYTES,
        Rule::LengthText,
    )?;
    optional_within(
        item.summary.as_deref(),
        1,
        MAX_SUMMARY_BYTES,
        Rule::LengthText,
    )?;
    if let Some(badge) = &item.badge {
        optional_within(
            badge.label.as_deref(),
            1,
            MAX_BADGE_LABEL_BYTES,
            Rule::LengthText,
        )?;
        if badge
            .rank
            .is_some_and(|rank| !(1..=MAX_BADGE_RANK).contains(&rank))
        {
            return fail(Rule::NumberRange);
        }
    }
    if item.chips.len() > MAX_FEATURED_CHIPS {
        return fail(Rule::CountChips);
    }
    item.chips.iter().try_for_each(validate_featured_chip)?;
    if item.accent.is_some_and(|accent| accent > MAX_ACCENT) {
        return fail(Rule::ColorRange);
    }
    Ok(())
}

/// Checks a chip's text, then its number against the range of its kind.
fn validate_featured_chip(chip: &FeaturedChip) -> Check {
    optional_within(
        chip.text.as_deref(),
        1,
        MAX_CHIP_TEXT_BYTES,
        Rule::LengthText,
    )?;
    match chip.number {
        Some(number) if !chip.kind.accepts_number(number) => fail(Rule::NumberRange),
        _ => Ok(()),
    }
}

fn optional_web_url(value: Option<&str>) -> Check {
    match value {
        Some(url) if !is_web_url(url) => fail(Rule::UrlScheme),
        _ => Ok(()),
    }
}

/// Whether `value` begins with an `http://` or `https://` scheme, compared without regard to ASCII
/// case. Hosts fetch only these schemes directly.
pub(crate) fn is_web_url(value: &str) -> bool {
    let bytes = value.as_bytes();
    ["http://", "https://"].iter().any(|scheme| {
        bytes
            .get(..scheme.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(scheme.as_bytes()))
    })
}
