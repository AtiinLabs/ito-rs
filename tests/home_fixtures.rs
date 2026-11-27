//! Pins the featured home wire layout, its downgrade and its first-violation order to
//! `fixtures/home-v2.json`. The bytes are written independently of the SDK's encoder: the SDK must
//! decode each one, encode it back unchanged and give the fixture's verdict. The export glue's
//! byte-level packing is checked in the unit tests of `src/export.rs`; the exports generated here
//! are checked by length, because a native packed pointer keeps only its low 32 bits.

use std::sync::Mutex;

use ito_rs::{
    Result,
    catalog::{self, Rule, Violation},
    models::{
        Chapter, HomeComponent, HomeComponentValue, HomeLayout, Listing, MediaEntry, MediaKind,
        MediaPageResult, Page,
        featured::{
            BadgeKind, Featured, FeaturedAction, FeaturedBadge, FeaturedChip, FeaturedItem,
            HomeFeatures,
        },
        manga::{self, Manga},
    },
    provider::SourceProvider,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("../fixtures/home-v2.json")).expect("valid home fixture JSON")
}

fn bytes(hex_text: &Value) -> Vec<u8> {
    hex::decode(hex_text.as_str().expect("hex string")).expect("valid hex")
}

fn features(bits: &Value) -> HomeFeatures {
    HomeFeatures::from_bits_retain(bits.as_u64().expect("feature bits") as u32)
}

fn outcome(result: std::result::Result<(), Violation>) -> String {
    match result {
        Ok(()) => "ok".into(),
        Err(violation) => format!("violation:{}", violation.rule.code()),
    }
}

/// Decodes `T` from exactly `wire`, then checks the SDK encodes it back to the same bytes.
fn decode_exact<T: Serialize + DeserializeOwned>(wire: &[u8], name: &str) -> T {
    let (value, rest): (T, _) = ito_rs::postcard::take_from_bytes(wire)
        .unwrap_or_else(|error| panic!("{name} decodes: {error}"));
    assert!(rest.is_empty(), "trailing fixture bytes {name}");
    assert_eq!(
        ito_rs::postcard::to_allocvec(&value).unwrap(),
        wire,
        "re-encoded fixture {name}"
    );
    value
}

#[test]
fn every_wire_case_round_trips_and_has_its_verdict() {
    let fixture = fixture();
    let cases = fixture["wire"].as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let wire = bytes(&case["hex"]);
        let expected = case["expect"].as_str().unwrap();
        match case["type"].as_str().unwrap() {
            "HomeComponentValue" => {
                let value: HomeComponentValue = decode_exact(&wire, name);
                assert_eq!(wire[0], case["variant"].as_u64().unwrap() as u8, "{name}");
                let layout = HomeLayout {
                    components: vec![HomeComponent {
                        title: None,
                        subtitle: None,
                        value,
                    }],
                };
                assert_eq!(outcome(catalog::validate_home(&layout)), expected, "{name}");
            }
            "HomeLayout" => {
                let layout: HomeLayout = decode_exact(&wire, name);
                assert_eq!(outcome(catalog::validate_home(&layout)), expected, "{name}");
            }
            other => panic!("unhandled fixture type {other}"),
        }
    }
}

#[test]
fn every_downgrade_case_lowers_to_its_pinned_bytes_with_its_verdict() {
    let fixture = fixture();
    for case in fixture["downgrade"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let input: HomeLayout = decode_exact(&bytes(&case["input"]), name);
        let output_wire = bytes(&case["output"]);
        let output: HomeLayout = decode_exact(&output_wire, name);

        let lowered = input.downgraded(features(&case["features"]));
        assert_eq!(
            ito_rs::postcard::to_allocvec(&lowered).unwrap(),
            output_wire,
            "lowered bytes {name}"
        );
        assert_eq!(
            outcome(catalog::validate_home(&output)),
            case["expect"].as_str().unwrap(),
            "{name}"
        );
    }
}

#[test]
fn the_full_example_is_built_with_the_public_builders() {
    let entry = MediaEntry::Manga(Manga {
        key: "m1".into(),
        title: "M".into(),
        authors: None,
        artist: None,
        description: None,
        tags: None,
        cover: Some("https://example.test/m1.jpg".into()),
        url: None,
        status: manga::Status::Unknown,
        content_rating: manga::ContentRating::Safe,
        nsfw: 0,
        viewer: manga::Viewer::Default,
        chapters: None,
        catalog: None,
    });
    let item = FeaturedItem::new(entry)
        .banner("https://example.test/banner.jpg")
        .logo("https://example.test/logo.png")
        .tagline("The legend returns")
        .summary("A short summary.")
        .badge(FeaturedBadge::new(BadgeKind::Trending).rank(1))
        .chip(FeaturedChip::year(2024))
        .chip(FeaturedChip::rating(4.0, 5.0))
        .chip(FeaturedChip::units(12))
        .chip(FeaturedChip::genre("Action"))
        .chip(FeaturedChip::format("Manhwa"))
        .accent(0xFF8800)
        .action(FeaturedAction::Start);
    let featured = Featured::new(vec![item])
        .auto_advance(8.0)
        .listing(Listing {
            id: "l".into(),
            name: "All".into(),
            kind: 0,
        });
    let layout = HomeLayout {
        components: vec![HomeComponent {
            title: Some("Featured".into()),
            subtitle: None,
            value: HomeComponentValue::Featured(featured),
        }],
    };

    let expected = fixture()["wire"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "home.featured.full")
        .map(|case| bytes(&case["hex"]))
        .expect("the full example is pinned");
    assert_eq!(ito_rs::postcard::to_allocvec(&layout).unwrap(), expected);
}

#[test]
fn every_new_code_is_listed_and_pinned_by_a_case() {
    let fixture = fixture();
    let listed: Vec<&str> = fixture["codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|code| code.as_str().unwrap())
        .collect();
    let pinned: Vec<&str> = fixture["wire"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|case| case["expect"].as_str()?.strip_prefix("violation:"))
        .collect();
    assert_eq!(listed.len(), 5);
    for code in &listed {
        assert!(
            Rule::ALL.iter().any(|rule| rule.code() == *code),
            "{code} is a rule"
        );
        assert!(pinned.contains(code), "no fixture case for {code}");
    }
}

#[test]
fn the_limits_constants_equal_the_shared_limits() {
    let fixture = fixture();
    let limits = &fixture["limits"];
    let size = |key: &str| limits[key].as_u64().unwrap() as usize;
    let number = |key: &str| limits[key].as_f64().unwrap();
    assert_eq!(size("max_featured_items"), catalog::MAX_FEATURED_ITEMS);
    assert_eq!(size("max_featured_chips"), catalog::MAX_FEATURED_CHIPS);
    assert_eq!(size("max_tagline_bytes"), catalog::MAX_TAGLINE_BYTES);
    assert_eq!(size("max_summary_bytes"), catalog::MAX_SUMMARY_BYTES);
    assert_eq!(
        size("max_badge_label_bytes"),
        catalog::MAX_BADGE_LABEL_BYTES
    );
    assert_eq!(size("max_chip_text_bytes"), catalog::MAX_CHIP_TEXT_BYTES);
    assert_eq!(size("max_url_bytes"), catalog::MAX_URL_BYTES);
    assert_eq!(size("max_accent"), catalog::MAX_ACCENT as usize);
    assert_eq!(
        limits["max_badge_rank"].as_i64().unwrap(),
        i64::from(catalog::MAX_BADGE_RANK)
    );
    assert_eq!(
        number("min_auto_advance"),
        f64::from(catalog::MIN_AUTO_ADVANCE)
    );
    assert_eq!(
        number("max_auto_advance"),
        f64::from(catalog::MAX_AUTO_ADVANCE)
    );
    assert_eq!(number("min_chip_year"), catalog::MIN_CHIP_YEAR);
    assert_eq!(number("max_chip_year"), catalog::MAX_CHIP_YEAR);
    assert_eq!(number("max_chip_rating"), catalog::MAX_CHIP_RATING);
    assert_eq!(number("max_chip_units"), catalog::MAX_CHIP_UNITS);
    assert_eq!(fixture["version"], u64::from(ito_rs::ABI_VERSION));
    assert_eq!(fixture["contract"], "ito-home");
}

struct Provider;

/// The downgrade case the exports under test return, selected by name. The export glue calls
/// `get_home` and `get_home_with` in this one thread, so a single test sets it.
static CASE: Mutex<String> = Mutex::new(String::new());

impl SourceProvider for Provider {
    const KINDS: &'static [MediaKind] = &[MediaKind::Manga];

    fn get_home() -> Result<HomeLayout> {
        let name = CASE.lock().expect("case lock").clone();
        let fixture = fixture();
        let case = fixture["downgrade"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["name"] == name.as_str())
            .expect("a fixture case is selected");
        Ok(ito_rs::postcard::from_bytes(&bytes(&case["input"])).expect("input decodes"))
    }

    fn get_listing(_: Listing, _: i32) -> Result<MediaPageResult> {
        Ok(MediaPageResult {
            entries: Vec::new(),
            has_next_page: false,
        })
    }

    fn search(
        _: &str,
        _: i32,
        _: Option<MediaKind>,
        _: Vec<ito_rs::models::FilterItem>,
    ) -> Result<MediaPageResult> {
        Ok(MediaPageResult {
            entries: Vec::new(),
            has_next_page: false,
        })
    }

    fn get_media_update(entry: MediaEntry, _: bool, _: bool) -> Result<MediaEntry> {
        Ok(entry)
    }

    fn get_chapter_pages(_: MediaEntry, _: Chapter) -> Result<Vec<Page>> {
        Ok(Vec::new())
    }
}

ito_rs::export_plugin!(Provider);

/// The export's packed length for a success, or 0 for a violation. Natively the pointer half is
/// not a usable address, so a success block is leaked here rather than freed.
fn packed_length(packed: i64) -> Option<usize> {
    (packed != 0).then_some((packed as u64 & 0xFFFF_FFFF) as usize)
}

#[test]
fn the_exports_lower_the_home_for_each_host() {
    let fixture = fixture();
    let cases = fixture["downgrade"].as_array().unwrap();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        *CASE.lock().unwrap() = name.to_owned();
        let output_len = bytes(&case["output"]).len();
        let verdict = case["expect"].as_str().unwrap();
        let bits = case["features"].as_u64().unwrap() as u32;
        let packed = get_home_v2(bits as i32);
        assert_eq!(
            packed_length(packed),
            (!verdict.starts_with("violation:")).then_some(output_len),
            "get_home_v2 {name}"
        );
        if bits == 0 {
            assert_eq!(
                packed_length(get_home()),
                (!verdict.starts_with("violation:")).then_some(output_len),
                "get_home {name}"
            );
        }
    }

    // The legacy export lowers its layout for hosts without the feature, whatever the plugin's
    // default, and an all-bits host keeps the featured layout.
    *CASE.lock().unwrap() = "downgrade.kept_for_featured_host".to_owned();
    let lowered = cases
        .iter()
        .find(|case| case["name"] == "downgrade.manga_only")
        .expect("lowered case");
    assert_eq!(
        packed_length(get_home()),
        Some(bytes(&lowered["output"]).len()),
        "legacy get_home lowers the featured input"
    );
    let kept = cases
        .iter()
        .find(|case| case["name"] == "downgrade.kept_for_featured_host")
        .expect("kept case");
    assert_eq!(
        packed_length(get_home_v2(-1)),
        Some(bytes(&kept["output"]).len()),
        "all feature bits keep the featured layout"
    );
}
