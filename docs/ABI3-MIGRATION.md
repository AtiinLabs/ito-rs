# Migrating a plugin from ABI 2 to ABI 3

`ito-rs` 0.8 exports `ito_abi_version() == 3`. Hosts that support ABI 3 run ABI 3 plugins only and
ask the user to update older sources, so every plugin must be ported and rebuilt. No export changed
its name or signature; only the layout of `Chapter`, `Manga` and `Novel` did. Plugins built for
ABI 1 also need the ABI 2 changes (`MediaKind`, `ContentKind`, `ito_supported_kinds`).

## What changed

| Type | ABI 2 | ABI 3 |
|---|---|---|
| `Chapter.volume`, `Chapter.chapter` | `Option<f32>` | `Option<f64>` |
| `Chapter.scanlator` | `Option<String>` | `Option<Scanlator>`, with `Scanlator { id: Option<String>, name: String }` |
| `Chapter` | ends at `kind` | adds `logical_unit_id: Option<String>`, `reading_order: Option<i64>`, `revision: Option<String>` after `kind` |
| `Manga`, `Novel` | end at `chapters` | add `catalog: Option<ChapterCatalog>` after `chapters` |

`ChapterCatalog { scope: CatalogScope, completeness: CatalogCompleteness }` says what the returned
`chapters` list covers:

- `CatalogScope`: `Unknown`, `Visible` (what the site shows by default, which may collapse
  alternative uploads) or `AllEligible` (every eligible alternative upload is listed).
- `CatalogCompleteness`: `Unknown`, `Partial` or `Complete`, for the signed-in account and the source
  settings in effect. It does not claim to be every chapter on the website for every account.

`Anime`, `Episode`, `Page` and every export signature are unchanged.

## Porting steps

1. Depend on the ABI 3 SDK and keep the build reproducible: pin `ito-rs` to a commit with
   `git = "https://github.com/AtiinLabs/ito-rs", rev = "<sha>"`, commit `Cargo.lock`, and build
   with `--locked`.
2. Build chapters with `Chapter::new` and functional update syntax. A field added by a later SDK is
   then a recompile, not an edit to every literal:

   ```rust
   use ito_rs::models::{Chapter, ContentKind, Scanlator};

   let chapter = Chapter {
       title: Some(title),
       chapter: Some(number),                 // f64: stop casting to f32
       date_updated: Some(timestamp),
       scanlator: Some(Scanlator::with_id(group_id, group_name)),
       ..Chapter::new(key, ContentKind::Images)
   };
   ```

3. Replace `scanlator: Some(name)` with `Some(Scanlator::named(name))`, or
   `Some(Scanlator::with_id(id, name))` when the site gives a stable group identifier. Use `None`
   when there is no group.
4. Drop `as f32` casts and parse numbers as `f64`.
5. Add `catalog: None` to every `Manga` and `Novel` literal (or set it, see below).
6. Rebuild, set the manifest's `min_app_version` to the first Ito release that supports ABI 3 (1.5.0),
   and bump the package version. Package with `ito-pkg`; where your `ito-pkg` supports it, pass
   `--expect-abi 3` (or set `ITO_EXPECTED_ABI=3`) so a build that links an older SDK fails instead
   of publishing an ABI mismatch.

## Field rules

- **`key`** identifies one exact upload within its entry. It is used to fetch pages and to remember
  reading progress and downloads, so it must stay stable across refreshes and must not depend on the
  scanlator's name, the user's settings or the chapter's position in the list. It is not a content
  hash and need not be unique across entries. Two chapters in one list never share a key.
- **`scanlator.id`** is a stable identifier of the group within your source. Set it only when the
  site provides one; never derive it from the display name. `name` is what users see and may change
  without changing the upload's `key`.
- **`volume` and `chapter`** are for display and ordering. Equal numbers do not mean two uploads are
  the same reading position, and no number means nothing about identity.
- **`logical_unit_id`** is the identifier shared by every upload the *site* declares to be the same
  reading position. It is scoped to the entry, stays stable across refreshes, and is never reused for
  a different position. Leave it `None` unless the site says so; the same number, title, volume,
  group or date is not enough.
- **`reading_order`**: lower values are read earlier; values are comparable only within one
  response; gaps are allowed; equal values mean the same reading position (for example alternative
  uploads of one chapter). It orders reading positions regardless of the order of the list, so a
  newest-first list can carry ascending values. It is not durable across refreshes.
- **`revision`** is a token that changes exactly when the upload's content changes. Leave it `None`
  unless the site provides one tied to the content, and never send an empty one. A modification time
  is not a revision.
- **`paywalled`** means locked for the signed-in account or session; `None` means unknown.
- **`catalog`** is `None` when you do not know what your list covers. It must be `None` when
  `chapters` is `None`: set it whenever you set `chapters`. The entry you receive in
  `get_media_update` never carries a catalog (the SDK clears the host's saved one before calling
  you). Claim `Complete` only when the site gave you everything within the scope, and never claim
  `AllEligible` for a list that hides alternative uploads.

## Validation

The export macros check every `Manga`, `Novel` and chapter a plugin returns from `get_home`,
`get_listing`, `search`, `get_media_update` and `handle_url`, and hosts apply the same rules to what
they decode. A violation fails the export with `Error::Contract { export, code }`. Hosts read it
from `ito_last_error` as category `ContractViolation` (8, the zigzag varint 16 on the wire) whose
message is the fixed rule code, such as `duplicate.key`; it never contains plugin text. Hosts that
predate the category cannot decode it, so only ABI 3 hosts load these plugins. The first violation
in the declared order wins: catalog without chapters, chapter count, each chapter in order (key,
title, scanlator name, scanlator id, logical unit id, revision, url, lang, volume, chapter, date),
duplicate keys, then scanlator conflicts.

Strings compare as exact UTF-8 bytes, with no normalization: `é` written as one code point and as
`e` plus a combining accent are different keys and different names. Every length is in UTF-8 bytes.

| Limit | Value |
|---|---|
| Chapters per entry | at most 20 000 |
| `key` | 1 to 512 bytes |
| `title` | at most 1 024 bytes |
| `scanlator.name` | 1 to 256 bytes |
| `scanlator.id`, `logical_unit_id` | 1 to 256 bytes when present |
| `revision` | 1 to 256 bytes when present |
| `url` | at most 4 096 bytes |
| `lang` | at most 35 bytes |
| `volume`, `chapter` | finite, magnitude at most 1e9 (finite as a `Float`, safe to convert to an `Int`) |
| `date_updated` | seconds since 1970-01-01 UTC, finite, magnitude at most 1e11 |
| `get_media_update` and `handle_url` response | at most 16 MiB encoded |

Rules beyond sizes:

- keys are unique within one `chapters` list;
- one `scanlator.id` never carries two different `name`s in one list;
- `catalog` requires `chapters`.

`ito_rs::catalog` exposes the limits as constants and the checks as `validate_chapter`,
`validate_chapters`, `validate_entry` and friends, so a plugin's own tests can assert them against
fixture responses.

## Wire layout

`fixtures/catalog-v3.json` pins the byte layout and the validation verdicts of the cases above, and
the `ito-rs` and host test suites both assert it. Encodings are Postcard; enum discriminants
(`CatalogScope`, `CatalogCompleteness`, `ContentKind`) travel as zigzag `i32` varints.

Evolution rule: a `CatalogScope` or `CatalogCompleteness` discriminant a reader does not know
decodes to `Unknown` on both sides, so later releases can add values without a new ABI. An unknown
`ContentKind` is rejected.
