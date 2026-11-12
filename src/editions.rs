//! Bounded enumeration of alternative chapter uploads: the optional `get_chapter_editions`
//! export, its limits and the per-page validation both sides apply.
//!
//! A source that lists several uploads of one chapter, such as the same chapter from different
//! scanlation groups, may return them page by page, beyond what `get_media_update` puts in the
//! entry's chapter list. The host asks only after an explicit user action. A source that does not
//! implement [`SourceProvider::get_chapter_editions`](crate::provider::SourceProvider::get_chapter_editions)
//! opts out with [`Error::Unsupported`], and hosts then treat the export as absent.
//!
//! - the entry is a manga or novel with its chapter list and catalog removed; its key is exact;
//! - `limit` is 1 to [`MAX_LIMIT`]; the page never holds more chapters;
//! - a present `cursor`, in a request or as a page's `next_cursor`, is 1 to [`MAX_CURSOR_BYTES`]
//!   UTF-8 bytes. Cursors are opaque, compared as exact bytes, kept in memory only and never
//!   logged. A source binds each cursor to the exact title and to the settings, account and filters
//!   in effect, and refuses a stale or foreign one instead of rebasing it onto another dataset.
//!   An error message is reported as the source's own text, so it never includes the cursor;
//! - a page with a `next_cursor` is `Partial` and holds at least one chapter, so a traversal always
//!   progresses; the last page may be `Unknown`, `Partial` or `Complete`;
//! - every chapter obeys the rules of [`catalog`], and chapter keys are unique within a page;
//! - the encoded page is at most [`catalog::MAX_RESPONSE_BYTES`].
//!
//! The SDK validates the shape of each page. The host validates across pages: duplicate keys,
//! metadata consistency, repeated cursors, a constant `scope`, and its own caps on pages, rows and
//! cumulative bytes.
//!
//! A malformed request fails with [`Error::Host`] and the fixed message `request.invalid` before
//! source code runs. A page that breaks a rule fails with [`Error::Contract`], whose code is
//! [`Violation::code`]; chapter rules keep their [`catalog`] codes. Plugin text never appears in
//! either. Any other error the source returns, such as a network failure, is passed on unchanged.
//!
//! `fixtures/chapter-editions-v1.json` pins the wire layouts and every verdict.

use crate::catalog;
use crate::models::{CatalogCompleteness, ChapterEditionsPage, ChapterEditionsRequest, MediaEntry};
use crate::{Error, Result};
use std::fmt;

/// The most chapters one page may carry, and the largest request `limit`.
pub const MAX_LIMIT: u32 = 100;
/// Bytes of a present cursor; cursors are never empty.
pub const MAX_CURSOR_BYTES: usize = 4_096;

/// A broken rule of the editions contract. It names the rule only, never the offending value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Violation {
    /// The request entry is not a manga or novel.
    RequestEntryKind,
    /// The request `limit` is outside 1 to [`MAX_LIMIT`].
    RequestLimit,
    /// The request cursor is present but empty or longer than [`MAX_CURSOR_BYTES`].
    RequestCursor,
    /// The page's `next_cursor` is present but empty or longer than [`MAX_CURSOR_BYTES`].
    PageCursor,
    /// The page holds more chapters than the request `limit`.
    PageCount,
    /// The page has a `next_cursor` but is not `Partial`.
    PageCompleteness,
    /// The page has a `next_cursor` but no chapters.
    PageNoProgress,
    /// A chapter or the encoded size broke a [`catalog`] rule.
    Catalog(catalog::Violation),
}

impl Violation {
    /// The fixed code reported for this violation.
    pub const fn code(self) -> &'static str {
        match self {
            Violation::RequestEntryKind => "request.entry_kind",
            Violation::RequestLimit => "request.limit",
            Violation::RequestCursor => "request.cursor",
            Violation::PageCursor => "page.cursor",
            Violation::PageCount => "page.count",
            Violation::PageCompleteness => "page.completeness",
            Violation::PageNoProgress => "page.no_progress",
            Violation::Catalog(violation) => violation.rule.code(),
        }
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for Violation {}

impl From<catalog::Violation> for Violation {
    fn from(violation: catalog::Violation) -> Self {
        Violation::Catalog(violation)
    }
}

fn cursor_within_bounds(cursor: &str) -> bool {
    (1..=MAX_CURSOR_BYTES).contains(&cursor.len())
}

/// Checks a request: a manga or novel entry, a limit of 1 to [`MAX_LIMIT`] and a present cursor
/// of 1 to [`MAX_CURSOR_BYTES`] bytes, in that order.
///
/// # Errors
/// The first broken rule.
pub fn validate_request(request: &ChapterEditionsRequest) -> std::result::Result<(), Violation> {
    if matches!(request.entry, MediaEntry::Anime(_)) {
        return Err(Violation::RequestEntryKind);
    }
    if !(1..=MAX_LIMIT).contains(&request.limit) {
        return Err(Violation::RequestLimit);
    }
    if request
        .cursor
        .as_deref()
        .is_some_and(|cursor| !cursor_within_bounds(cursor))
    {
        return Err(Violation::RequestCursor);
    }
    Ok(())
}

/// Checks the shape of one page answering a request with this `limit`: the next cursor, the
/// chapter count, completeness, progress, then the chapters (see [`catalog::validate_chapters`]).
/// The encoded size is checked when the export packs the page.
///
/// # Errors
/// The first broken rule.
pub fn validate_page(page: &ChapterEditionsPage, limit: u32) -> std::result::Result<(), Violation> {
    let continues = match page.next_cursor.as_deref() {
        Some(cursor) if !cursor_within_bounds(cursor) => return Err(Violation::PageCursor),
        Some(_) => true,
        None => false,
    };
    if !u32::try_from(page.editions.len()).is_ok_and(|count| count <= limit) {
        return Err(Violation::PageCount);
    }
    if continues && page.completeness != CatalogCompleteness::Partial {
        return Err(Violation::PageCompleteness);
    }
    if continues && page.editions.is_empty() {
        return Err(Violation::PageNoProgress);
    }
    Ok(catalog::validate_chapters(&page.editions)?)
}

/// Encodes a page after validating it against `limit` and the response bound.
///
/// # Errors
/// [`Error::Contract`] for `export` when a rule is broken, or the encoding error.
#[doc(hidden)]
pub fn __encode_page(export: &str, page: &ChapterEditionsPage, limit: u32) -> Result<Vec<u8>> {
    let invalid = |violation: Violation| Error::Contract {
        export: export.to_owned(),
        code: violation.code(),
    };
    validate_page(page, limit).map_err(invalid)?;
    let bytes = postcard::to_allocvec(page)?;
    if bytes.len() > catalog::MAX_RESPONSE_BYTES {
        return Err(invalid(
            catalog::Violation::new(catalog::Rule::ResponseTooLarge).into(),
        ));
    }
    Ok(bytes)
}
