//! Featured home components: a short carousel of artwork-led items, with badges and chips, that a
//! host shows above its shelves. Plugins return it from `get_home` like any other component.
//!
//! The export glue lowers a featured component to the BigScroller of its entries for a host that
//! lacks [`HomeFeatures::FEATURED`], so the decorations never reach hosts that cannot show them.
//! The builders drop decorative input that breaks a limit, so a scraped bad image does not fail
//! the whole home; everything else is checked strictly by [`crate::catalog`]. See
//! `docs/HOME-FEATURED.md`.

use super::{
    HomeComponent, HomeComponentValue, HomeLayout, Listing, MediaEntry, MediaKind, anime::Anime,
    manga::Manga, novel::Novel,
};
use crate::catalog::{
    MAX_ACCENT, MAX_BADGE_LABEL_BYTES, MAX_BADGE_RANK, MAX_CHIP_RATING, MAX_CHIP_TEXT_BYTES,
    MAX_CHIP_UNITS, MAX_CHIP_YEAR, MAX_FEATURED_CHIPS, MAX_SUMMARY_BYTES, MAX_TAGLINE_BYTES,
    MAX_URL_BYTES, MIN_CHIP_YEAR, is_web_url,
};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// A featured home component. Its wire layout is its fields in declaration order.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[non_exhaustive]
pub struct Featured {
    pub items: Vec<FeaturedItem>,
    /// Seconds between pages, within the limits in [`crate::catalog`]. `None` means the user pages.
    pub auto_advance: Option<f32>,
    /// The target of the component's "See all" link.
    pub listing: Option<Listing>,
}

impl Featured {
    /// A component of `items`, shown in the order given. Validation requires 1 to 20 items.
    pub fn new(items: Vec<FeaturedItem>) -> Self {
        Featured {
            items,
            auto_advance: None,
            listing: None,
        }
    }

    /// Advances the carousel every `seconds`. Validation accepts 2 to 60.
    pub fn auto_advance(mut self, seconds: f32) -> Self {
        self.auto_advance = Some(seconds);
        self
    }

    pub fn listing(mut self, listing: Listing) -> Self {
        self.listing = Some(listing);
        self
    }
}

/// One page of a featured component. `entry` is the title the user opens, and it is passed on
/// unchanged; every other field only decorates the page.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[non_exhaustive]
pub struct FeaturedItem {
    pub entry: MediaEntry,
    /// A wide backdrop, about 16:9, distinct from the entry's 2:3 cover.
    pub banner: Option<String>,
    /// Transparent title art.
    pub logo: Option<String>,
    pub tagline: Option<String>,
    /// A short display text. Hosts show `entry.description` when it is absent.
    pub summary: Option<String>,
    pub badge: Option<FeaturedBadge>,
    /// Structured metadata, at most [`MAX_FEATURED_CHIPS`]. Status is not a chip: hosts read
    /// `entry.status`.
    pub chips: Vec<FeaturedChip>,
    /// `0xRRGGBB`, a tint hint only.
    pub accent: Option<u32>,
    pub action: FeaturedAction,
}

impl FeaturedItem {
    pub fn new(entry: MediaEntry) -> Self {
        FeaturedItem {
            entry,
            banner: None,
            logo: None,
            tagline: None,
            summary: None,
            badge: None,
            chips: Vec::new(),
            accent: None,
            action: FeaturedAction::Details,
        }
    }

    /// Sets the backdrop. A URL that is not `http` or `https`, or is outside the limit, is dropped.
    pub fn banner(mut self, url: impl Into<String>) -> Self {
        self.banner = web_url(url.into());
        self
    }

    /// Sets the title art. A URL that is not `http` or `https`, or is outside the limit, is dropped.
    pub fn logo(mut self, url: impl Into<String>) -> Self {
        self.logo = web_url(url.into());
        self
    }

    /// Sets the one-line tagline. Text outside 1 to 256 bytes is dropped.
    pub fn tagline(mut self, text: impl Into<String>) -> Self {
        self.tagline = text_within(text.into(), MAX_TAGLINE_BYTES);
        self
    }

    /// Sets the display summary. Text outside 1 to 2048 bytes is dropped.
    pub fn summary(mut self, text: impl Into<String>) -> Self {
        self.summary = text_within(text.into(), MAX_SUMMARY_BYTES);
        self
    }

    /// Sets the badge. Invalid parts are dropped with them: see [`FeaturedBadge`].
    pub fn badge(mut self, badge: FeaturedBadge) -> Self {
        self.badge = badge.sanitized();
        self
    }

    /// Appends a chip. `None`, an invalid chip, or a chip beyond [`MAX_FEATURED_CHIPS`] is dropped,
    /// so `rating` can be passed directly.
    pub fn chip(mut self, chip: impl Into<Option<FeaturedChip>>) -> Self {
        let chip = chip.into().and_then(FeaturedChip::sanitized);
        if self.chips.len() < MAX_FEATURED_CHIPS {
            self.chips.extend(chip);
        }
        self
    }

    /// Sets the tint, `0xRRGGBB`. A colour above `0xFFFFFF` is dropped.
    pub fn accent(mut self, rgb: u32) -> Self {
        self.accent = (rgb <= MAX_ACCENT).then_some(rgb);
        self
    }

    pub fn action(mut self, action: FeaturedAction) -> Self {
        self.action = action;
        self
    }
}

/// The action a page suggests. Hosts choose "Continue" themselves from their own progress.
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(i32)]
#[non_exhaustive]
pub enum FeaturedAction {
    #[default]
    Details = 0,
    Start = 1,
}

/// A badge over an item. The host localizes the known kinds; `label` is plugin text and is
/// shown verbatim.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[non_exhaustive]
pub struct FeaturedBadge {
    pub kind: BadgeKind,
    /// Shown as "Trending #n" with [`BadgeKind::Trending`].
    pub rank: Option<i32>,
    pub label: Option<String>,
}

impl FeaturedBadge {
    pub fn new(kind: BadgeKind) -> Self {
        FeaturedBadge {
            kind,
            rank: None,
            label: None,
        }
    }

    /// Sets the rank. A rank outside 1 to 999 is dropped when the badge is set on an item.
    pub fn rank(mut self, rank: i32) -> Self {
        self.rank = Some(rank);
        self
    }

    /// Sets the label. Text outside 1 to 64 bytes is dropped when the badge is set on an item.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Drops an invalid rank or label, and a custom badge left without a label, which would show
    /// nothing. `None` means the badge is dropped.
    fn sanitized(mut self) -> Option<Self> {
        self.rank = self.rank.filter(|rank| (1..=MAX_BADGE_RANK).contains(rank));
        self.label = self
            .label
            .and_then(|label| text_within(label, MAX_BADGE_LABEL_BYTES));
        (self.kind != BadgeKind::Custom || self.label.is_some()).then_some(self)
    }
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
#[non_exhaustive]
pub enum BadgeKind {
    /// Needs a label.
    Custom = 0,
    New = 1,
    /// Uses the rank.
    Trending = 2,
    Popular = 3,
    Updated = 4,
}

/// A structured fact about an item. Use the constructors, which fill the field each kind needs.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[non_exhaustive]
pub struct FeaturedChip {
    pub kind: ChipKind,
    pub number: Option<f64>,
    pub text: Option<String>,
}

impl FeaturedChip {
    /// The release year.
    pub fn year(year: i32) -> Self {
        Self::with_number(ChipKind::Year, f64::from(year))
    }

    /// A rating, normalized to a 0 to 10 scale from `value` out of `out_of`, such as 4 of 5 or
    /// 80 of 100. `None` when `out_of` is not positive or `value` is not within `0..=out_of`.
    pub fn rating(value: f64, out_of: f64) -> Option<Self> {
        (out_of.is_finite() && out_of > 0.0 && (0.0..=out_of).contains(&value))
            .then(|| Self::with_number(ChipKind::Rating, value / out_of * MAX_CHIP_RATING))
    }

    /// A count of units. The host words it by entry kind: chapters, episodes or pages.
    pub fn units(count: u32) -> Self {
        Self::with_number(ChipKind::Units, f64::from(count))
    }

    pub fn genre(name: impl Into<String>) -> Self {
        Self::with_text(ChipKind::Genre, name)
    }

    pub fn format(name: impl Into<String>) -> Self {
        Self::with_text(ChipKind::Format, name)
    }

    pub fn text(text: impl Into<String>) -> Self {
        Self::with_text(ChipKind::Text, text)
    }

    fn with_number(kind: ChipKind, number: f64) -> Self {
        FeaturedChip {
            kind,
            number: Some(number),
            text: None,
        }
    }

    fn with_text(kind: ChipKind, text: impl Into<String>) -> Self {
        FeaturedChip {
            kind,
            number: None,
            text: Some(text.into()),
        }
    }

    /// Drops a number or text outside its limits, and drops the chip when nothing is left.
    fn sanitized(mut self) -> Option<Self> {
        let kind = self.kind;
        self.number = self.number.filter(|number| kind.accepts_number(*number));
        self.text = self
            .text
            .and_then(|text| text_within(text, MAX_CHIP_TEXT_BYTES));
        (self.number.is_some() || self.text.is_some()).then_some(self)
    }
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
#[non_exhaustive]
pub enum ChipKind {
    Text = 0,
    /// The number is the year.
    Year = 1,
    /// The number is the rating on a 0 to 10 scale.
    Rating = 2,
    /// The number is a count the host words by entry kind.
    Units = 3,
    Genre = 4,
    Format = 5,
}

impl ChipKind {
    /// Whether `number` is a value this kind can show. Kinds without a numeric meaning accept any.
    pub(crate) fn accepts_number(self, number: f64) -> bool {
        match self {
            ChipKind::Year => {
                number.fract() == 0.0 && (MIN_CHIP_YEAR..=MAX_CHIP_YEAR).contains(&number)
            }
            ChipKind::Rating => (0.0..=MAX_CHIP_RATING).contains(&number),
            ChipKind::Units => number.fract() == 0.0 && (0.0..=MAX_CHIP_UNITS).contains(&number),
            ChipKind::Text | ChipKind::Genre | ChipKind::Format => true,
        }
    }
}

/// The home features a host supports, as bits. A host passes them to the optional `get_home_v2`
/// export; a plugin ignores bits it does not know.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HomeFeatures(u32);

impl HomeFeatures {
    pub const NONE: Self = Self(0);
    /// The host shows [`Featured`] components.
    pub const FEATURED: Self = Self(1 << 0);

    pub const fn from_bits_retain(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn contains(self, feature: Self) -> bool {
        self.0 & feature.0 == feature.0
    }
}

impl HomeLayout {
    /// The layout a host with `host` features can decode. A featured component the host cannot
    /// show becomes the BigScroller of its entries: `BigScroller`, `AnimeBigScroller` or
    /// `NovelBigScroller` when every entry is one kind, otherwise `MediaBigScroller`. Its title and
    /// subtitle stay, and its banner, logo, tagline, summary, badges, chips and listing are dropped.
    /// A featured component without items is dropped. Other components are unchanged.
    pub fn downgraded(self, host: HomeFeatures) -> Self {
        let components = self
            .components
            .into_iter()
            .filter_map(|component| {
                let value = match component.value {
                    HomeComponentValue::Featured(featured)
                        if !host.contains(HomeFeatures::FEATURED) =>
                    {
                        downgrade_featured(featured)?
                    }
                    value => value,
                };
                Some(HomeComponent { value, ..component })
            })
            .collect();
        HomeLayout { components }
    }
}

fn downgrade_featured(featured: Featured) -> Option<HomeComponentValue> {
    let auto_advance = featured.auto_advance;
    let entries: Vec<MediaEntry> = featured.items.into_iter().map(|item| item.entry).collect();
    let kind = entries.first()?.kind();
    if entries.iter().any(|entry| entry.kind() != kind) {
        return Some(HomeComponentValue::MediaBigScroller(entries, auto_advance));
    }
    // Every entry has `kind` here, so the filters below keep all of them.
    Some(match kind {
        MediaKind::Manga => HomeComponentValue::BigScroller(
            entries
                .into_iter()
                .filter_map(|entry| match entry {
                    MediaEntry::Manga(manga) => Some(manga),
                    _ => None,
                })
                .collect::<Vec<Manga>>(),
            auto_advance,
        ),
        MediaKind::Anime => HomeComponentValue::AnimeBigScroller(
            entries
                .into_iter()
                .filter_map(|entry| match entry {
                    MediaEntry::Anime(anime) => Some(anime),
                    _ => None,
                })
                .collect::<Vec<Anime>>(),
            auto_advance,
        ),
        MediaKind::Novel => HomeComponentValue::NovelBigScroller(
            entries
                .into_iter()
                .filter_map(|entry| match entry {
                    MediaEntry::Novel(novel) => Some(novel),
                    _ => None,
                })
                .collect::<Vec<Novel>>(),
            auto_advance,
        ),
    })
}

/// Keeps `text` when it is 1 to `max` bytes, the bounds the validator checks.
fn text_within(text: String, max: usize) -> Option<String> {
    (1..=max).contains(&text.len()).then_some(text)
}

fn web_url(url: String) -> Option<String> {
    ((1..=MAX_URL_BYTES).contains(&url.len()) && is_web_url(&url)).then_some(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{anime, manga, novel};

    fn manga(key: &str) -> MediaEntry {
        MediaEntry::Manga(manga::Manga {
            key: key.into(),
            title: "M".into(),
            authors: None,
            artist: None,
            description: None,
            tags: None,
            cover: Some("https://example.test/cover.jpg".into()),
            url: None,
            status: manga::Status::Unknown,
            content_rating: manga::ContentRating::Safe,
            nsfw: 0,
            viewer: manga::Viewer::Default,
            chapters: None,
            catalog: None,
        })
    }

    fn item(key: &str) -> FeaturedItem {
        FeaturedItem::new(manga(key))
    }

    #[test]
    fn enum_discriminants_are_the_wire_format() {
        assert_eq!(postcard::to_allocvec(&FeaturedAction::Start).unwrap(), [2]);
        assert_eq!(postcard::to_allocvec(&BadgeKind::Trending).unwrap(), [4]);
        assert_eq!(postcard::to_allocvec(&BadgeKind::Updated).unwrap(), [8]);
        assert_eq!(postcard::to_allocvec(&ChipKind::Text).unwrap(), [0]);
        assert_eq!(postcard::to_allocvec(&ChipKind::Year).unwrap(), [2]);
        assert_eq!(postcard::to_allocvec(&ChipKind::Rating).unwrap(), [4]);
        assert_eq!(postcard::to_allocvec(&ChipKind::Units).unwrap(), [6]);
        assert_eq!(postcard::to_allocvec(&ChipKind::Genre).unwrap(), [8]);
        assert_eq!(postcard::to_allocvec(&ChipKind::Format).unwrap(), [10]);
        assert!(postcard::from_bytes::<ChipKind>(&[12]).is_err());
    }

    #[test]
    fn rating_normalizes_to_ten_and_rejects_values_off_the_scale() {
        let rating = |value, out_of| FeaturedChip::rating(value, out_of).and_then(|c| c.number);
        assert_eq!(rating(4.0, 5.0), Some(8.0));
        assert_eq!(rating(80.0, 100.0), Some(8.0));
        assert_eq!(rating(10.0, 10.0), Some(10.0));
        assert_eq!(rating(0.0, 5.0), Some(0.0));
        assert_eq!(rating(6.0, 5.0), None);
        assert_eq!(rating(-1.0, 5.0), None);
        assert_eq!(rating(f64::NAN, 5.0), None);
        assert_eq!(rating(1.0, 0.0), None);
        assert_eq!(rating(1.0, f64::INFINITY), None);
    }

    #[test]
    fn builders_drop_invalid_decorative_input() {
        let item = item("k")
            .banner("ftp://example.test/banner.jpg")
            .logo("https://example.test/logo.png")
            .tagline("")
            .summary("x".repeat(MAX_SUMMARY_BYTES + 1))
            .accent(MAX_ACCENT + 1)
            .chip(FeaturedChip::year(1799))
            .chip(FeaturedChip::units(1_000_001))
            .chip(FeaturedChip::text(""))
            .chip(FeaturedChip::year(2024));
        assert!(item.banner.is_none());
        assert_eq!(item.logo.as_deref(), Some("https://example.test/logo.png"));
        assert!(item.tagline.is_none());
        assert!(item.summary.is_none());
        assert!(item.accent.is_none());
        assert_eq!(item.chips.len(), 1);
        assert_eq!(item.chips[0].number, Some(2024.0));
    }

    #[test]
    fn builders_keep_valid_input_and_cap_chips() {
        let mut item = item("k")
            .banner("HTTPS://example.test/banner.jpg")
            .accent(MAX_ACCENT)
            .action(FeaturedAction::Start);
        for _ in 0..MAX_FEATURED_CHIPS + 2 {
            item = item.chip(FeaturedChip::genre("Action"));
        }
        assert_eq!(
            item.banner.as_deref(),
            Some("HTTPS://example.test/banner.jpg")
        );
        assert_eq!(item.accent, Some(MAX_ACCENT));
        assert_eq!(item.action, FeaturedAction::Start);
        assert_eq!(item.chips.len(), MAX_FEATURED_CHIPS);
    }

    #[test]
    fn badges_drop_bad_parts_and_an_unlabelled_custom_badge() {
        let badge = |badge: FeaturedBadge| item("k").badge(badge).badge;
        let trending = badge(FeaturedBadge::new(BadgeKind::Trending).rank(0).label(""))
            .expect("a trending badge needs neither rank nor label");
        assert!(trending.rank.is_none());
        assert!(trending.label.is_none());
        assert_eq!(
            badge(FeaturedBadge::new(BadgeKind::Trending).rank(999)).and_then(|b| b.rank),
            Some(999)
        );
        assert!(badge(FeaturedBadge::new(BadgeKind::Custom)).is_none());
        assert!(badge(FeaturedBadge::new(BadgeKind::Custom).label("")).is_none());
        assert_eq!(
            badge(FeaturedBadge::new(BadgeKind::Custom).label("Editor's pick"))
                .and_then(|b| b.label)
                .as_deref(),
            Some("Editor's pick")
        );
    }

    #[test]
    fn downgrade_lowers_each_kind_to_its_big_scroller() {
        let anime = MediaEntry::Anime(anime::Anime {
            key: "a".into(),
            title: "A".into(),
            studios: None,
            description: None,
            tags: None,
            cover: None,
            url: None,
            status: anime::Status::Ongoing,
            content_rating: anime::ContentRating::Safe,
            nsfw: 0,
            episodes: None,
            seasons: None,
        });
        let novel = MediaEntry::Novel(novel::Novel {
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
        });
        let component = |items: Vec<FeaturedItem>| HomeComponent {
            title: Some("Hero".into()),
            subtitle: None,
            value: HomeComponentValue::Featured(Featured::new(items).auto_advance(8.0).listing(
                Listing {
                    id: "l".into(),
                    name: "All".into(),
                    kind: 0,
                },
            )),
        };
        let downgrade = |items| {
            HomeLayout {
                components: vec![component(items)],
            }
            .downgraded(HomeFeatures::NONE)
            .components
            .remove(0)
        };

        let manga_only = downgrade(vec![item("m1"), item("m2")]);
        assert_eq!(manga_only.title.as_deref(), Some("Hero"));
        assert!(matches!(
            &manga_only.value,
            HomeComponentValue::BigScroller(mangas, Some(seconds))
                if mangas.len() == 2 && *seconds == 8.0
        ));

        assert!(matches!(
            downgrade(vec![FeaturedItem::new(anime)]).value,
            HomeComponentValue::AnimeBigScroller(animes, Some(_)) if animes.len() == 1
        ));
        assert!(matches!(
            downgrade(vec![FeaturedItem::new(novel.clone())]).value,
            HomeComponentValue::NovelBigScroller(novels, Some(_)) if novels.len() == 1
        ));
        assert!(matches!(
            downgrade(vec![item("m1"), FeaturedItem::new(novel)]).value,
            HomeComponentValue::MediaBigScroller(entries, Some(_)) if entries.len() == 2
        ));
        assert!(
            HomeLayout {
                components: vec![component(Vec::new())]
            }
            .downgraded(HomeFeatures::NONE)
            .components
            .is_empty()
        );
    }

    #[test]
    fn downgrade_keeps_featured_for_a_host_that_has_the_feature() {
        let layout = HomeLayout {
            components: vec![HomeComponent {
                title: None,
                subtitle: None,
                value: HomeComponentValue::Featured(Featured::new(vec![item("m1")])),
            }],
        };
        let kept = layout.clone().downgraded(HomeFeatures::FEATURED);
        assert!(matches!(
            kept.components[0].value,
            HomeComponentValue::Featured(_)
        ));
        let unknown_bits = layout.downgraded(HomeFeatures::from_bits_retain(0b1110));
        assert!(matches!(
            unknown_bits.components[0].value,
            HomeComponentValue::BigScroller(..)
        ));
    }
}
