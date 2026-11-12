# Listing every edition of a chapter

Some sites hold several uploads of one chapter, such as the same chapter from different scanlation
groups, and show only some of them by default. A source may let the user ask for the rest with the
optional `get_chapter_editions` export. It is an additive export of ABI 3: `ito_abi_version()` stays
3, existing plugins keep working unchanged. A plugin built with this SDK always exports it and
answers `Unsupported` unless it implements it; only plugins built with an older SDK lack the export.
Hosts call it only after an explicit user action, never while refreshing a title,
in the background or from the reader.

## Implementing it

A source serving several kinds implements `SourceProvider::get_chapter_editions`; a single-kind
source implements `MangaProvider::get_chapter_editions` or `NovelProvider::get_chapter_editions`,
which receive the typed title. The default is `Err(Error::Unsupported)`, which hosts treat as "this
source cannot list editions". Anime has no editions; an anime source never receives the call.

```rust,ignore
use ito_rs::{
    Result,
    models::{CatalogCompleteness, CatalogScope, ChapterEditionsPage, ChapterEditionsRequest},
};

// Inside `impl SourceProvider for Source`:
fn get_chapter_editions(request: ChapterEditionsRequest) -> Result<ChapterEditionsPage> {
    // `request.entry` has no chapters or catalog; `request.cursor` is None on the first page.
    let (editions, next_cursor) = fetch_page(&request)?; // your own paging, at most `request.limit`
    let completeness = if next_cursor.is_some() {
        CatalogCompleteness::Partial
    } else {
        CatalogCompleteness::Complete
    };
    Ok(ChapterEditionsPage {
        editions,
        next_cursor,
        scope: CatalogScope::Visible,
        completeness,
    })
}
```

Editions are ordinary `Chapter` values in the ABI 3 layout, so every field rule in
[`ABI3-MIGRATION.md`](ABI3-MIGRATION.md) applies: exact `key`, optional `scanlator.id`,
`logical_unit_id`, `reading_order` and `revision`, and a `kind` that says whether a chapter opens
the page reader or the text reader. A page may mix `Text` and `Images` chapters. There is no separate
edition type.

## Wire layout

Both values are Postcard, with the field order below.

| Type | Fields |
|---|---|
| `ChapterEditionsRequest` | `entry: MediaEntry`, `cursor: Option<String>`, `limit: u32` |
| `ChapterEditionsPage` | `editions: Vec<Chapter>`, `next_cursor: Option<String>`, `scope: CatalogScope`, `completeness: CatalogCompleteness` |

`fixtures/chapter-editions-v1.json` pins the bytes and every verdict below, including a page that
mixes a text chapter and an image chapter. `ito-rs` asserts it; hosts pin the same file.

## Rules

| Rule | Value |
|---|---|
| Request entry | manga or novel; its chapter list and catalog are removed before the source sees it, and its key is exact |
| `limit` | 1 to 100 |
| Present cursor (request or `next_cursor`) | 1 to 4 096 UTF-8 bytes |
| Chapters per page | at most the request's `limit` |
| Page with a `next_cursor` | `completeness` is `Partial` and the page holds at least one chapter |
| Last page (no `next_cursor`) | `completeness` may be `Unknown`, `Partial` or `Complete`; an empty page is valid |
| Chapters | the limits and uniqueness rules of ABI 3, per page |
| Encoded page | at most 16 MiB |

- `scope` is a constant of the traversal, the identity of what is listed. `completeness` is not: it
  is `Partial` while pages remain and may become `Complete` on the last page. `Visible` with
  `Complete` says the visible scope is complete, not that every eligible upload is listed.
- A cursor is opaque and compared as exact bytes. A source binds it to the exact title and to the
  settings, account and filters in effect when it issued it, and refuses a stale or foreign cursor
  with an error; it never rebases a token onto another dataset. Cursors stay in memory, are never
  logged and are never backed up. A refusal's message is reported as the source's error text, so it
  must not contain the cursor or any token.
- Each call returns one page. The host calls pages one at a time with the same operation, and stops
  at its own run caps (20 000 rows, 256 pages, 16 MiB of encoded pages in total), reporting the
  result as incomplete instead of exhaustive.

## Errors and validation

The export checks a request before your code runs. A request that breaks a rule fails with
`Error::Host` and the fixed message `request.invalid`; your source is not called. The export
checks the page you return before encoding it and fails with `Error::Contract { export, code }`,
reported to hosts as category `ContractViolation` whose message is the fixed code. The first
violation in this order wins:

| Code | Broken rule |
|---|---|
| `page.cursor` | `next_cursor` present but empty or longer than 4 096 bytes |
| `page.count` | more chapters than the request's `limit` |
| `page.completeness` | `next_cursor` present, completeness not `Partial` |
| `page.no_progress` | `next_cursor` present, no chapters |
| `length.*`, `number.*`, `duplicate.key`, `scanlator.conflict` | the chapter rules of [`ABI3-MIGRATION.md`](ABI3-MIGRATION.md) |
| `response.too_large` | encoded page over 16 MiB |

`Error::Unsupported` means the source does not list editions. Every other error is a failure of the
traversal and is reported as it is: a network error, a refused cursor or a decode failure is never
turned into "unsupported" or an empty page. A host treats `Unsupported` as final only on the first
page; on a later page it is a failure of a traversal that had started.

The host, not the SDK, validates across pages: byte-exact key de-duplication, group id and name
consistency, repeated cursors and a changed `scope`. `ito_rs::editions` exposes the limits and the
per-page checks (`validate_request`, `validate_page`) for a source's own tests.
