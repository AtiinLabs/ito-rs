//! Pins every ABI 3 wire layout and catalog rule to `fixtures/catalog-v3.json`, which hosts
//! assert as well. The fixture bytes are written independently of the SDK's encoder.

use ito_rs::{
    Error,
    catalog::{self, Rule, Violation},
    error::{ErrorCategory, ErrorReport},
    export,
    models::{
        CatalogCompleteness as Completeness, CatalogScope as Scope, Chapter, ChapterCatalog,
        ContentKind, MediaEntry, Page, PageContent, Scanlator,
        anime::{self, Anime, Episode},
        manga::{self, Manga},
        novel::{self, Novel},
    },
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{collections::HashMap, fmt::Debug};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../fixtures/catalog-v3.json"))
        .expect("valid shared fixture JSON")
}

fn wire_bytes(entry: &Value) -> Vec<u8> {
    let part =
        |key: &str| hex::decode(entry.get(key).and_then(Value::as_str).unwrap_or("")).unwrap();
    let mut bytes = part("hex");
    if let Some(count) = entry.get("repeat_count") {
        let unit = part("repeat_hex");
        for _ in 0..count.as_u64().unwrap() {
            bytes.extend_from_slice(&unit);
        }
        bytes.extend_from_slice(&part("suffix_hex"));
    }
    bytes
}

fn chapter(key: &str) -> Chapter {
    Chapter::new(key, ContentKind::Text)
}

fn grouped(key: &str, number: f64, scanlator: Scanlator) -> Chapter {
    Chapter {
        chapter: Some(number),
        scanlator: Some(scanlator),
        ..chapter(key)
    }
}

fn alpha() -> Scanlator {
    Scanlator::with_id("g-1", "Alpha Scans")
}

fn beta() -> Scanlator {
    Scanlator::with_id("g-2", "Beta Scans")
}

fn catalog_of(scope: Scope, completeness: Completeness) -> Option<ChapterCatalog> {
    Some(ChapterCatalog {
        scope,
        completeness,
    })
}

fn manga(chapters: Option<Vec<Chapter>>, catalog: Option<ChapterCatalog>) -> MediaEntry {
    MediaEntry::Manga(Manga {
        key: "m".into(),
        title: "M".into(),
        authors: Some(vec!["A".into()]),
        artist: None,
        description: Some("About".into()),
        tags: Some(vec!["Tag".into()]),
        cover: None,
        url: Some("https://example.test/m".into()),
        status: manga::Status::Ongoing,
        content_rating: manga::ContentRating::Safe,
        nsfw: 0,
        viewer: manga::Viewer::Rtl,
        chapters,
        catalog,
    })
}

fn novel(chapters: Option<Vec<Chapter>>, catalog: Option<ChapterCatalog>) -> MediaEntry {
    MediaEntry::Novel(Novel {
        key: "n".into(),
        title: "N".into(),
        authors: Some(vec!["A".into()]),
        artist: None,
        description: Some("About".into()),
        tags: None,
        cover: None,
        url: None,
        status: novel::Status::Completed,
        content_rating: novel::ContentRating::Suggestive,
        nsfw: 0,
        chapters,
        catalog,
    })
}

fn anime() -> MediaEntry {
    MediaEntry::Anime(Anime {
        key: "a".into(),
        title: "A".into(),
        studios: Some(vec!["Studio".into()]),
        description: None,
        tags: None,
        cover: None,
        url: None,
        status: anime::Status::Ongoing,
        content_rating: anime::ContentRating::Safe,
        nsfw: 0,
        episodes: Some(vec![Episode {
            key: "e1".into(),
            title: Some("Pilot".into()),
            episode: Some(1.5),
            date_updated: Some(1_700_000_000.0),
            url: None,
            lang: Some("ja".into()),
            paywalled: Some(false),
        }]),
        seasons: None,
    })
}

fn page(index: i32, content: PageContent) -> Page {
    Page {
        index,
        content,
        has_description: false,
        description: None,
        headers: None,
    }
}

fn url(value: &str) -> PageContent {
    PageContent::Url(value.into())
}

/// One chapter of a one-chapter manga, for the cases that vary a single chapter field.
fn with(edit: impl FnOnce(&mut Chapter)) -> MediaEntry {
    let mut chapter = chapter("k");
    edit(&mut chapter);
    manga(Some(vec![chapter]), None)
}

enum Built {
    Chapter(Chapter),
    Entry(MediaEntry),
    Pages(Vec<Page>),
    Report(ErrorReport),
}

fn build(name: &str) -> Built {
    use Completeness::{Complete, Partial};
    use Scope::{AllEligible, Visible};
    let single = Some(vec![grouped("c1", 1.0, alpha())]);
    let long = |character: char, count: usize| character.to_string().repeat(count);
    let entry = match name {
        "chapter.minimal" => return Built::Chapter(chapter("k")),
        "chapter.scanlator_name_only" => {
            return Built::Chapter(Chapter {
                scanlator: Some(Scanlator::named("Alpha Scans")),
                ..chapter("k")
            });
        }
        "chapter.scanlator_with_id" => {
            return Built::Chapter(Chapter {
                scanlator: Some(alpha()),
                ..chapter("k")
            });
        }
        "chapter.full" => {
            return Built::Chapter(Chapter {
                title: Some("Chapter 12.5: Extra".into()),
                volume: Some(3.0),
                chapter: Some(12.5),
                date_updated: Some(1_700_000_000.5),
                scanlator: Some(alpha()),
                url: Some("https://example.test/c/12-5".into()),
                lang: Some("en".into()),
                paywalled: Some(true),
                logical_unit_id: Some("lu-12.5".into()),
                reading_order: Some(-7),
                revision: Some("r3".into()),
                ..Chapter::new("c-12.5", ContentKind::Images)
            });
        }
        "entry.manga.minimal" => manga(None, None),
        "entry.novel.minimal" => novel(None, None),
        "entry.anime.with_episode" => anime(),
        "entry.manga.catalog_absent" => manga(single, None),
        "entry.manga.catalog_visible_partial" => manga(single, catalog_of(Visible, Partial)),
        "entry.manga.catalog_all_eligible_complete" => {
            manga(single, catalog_of(AllEligible, Complete))
        }
        "entry.manga.catalog_unknown" => {
            manga(single, catalog_of(Scope::Unknown, Completeness::Unknown))
        }
        "entry.novel.catalog_all_eligible_complete" => novel(
            Some(vec![Chapter {
                title: Some("One".into()),
                logical_unit_id: Some("u1".into()),
                ..grouped("t1", 1.0, alpha())
            }]),
            catalog_of(AllEligible, Complete),
        ),
        "entry.manga.same_number_different_groups" => {
            let english = |chapter: Chapter| Chapter {
                lang: Some("en".into()),
                ..chapter
            };
            manga(
                Some(vec![
                    english(grouped("a-10", 10.0, alpha())),
                    english(grouped("b-10", 10.0, beta())),
                    Chapter {
                        lang: Some("fr".into()),
                        ..grouped("x-10", 10.0, Scanlator::named("Unlisted Group"))
                    },
                ]),
                catalog_of(AllEligible, Complete),
            )
        }
        "entry.manga.same_group_split_bonus" => {
            let part = |key: &str, title: &str, order: i64| Chapter {
                title: Some(title.into()),
                reading_order: Some(order),
                ..grouped(key, 5.0, alpha())
            };
            manga(
                Some(vec![
                    part("a-5-1", "Chapter 5 (part 1)", 10),
                    part("a-5-2", "Chapter 5 (part 2)", 11),
                    Chapter {
                        logical_unit_id: Some("lu-bonus-5".into()),
                        ..part("a-5-bonus", "Chapter 5 bonus", 12)
                    },
                ]),
                catalog_of(Visible, Partial),
            )
        }
        "entry.manga.fractional_and_unnumbered" => {
            let ordered = |key: &str, number: Option<f64>, order: Option<i64>| Chapter {
                chapter: number,
                reading_order: order,
                ..chapter(key)
            };
            manga(
                Some(vec![
                    ordered("f-1", Some(1.0), Some(0)),
                    ordered("f-1-5", Some(1.5), Some(1)),
                    Chapter {
                        title: Some("Oneshot".into()),
                        ..ordered("f-oneshot", None, Some(2))
                    },
                    Chapter {
                        title: Some("Extra".into()),
                        ..ordered("f-extra", None, None)
                    },
                ]),
                None,
            )
        }
        "entry.manga.volume_reset" => {
            let numbered = |key: &str, volume: f64, number: f64| Chapter {
                volume: Some(volume),
                ..grouped(key, number, alpha())
            };
            manga(
                Some(vec![
                    numbered("v1-1", 1.0, 1.0),
                    numbered("v1-2", 1.0, 2.0),
                    numbered("v2-1", 2.0, 1.0),
                ]),
                catalog_of(AllEligible, Complete),
            )
        }
        "entry.manga.f64_collision" => manga(
            Some(vec![
                grouped("n-1", 10.000_000_1, alpha()),
                grouped("n-2", 10.000_000_2, alpha()),
            ]),
            catalog_of(Visible, Complete),
        ),
        "entry.manga.reading_order_present" => {
            let ordered = |key: &str, order| Chapter {
                reading_order: Some(order),
                ..chapter(key)
            };
            manga(Some(vec![ordered("o-1", 1), ordered("o-2", 2)]), None)
        }
        "entry.manga.reading_order_absent" => {
            manga(Some(vec![chapter("o-1"), chapter("o-2")]), None)
        }
        "entry.manga.reading_order_duplicated" => {
            let ordered = |key: &str| Chapter {
                reading_order: Some(1),
                ..chapter(key)
            };
            manga(Some(vec![ordered("o-1"), ordered("o-2")]), None)
        }
        "entry.manga.number_at_limit" => with(|c| {
            c.key = "l-1".into();
            c.volume = Some(-1e9);
            c.chapter = Some(1e9);
        }),
        "pages.image" => {
            return Built::Pages(vec![
                page(0, url("https://example.test/p/0.jpg")),
                page(1, url("https://example.test/p/1.jpg")),
            ]);
        }
        "pages.text" => {
            return Built::Pages(vec![Page {
                has_description: true,
                description: Some("Opening".into()),
                ..page(0, PageContent::Text("Once upon a time.".into()))
            }]);
        }
        "pages.mixed" => {
            return Built::Pages(vec![
                page(0, PageContent::Text("Prologue".into())),
                page(1, url("https://example.test/i/1.png")),
            ]);
        }
        "pages.private_headers" => {
            return Built::Pages(vec![Page {
                headers: Some(HashMap::from([(
                    "Referer".into(),
                    "https://example.test/".into(),
                )])),
                ..page(0, url("https://example.test/p/0.jpg"))
            }]);
        }
        "entry.manga.duplicate_keys" => manga(
            Some(vec![chapter("d-1"), chapter("d-2"), chapter("d-1")]),
            None,
        ),
        "entry.manga.scanlator_id_name_conflict" => manga(
            Some(vec![
                Chapter {
                    scanlator: Some(Scanlator::with_id("g-1", "Alpha Scans")),
                    ..chapter("s-1")
                },
                Chapter {
                    scanlator: Some(Scanlator::with_id("g-1", "Alpha Renamed")),
                    ..chapter("s-2")
                },
            ]),
            None,
        ),
        "entry.manga.catalog_without_chapters" => manga(None, catalog_of(Visible, Partial)),
        "entry.novel.catalog_without_chapters" => novel(None, catalog_of(AllEligible, Complete)),
        "entry.manga.number_nan" => with(|c| {
            c.key = "x".into();
            c.chapter = Some(f64::NAN);
        }),
        "entry.manga.number_infinity" => with(|c| {
            c.key = "x".into();
            c.volume = Some(f64::INFINITY);
        }),
        "entry.manga.number_too_large" => with(|c| {
            c.key = "x".into();
            c.chapter = Some(1e10);
        }),
        "entry.manga.number_too_negative" => with(|c| {
            c.key = "x".into();
            c.volume = Some(-1e10);
        }),
        "entry.manga.key_at_limit" => with(|c| c.key = long('k', 512)),
        "entry.manga.key_empty" => with(|c| c.key.clear()),
        "entry.manga.key_oversized" => with(|c| c.key = long('k', 513)),
        "entry.manga.title_oversized" => with(|c| c.title = Some(long('t', 1025))),
        "entry.manga.scanlator_name_empty" => with(|c| c.scanlator = Some(Scanlator::named(""))),
        "entry.manga.scanlator_name_oversized" => {
            with(|c| c.scanlator = Some(Scanlator::named(long('n', 257))))
        }
        "entry.manga.scanlator_id_empty" => {
            with(|c| c.scanlator = Some(Scanlator::with_id("", "Name")))
        }
        "entry.manga.scanlator_id_oversized" => {
            with(|c| c.scanlator = Some(Scanlator::with_id(long('i', 257), "Name")))
        }
        "entry.manga.logical_unit_id_oversized" => {
            with(|c| c.logical_unit_id = Some(long('u', 257)))
        }
        "entry.manga.revision_oversized" => with(|c| c.revision = Some(long('r', 257))),
        "entry.manga.url_oversized" => with(|c| c.url = Some(long('u', 4097))),
        "entry.manga.lang_oversized" => with(|c| c.lang = Some(long('l', 36))),
        "entry.manga.too_many_chapters" => manga(Some(vec![chapter("k"); 20_001]), None),
        "entry.manga.unknown_scope" => {
            manga(single, catalog_of(Scope::Unknown, Completeness::Unknown))
        }
        "entry.manga.unknown_completeness" => {
            manga(single, catalog_of(Visible, Completeness::Unknown))
        }
        "entry.manga.host_to_plugin_no_catalog" => manga(single, None),
        "entry.manga.reading_order_newest_first" => {
            let ordered = |key: &str, order| Chapter {
                reading_order: Some(order),
                ..chapter(key)
            };
            manga(Some(vec![ordered("o-2", 2), ordered("o-1", 1)]), None)
        }
        "entry.manga.reading_order_extremes" => {
            let ordered = |key: &str, order| Chapter {
                reading_order: Some(order),
                ..chapter(key)
            };
            manga(
                Some(vec![ordered("o-min", i64::MIN), ordered("o-max", i64::MAX)]),
                None,
            )
        }
        "entry.manga.keys_nfc_nfd" => {
            manga(Some(vec![chapter("\u{e9}"), chapter("e\u{301}")]), None)
        }
        "entry.manga.scanlator_nfc_nfd_conflict" => manga(
            Some(vec![
                Chapter {
                    scanlator: Some(Scanlator::with_id("g-1", "\u{e9}")),
                    ..chapter("s-1")
                },
                Chapter {
                    scanlator: Some(Scanlator::with_id("g-1", "e\u{301}")),
                    ..chapter("s-2")
                },
            ]),
            None,
        ),
        "entry.manga.key_multibyte_oversized" => with(|c| c.key = long('\u{20ac}', 171)),
        "entry.manga.key_multibyte_at_limit" => with(|c| c.key = long('\u{1F600}', 128)),
        "entry.manga.date_at_limit" => with(|c| {
            c.key = "x".into();
            c.date_updated = Some(1e11);
        }),
        "entry.manga.date_nan" => with(|c| {
            c.key = "x".into();
            c.date_updated = Some(f64::NAN);
        }),
        "entry.manga.date_infinity" => with(|c| {
            c.key = "x".into();
            c.date_updated = Some(f64::INFINITY);
        }),
        "entry.manga.date_too_large" => with(|c| {
            c.key = "x".into();
            c.date_updated = Some(1e12);
        }),
        "entry.manga.revision_empty" => with(|c| c.revision = Some(String::new())),
        "entry.manga.key_oversized_and_nan" => with(|c| {
            c.key = long('k', 513);
            c.chapter = Some(f64::NAN);
        }),
        "entry.manga.duplicate_key_and_conflict" => manga(
            Some(vec![
                Chapter {
                    scanlator: Some(Scanlator::with_id("g-1", "Alpha Scans")),
                    ..chapter("d")
                },
                Chapter {
                    scanlator: Some(Scanlator::with_id("g-1", "Alpha Renamed")),
                    ..chapter("d")
                },
            ]),
            None,
        ),
        "error.contract_violation" => {
            return Built::Report(
                Error::Contract {
                    export: "get_media_update".into(),
                    code: "duplicate.key",
                }
                .report(),
            );
        }
        other => panic!("unhandled shared fixture {other}"),
    };
    Built::Entry(entry)
}

/// Checks the built value against the fixture bytes. A case with canonical bytes is accepted as
/// written but decodes to the built value, which encodes to the canonical bytes.
fn check<T: Serialize + DeserializeOwned + Debug>(
    expected: &T,
    bytes: &[u8],
    canonical: Option<&[u8]>,
    name: &str,
) {
    let canonical_bytes = canonical.unwrap_or(bytes);
    assert_eq!(
        ito_rs::postcard::to_allocvec(expected).unwrap(),
        canonical_bytes,
        "exact fixture encoding {name}"
    );
    let (decoded, rest): (T, _) = ito_rs::postcard::take_from_bytes(bytes).unwrap();
    assert!(rest.is_empty(), "trailing fixture bytes {name}");
    assert_eq!(
        format!("{decoded:?}"),
        format!("{expected:?}"),
        "decoded fixture {name}"
    );
    assert_eq!(
        ito_rs::postcard::to_allocvec(&decoded).unwrap(),
        canonical_bytes,
        "re-encoded fixture {name}"
    );
}

fn outcome(result: Result<(), Violation>) -> String {
    match result {
        Ok(()) => "ok".into(),
        Err(violation) => format!("violation:{}", violation.rule.code()),
    }
}

fn expected_outcome(entry: &Value) -> &str {
    entry
        .get("sdk_expect")
        .unwrap_or(&entry["expect"])
        .as_str()
        .unwrap()
}

/// Whether the SDK should be unable to decode the case at all.
fn undecodable(name: &str, bytes: &[u8]) -> bool {
    match name.split('.').next().unwrap() {
        "chapter" => ito_rs::postcard::from_bytes::<Chapter>(bytes).is_err(),
        "entry" => ito_rs::postcard::from_bytes::<MediaEntry>(bytes).is_err(),
        _ => panic!("undecodable case {name} has no type"),
    }
}

#[test]
fn every_shared_wire_fixture_has_exact_structure_and_validation() {
    let fixture = fixture();
    let cases = fixture["wire"].as_array().unwrap();
    assert!(!cases.is_empty());
    for entry in cases {
        let name = entry["name"].as_str().unwrap();
        let bytes = wire_bytes(entry);
        let canonical = entry
            .get("canonical_hex")
            .map(|hex| hex::decode(hex.as_str().unwrap()).unwrap());
        let expected = expected_outcome(entry);
        if expected == "decode_error" {
            assert!(undecodable(name, &bytes), "SDK must reject {name}");
            continue;
        }
        let result = match build(name) {
            Built::Chapter(value) => {
                assert_eq!(entry["type"], "Chapter", "{name}");
                check(&value, &bytes, canonical.as_deref(), name);
                catalog::validate_chapter(&value)
            }
            Built::Entry(value) => {
                assert_eq!(entry["type"], "MediaEntry", "{name}");
                check(&value, &bytes, canonical.as_deref(), name);
                catalog::validate_entry(&value)
            }
            Built::Pages(value) => {
                assert_eq!(entry["type"], "Vec<Page>", "{name}");
                check(&value, &bytes, canonical.as_deref(), name);
                Ok(())
            }
            Built::Report(value) => {
                assert_eq!(entry["type"], "ErrorReport", "{name}");
                check(&value, &bytes, canonical.as_deref(), name);
                Ok(())
            }
        };
        assert_eq!(outcome(result), expected, "validation {name}");
    }
}

#[test]
fn every_violation_stops_the_export_with_its_fixed_code() {
    for entry in fixture()["wire"].as_array().unwrap() {
        let name = entry["name"].as_str().unwrap();
        let expected = expected_outcome(entry);
        let Some(code) = expected.strip_prefix("violation:") else {
            continue;
        };
        let Built::Entry(value) = build(name) else {
            continue;
        };
        let error =
            export::__encode_checked("get_media_update", Ok(value), catalog::validate_entry, None)
                .unwrap_err();
        assert_eq!(error.category(), ErrorCategory::ContractViolation, "{name}");
        assert_eq!(error.message(), code, "{name}");
        assert!(
            matches!(&error, Error::Contract { export, .. } if export == "get_media_update"),
            "{name}"
        );
    }
}

#[test]
fn the_fixture_lists_every_code_and_pins_each_one_but_the_response_bound() {
    let fixture = fixture();
    let mut listed: Vec<&str> = fixture["codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    let mut known: Vec<&str> = Rule::ALL.iter().map(|rule| rule.code()).collect();
    known.push("enum.unknown");
    listed.sort_unstable();
    known.sort_unstable();
    assert_eq!(listed, known);

    let pinned: Vec<&str> = fixture["wire"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|entry| entry["expect"].as_str()?.strip_prefix("violation:"))
        .collect();
    for code in listed
        .into_iter()
        .filter(|code| *code != Rule::ResponseTooLarge.code())
    {
        assert!(pinned.contains(&code), "no fixture case for {code}");
    }
}

#[test]
fn the_limits_constants_equal_the_shared_limits() {
    let fixture = fixture();
    let limits = &fixture["limits"];
    let size = |key: &str| limits[key].as_u64().unwrap() as usize;
    assert_eq!(size("max_chapters"), catalog::MAX_CHAPTERS);
    assert_eq!(size("max_key_bytes"), catalog::MAX_KEY_BYTES);
    assert_eq!(size("max_title_bytes"), catalog::MAX_TITLE_BYTES);
    assert_eq!(
        size("max_scanlator_name_bytes"),
        catalog::MAX_SCANLATOR_NAME_BYTES
    );
    assert_eq!(size("max_id_bytes"), catalog::MAX_ID_BYTES);
    assert_eq!(size("max_revision_bytes"), catalog::MAX_REVISION_BYTES);
    assert_eq!(size("max_url_bytes"), catalog::MAX_URL_BYTES);
    assert_eq!(size("max_lang_bytes"), catalog::MAX_LANG_BYTES);
    assert_eq!(size("max_response_bytes"), catalog::MAX_RESPONSE_BYTES);
    assert_eq!(
        limits["max_abs_number"].as_f64().unwrap(),
        catalog::MAX_ABS_NUMBER
    );
    assert_eq!(
        limits["max_abs_date"].as_f64().unwrap(),
        catalog::MAX_ABS_DATE
    );
    assert_eq!(fixture["contract"], "ito-catalog");
    assert_eq!(fixture["version"], u64::from(ito_rs::ABI_VERSION));
}

#[test]
fn the_f64_collision_case_keeps_numbers_that_f32_would_merge() {
    let Built::Entry(MediaEntry::Manga(manga)) = build("entry.manga.f64_collision") else {
        panic!("a manga case");
    };
    let numbers: Vec<f64> = manga
        .chapters
        .unwrap()
        .iter()
        .map(|c| c.chapter.unwrap())
        .collect();
    assert_ne!(numbers[0], numbers[1]);
    assert_eq!(numbers[0] as f32, numbers[1] as f32);
}

#[test]
fn a_catalog_scope_or_completeness_beyond_the_known_values_decodes_to_unknown() {
    assert_eq!(
        ito_rs::postcard::from_bytes::<Scope>(&[6]).unwrap(),
        Scope::Unknown
    );
    assert_eq!(
        ito_rs::postcard::from_bytes::<Completeness>(&[6]).unwrap(),
        Completeness::Unknown
    );
    assert_eq!(
        ito_rs::postcard::from_bytes::<Scope>(&[4]).unwrap(),
        Scope::AllEligible
    );
    assert_eq!(
        ito_rs::postcard::from_bytes::<Completeness>(&[4]).unwrap(),
        Completeness::Complete
    );
    // Content kinds stay strict.
    assert!(ito_rs::postcard::from_bytes::<ContentKind>(&[4]).is_err());
}

#[test]
fn an_entry_that_travels_to_the_plugin_carries_no_catalog() {
    for entry in fixture()["wire"].as_array().unwrap() {
        let name = entry["name"].as_str().unwrap();
        let direction = entry["direction"].as_str().unwrap();
        assert!(
            ["plugin_to_host", "host_to_plugin", "both"].contains(&direction),
            "{name}"
        );
        if direction == "plugin_to_host" || entry["type"] != "MediaEntry" {
            continue;
        }
        let decoded: MediaEntry = ito_rs::postcard::from_bytes(&wire_bytes(entry)).unwrap();
        let has_catalog = match decoded {
            MediaEntry::Manga(manga) => manga.catalog.is_some(),
            MediaEntry::Novel(novel) => novel.catalog.is_some(),
            MediaEntry::Anime(_) => false,
        };
        assert!(!has_catalog, "{name} reaches the plugin with a catalog");
    }
}
