# Featured home components

A featured component is a short hero carousel above a source's shelves. Each page shows one
title with artwork, an optional badge, a few structured chips and a tagline. It is the richer
form of the `*BigScroller` components, which show only a cover.

Plugins write one `get_home` as before and return a `HomeComponentValue::Featured`. The SDK
exports two functions from that one method, so a plugin never writes a second home:

- `get_home()` is the legacy export. It lowers every featured component to the matching
  BigScroller before the contract check, so hosts without the feature see the cover carousel they
  already know.
- `get_home_v2(features: i32)` is the optional export a host calls when it understands featured
  components. It passes the host's feature bits to the plugin and lowers only what the host lacks.
  Every plugin built with this SDK exports it.

The ABI stays 3. The new value is postcard variant 17 of `HomeComponentValue`, and the change adds
no host import and no `min_app_version` bump.

## Writing a featured component

```rust,ignore
use ito_rs::{
    Result,
    models::{
        HomeComponent, HomeComponentValue, HomeLayout,
        featured::{BadgeKind, Featured, FeaturedBadge, FeaturedChip, FeaturedItem},
    },
};

fn get_home() -> Result<HomeLayout> {
    let items = spotlights()?
        .into_iter()
        .map(|spotlight| {
            FeaturedItem::new(spotlight.entry)
                .banner(spotlight.backdrop_url)
                .tagline(spotlight.tagline)
                .badge(FeaturedBadge::new(BadgeKind::Trending).rank(spotlight.rank))
                .chip(FeaturedChip::year(spotlight.year))
                .chip(FeaturedChip::rating(spotlight.score, 5.0))
        })
        .collect();
    Ok(HomeLayout {
        components: vec![HomeComponent {
            title: Some("Featured".into()),
            subtitle: None,
            value: HomeComponentValue::Featured(Featured::new(items).auto_advance(8.0)),
        }],
    })
}
```

- `Featured::new(items)` takes the items in display order. `auto_advance` is optional.
- `FeaturedItem::new(entry)` takes the title the user opens. The entry is passed on unchanged: a
  banner is never written into its cover, and the tapped entry is the one that reaches details
  and the library.
- Every builder replaces the field it sets. Chips append, up to six.
- `FeaturedChip::rating(value, out_of)` normalizes to a 0 to 10 scale and returns `None` when the
  value is not on its scale. `.chip` accepts the `Option` directly.
- A custom badge needs a label; its label is shown verbatim, so it is never translated.

### Dropped input

The builders drop decorative input that breaks a limit, so one scraped image never fails the
whole home:

| Field | Dropped when |
|---|---|
| `banner`, `logo` | not `http`/`https`, empty, or longer than 4 096 bytes |
| `tagline` | empty or longer than 256 bytes |
| `summary` | empty or longer than 2 048 bytes |
| badge `rank` | outside 1 to 999 (the badge stays) |
| badge `label` | empty or longer than 64 bytes (the badge stays, unless it is a custom badge) |
| custom badge | has no valid label |
| chip `text` | empty or longer than 64 bytes |
| chip `number` | outside its kind's range (see below) |
| chip | left with neither text nor number, or beyond six per item |
| `accent` | above `0xFFFFFF` |

Everything else is checked strictly when the component is returned, so a mistake in the items or
the component itself fails the export and is reported to the host.

## What older hosts see

Hosts built before this feature call `get_home`, and they receive the lowered layout:

- A featured component becomes `BigScroller` when every entry is a manga, `AnimeBigScroller` for
  anime, `NovelBigScroller` for novels, and `MediaBigScroller` for a mix. Its `auto_advance` is
  kept.
- Its title and subtitle stay. Its banner, logo, tagline, summary, badges, chips, accent, action
  and listing are dropped.
- A featured component with no items is dropped. Other components are unchanged.
- The lowered layout is what the export checks. A decoration that is dropped never fails a legacy
  host, even if it would fail a featured host.

A host that calls `get_home_v2` with `HomeFeatures::FEATURED` receives the layout unchanged.
Unknown feature bits are ignored.

## Hosts and the optional method

A plugin can override `get_home_with(features)` to skip work the host cannot show, such as
fetching detail pages for chips. The default calls `get_home`:

```rust,ignore
fn get_home_with(features: HomeFeatures) -> Result<HomeLayout> {
    if features.contains(HomeFeatures::FEATURED) {
        featured_home()
    } else {
        plain_home()
    }
}
```

## Limits

Lengths are UTF-8 bytes. Validation requires each value below and reports the first violation in
the order listed in [`fixtures/home-v2.json`](../fixtures/home-v2.json).

| Field | Limit | Rule code |
|---|---|---|
| items per component | 1 to 20 | `count.featured` |
| `auto_advance` | finite, 2 to 60 seconds | `number.range` |
| banner, logo | 1 to 4 096 bytes | `length.url` |
| banner, logo | `http` or `https` scheme, any letter case | `url.scheme` |
| tagline | 1 to 256 bytes | `length.text` |
| summary | 1 to 2 048 bytes | `length.text` |
| badge label | 1 to 64 bytes | `length.text` |
| badge rank | 1 to 999 | `number.range` |
| chips per item | at most 6 | `count.chips` |
| chip text | 1 to 64 bytes | `length.text` |
| chip year | integer, 1800 to 2200 | `number.range` |
| chip rating | 0 to 10 | `number.range` |
| chip units | integer, 0 to 1 000 000 | `number.range` |
| accent | at most `0xFFFFFF` | `color.range` |
| items | unique by (kind, key) | `duplicate.key` |

Entries keep the existing entry rules, and an item's entry is checked first. Within one item the
order is: entry, banner and logo lengths, banner and logo schemes, tagline, summary, badge label,
badge rank, chip count, each chip's text and then number, accent. Duplicates are checked after
every item of the component.

The entry kind of every item must be one the source serves. Hosts check that; the SDK's
validation does not know which kinds a host has been asked for, so a plugin's own tests should
cover it.

## Evolution

A later release that needs more fields adds a new variant behind a new feature bit. The export
lowers that variant for hosts without the bit, the same way it lowers `Featured` today. Existing
wire types are never reordered.
