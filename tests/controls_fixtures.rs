use ito_rs::{
    controls::*,
    models::{FilterItem, FilterValue},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::collections::HashMap;

fn fixture() -> Value {
    serde_json::from_str(include_str!("../fixtures/controls-v1.json"))
        .expect("valid shared fixture JSON")
}
fn adult() -> Control {
    Control::toggle("adult", "Adult", false)
}
fn tags() -> Control {
    Control::multi_select(
        "tags",
        "Tags",
        OptionSource::searchable_catalog("tags"),
        vec![],
    )
    .visible_when("adult", vec!["true".into()])
}
fn sort() -> Control {
    Control::select(
        "sort",
        "Sort",
        OptionSource::inline(vec![
            ChoiceOption::new("views", "Popularity"),
            ChoiceOption::new("title", "Title"),
        ]),
        Some("views".into()),
    )
}
fn letters(grouped: bool) -> OptionSource {
    OptionSource::inline(
        [("a", "Alpha"), ("b", "Beta")]
            .into_iter()
            .map(|(v, l)| {
                let o = ChoiceOption::new(v, l);
                if grouped { o.grouped("Letters") } else { o }
            })
            .collect(),
    )
}
fn section(controls: Vec<Control>) -> ControlSection {
    ControlSection {
        id: "content".into(),
        title: "Content".into(),
        summary: None,
        flags: 0,
        controls,
    }
}
fn form(controls: Vec<Control>) -> SettingsForm {
    SettingsForm::new(vec![section(controls)])
}
fn all_kinds() -> SettingsForm {
    let mut text =
        Control::text("text", "Text", "https://example.test\npath").summary("Line one\nLine two");
    if let ControlKind::Text(t) = &mut text.kind {
        t.placeholder = Some("Enter a URL".into());
        t.flags = 3;
    }
    let mut secret = Control::secret("secret", "Secret");
    if let ControlKind::Secret(s) = &mut secret.kind {
        s.placeholder = Some("Credential".into());
    }
    let mut multi = Control::multi_select("multi", "Multi", letters(true), vec!["a".into()]);
    if let ControlKind::MultiSelect(m) = &mut multi.kind {
        m.max_selected = Some(2);
    }
    let mut integer = Control::integer("integer", "Integer", -10, 10);
    if let ControlKind::Integer(i) = &mut integer.kind {
        i.default_value = Some(0);
        i.unit = Some("items".into());
    }
    let mut range = Control::integer_range("range", "Range", 1900, 2100);
    if let ControlKind::IntegerRange(r) = &mut range.kind {
        r.default_lower = Some(2015);
        r.default_upper = Some(2024);
        r.unit = Some("year".into());
    }
    let mut s = section(vec![
        adult(),
        text,
        secret,
        Control::select("select", "Select", letters(true), Some("a".into())),
        multi,
        integer,
        range,
        tags(),
    ]);
    for c in &mut s.controls {
        if c.summary.is_none() {
            c.summary = Some("Details".into());
        }
        if c.id == "tags"
            && let ControlKind::MultiSelect(m) = &mut c.kind
        {
            m.max_selected = Some(64);
        }
    }
    s.summary = Some("All controls".into());
    s.flags = ControlSection::PAGE;
    SettingsForm::new(vec![s])
}
fn full() -> SearchCapabilities {
    let mut genres = Control::multi_select("genres", "Genres", letters(false), vec!["a".into()]);
    if let ControlKind::MultiSelect(m) = &mut genres.kind {
        m.max_selected = Some(2);
    }
    SearchCapabilities {
        flags: 15 | (1 << 31),
        sort: Some(sort()),
        sections: vec![ControlSection {
            id: "filters".into(),
            title: "Filters".into(),
            summary: None,
            flags: 0,
            controls: vec![
                adult(),
                Control::integer_range("year", "Year", 1900, 2100),
                genres,
                tags(),
            ],
        }],
    }
}
fn request(values: bool) -> OptionRequest {
    OptionRequest {
        catalog: "tags".into(),
        query: if values {
            OptionQuery::Values(vec!["t1".into(), "t9".into()])
        } else {
            OptionQuery::Search {
                text: "ro".into(),
                offset: 50,
            }
        },
        limit: if values { 2 } else { 50 },
    }
}
fn check<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(
    expected: &T,
    bytes: &[u8],
    name: &str,
) {
    assert_eq!(
        ito_rs::postcard::to_allocvec(expected).unwrap(),
        bytes,
        "exact fixture encoding {name}"
    );
    let (actual, rest): (T, _) = ito_rs::postcard::take_from_bytes(bytes).unwrap();
    assert!(rest.is_empty(), "trailing fixture bytes {name}");
    assert_eq!(&actual, expected, "decoded fixture {name}");
}
fn outcome(result: Result<(), Violation>) -> String {
    match result {
        Ok(()) => "ok".into(),
        Err(v) => format!(
            "violation:{}{}",
            v.rule.code(),
            v.id.map(|id| format!(":{id}")).unwrap_or_default()
        ),
    }
}
#[test]
fn every_shared_wire_fixture_has_exact_structure_and_validation() {
    for entry in fixture()["wire"].as_array().unwrap() {
        let name = entry["name"].as_str().unwrap();
        let bytes = hex::decode(entry["hex"].as_str().unwrap()).unwrap();
        let expected = entry
            .get("sdk_expect")
            .unwrap_or(&entry["expect"])
            .as_str()
            .unwrap();
        let result = match name {
            "settings_form.empty"
            | "settings_form.one_toggle"
            | "settings_form.all_kinds"
            | "settings_form.flags"
            | "settings_form.duplicate_control"
            | "settings_form.bad_id"
            | "settings_form.too_many_sections"
            | "control.condition_on_conditional"
            | "control.condition_toggle_value" => {
                let v = match name {
                    "settings_form.empty" => SettingsForm::new(vec![]),
                    "settings_form.one_toggle" => form(vec![adult()]),
                    "settings_form.all_kinds" => all_kinds(),
                    "settings_form.flags" => {
                        let mut c = Control::text("text", "Text", "https://example.test\npath");
                        if let ControlKind::Text(t) = &mut c.kind {
                            t.placeholder = Some("URL".into());
                            t.flags = 3;
                        }
                        let mut f = form(vec![c]);
                        f.sections[0].flags = ControlSection::PAGE | (1 << 31);
                        f
                    }
                    "settings_form.duplicate_control" => form(vec![adult(), adult()]),
                    "settings_form.bad_id" => form(vec![Control::toggle("bad id", "Adult", false)]),
                    "settings_form.too_many_sections" => SettingsForm::new(
                        (0..33)
                            .map(|n| ControlSection::new(format!("s{n}"), "Section"))
                            .collect(),
                    ),
                    "control.condition_on_conditional" => form(vec![
                        adult(),
                        tags(),
                        Control::toggle("nested", "Nested", false)
                            .visible_when("tags", vec!["t1".into()]),
                    ]),
                    "control.condition_toggle_value" => form(vec![
                        adult(),
                        Control::toggle("nested", "Nested", false)
                            .visible_when("adult", vec!["yes".into()]),
                    ]),
                    _ => unreachable!(),
                };
                check(&v, &bytes, name);
                validate_form(&v)
            }
            "control.multi_select_catalog"
            | "control.integer"
            | "control.select_default_missing"
            | "control.min_gt_max" => {
                let c = match name {
                    "control.multi_select_catalog" => tags(),
                    "control.integer" => Control::integer("year", "Year", 1900, 2100),
                    "control.select_default_missing" => Control::select(
                        "select",
                        "Select",
                        OptionSource::inline(vec![ChoiceOption::new("a", "Alpha")]),
                        Some("b".into()),
                    ),
                    "control.min_gt_max" => Control::integer("year", "Year", 10, 1),
                    _ => unreachable!(),
                };
                check(&c, &bytes, name);
                limits::validate_control(&c)
            }
            "control.unknown_kind_99"
            | "control.unknown_kind_empty_body"
            | "control.body_trailing_byte" => {
                let (id, title, tag, body) = match name {
                    "control.unknown_kind_99" => ("future", "Future", 99u32, vec![0xaa, 0xbb]),
                    "control.unknown_kind_empty_body" => ("future", "Future", 7u32, vec![]),
                    _ => ("adult", "Adult", 0u32, vec![0, 0]),
                };
                let header = (
                    id,
                    title,
                    None::<String>,
                    0u32,
                    Vec::<ControlCondition>::new(),
                    tag,
                    serde_bytes::Bytes::new(&body),
                );
                assert_eq!(
                    ito_rs::postcard::to_allocvec(&header).unwrap(),
                    bytes,
                    "raw envelope {name}"
                );
                assert!(
                    ito_rs::postcard::from_bytes::<Control>(&bytes).is_err(),
                    "SDK must reject {name}"
                );
                continue;
            }
            "search_capabilities.sort_only"
            | "search_capabilities.full"
            | "search_capabilities.secret_filter"
            | "search_capabilities.sort_catalog" => {
                let v = match name {
                    "search_capabilities.sort_only" => SearchCapabilities {
                        flags: 5,
                        sort: Some(sort()),
                        sections: vec![],
                    },
                    "search_capabilities.full" => full(),
                    "search_capabilities.secret_filter" => SearchCapabilities {
                        flags: 0,
                        sort: None,
                        sections: vec![ControlSection {
                            id: "filters".into(),
                            title: "Filters".into(),
                            summary: None,
                            flags: 0,
                            controls: vec![Control::secret("secret", "Secret")],
                        }],
                    },
                    _ => SearchCapabilities {
                        flags: 0,
                        sort: Some(Control::select(
                            "sort",
                            "Sort",
                            OptionSource::searchable_catalog("tags"),
                            Some("t1".into()),
                        )),
                        sections: vec![],
                    },
                };
                check(&v, &bytes, name);
                validate_capabilities(&v)
            }
            "option_page.utf8_distinct" | "option_page.utf8_wrong_normalization" => {
                let distinct = name == "option_page.utf8_distinct";
                let requested = if distinct {
                    vec!["é", "e\u{301}"]
                } else {
                    vec!["é"]
                };
                let returned = if distinct {
                    vec!["é", "e\u{301}"]
                } else {
                    vec!["e\u{301}"]
                };
                let request = OptionRequest {
                    catalog: "tags".into(),
                    query: OptionQuery::Values(requested.into_iter().map(String::from).collect()),
                    limit: if distinct { 2 } else { 1 },
                };
                let page = OptionPage {
                    options: returned
                        .into_iter()
                        .map(|v| ChoiceOption::new(v, "Label"))
                        .collect(),
                    next_offset: None,
                    total: None,
                    revision: None,
                };
                check(&page, &bytes, name);
                validate_page(&page, &request)
            }
            "option_request.utf8_distinct" | "option_request.utf8_composed" => {
                let distinct = name == "option_request.utf8_distinct";
                let values = if distinct {
                    vec!["é", "e\u{301}"]
                } else {
                    vec!["é"]
                };
                let request = OptionRequest {
                    catalog: "tags".into(),
                    query: OptionQuery::Values(values.into_iter().map(String::from).collect()),
                    limit: if distinct { 2 } else { 1 },
                };
                check(&request, &bytes, name);
                validate_request(&request)
            }
            "option_page.first"
            | "option_page.values"
            | "option_page.no_progress"
            | "option_page.unrequested_value"
            | "option_page.too_many_options" => {
                let v = match name {
                    "option_page.first" => OptionPage {
                        options: vec![ChoiceOption::new("t1", "Romance").grouped("Themes")],
                        next_offset: Some(51),
                        total: Some(6141),
                        revision: None,
                    },
                    "option_page.values" => OptionPage {
                        options: vec![
                            ChoiceOption::new("t1", "Romance").grouped("Themes"),
                            ChoiceOption::new("t9", "Adventure"),
                        ],
                        next_offset: None,
                        total: None,
                        revision: Some("v1".into()),
                    },
                    "option_page.no_progress" => OptionPage {
                        options: vec![ChoiceOption::new("t1", "Romance")],
                        next_offset: Some(50),
                        total: None,
                        revision: None,
                    },
                    "option_page.unrequested_value" => OptionPage {
                        options: vec![ChoiceOption::new("other", "Other")],
                        next_offset: None,
                        total: None,
                        revision: None,
                    },
                    _ => OptionPage {
                        options: (0..51)
                            .map(|n| ChoiceOption::new(format!("t{n}"), "Tag"))
                            .collect(),
                        next_offset: None,
                        total: None,
                        revision: None,
                    },
                };
                check(&v, &bytes, name);
                validate_page(
                    &v,
                    &request(matches!(
                        name,
                        "option_page.values" | "option_page.unrequested_value"
                    )),
                )
            }
            "suggestions.one" | "suggestions.empty" => {
                let v = if name == "suggestions.one" {
                    vec![SearchSuggestion {
                        query: "Frieren".into(),
                        subtitle: None,
                    }]
                } else {
                    vec![]
                };
                check(&v, &bytes, name);
                validate_suggestions(&v)
            }
            "option_request.search" | "option_request.values" => {
                let v = request(name.ends_with("values"));
                check(&v, &bytes, name);
                validate_request(&v)
            }
            "filters.dense" | "filters.empty" => {
                let v = if name == "filters.empty" {
                    vec![]
                } else {
                    full().filter_items(&HashMap::from([
                        ("sort".into(), ControlValue::Select(Some("title".into()))),
                        ("adult".into(), ControlValue::Toggle(false)),
                        ("year".into(), ControlValue::IntegerRange(Some(2101), None)),
                        ("tags".into(), ControlValue::MultiSelect(vec!["t9".into()])),
                    ]))
                };
                assert_eq!(
                    ito_rs::postcard::to_allocvec(&v).unwrap(),
                    bytes,
                    "dense effective transport {name}"
                );
                let decoded: Vec<FilterItem> = ito_rs::postcard::from_bytes(&bytes).unwrap();
                assert_eq!(decoded.len(), v.len());
                for (a, b) in decoded.iter().zip(&v) {
                    assert_eq!((&a.type_name, &a.name), (&b.type_name, &b.name));
                    let (FilterValue::String(a), FilterValue::String(b)) = (&a.value, &b.value)
                    else {
                        panic!("noncanonical dense filter")
                    };
                    assert_eq!(a, b);
                }
                let values = ControlValues::search(&full(), &decoded).unwrap();
                assert_eq!(values.is_host_provided(), name == "filters.dense");
                assert_eq!(values.multi_select("genres").unwrap(), &["a"]);
                assert!(values.multi_select("tags").unwrap().is_empty());
                Ok(())
            }
            _ => panic!("unhandled shared fixture {name}"),
        };
        assert_eq!(outcome(result), expected, "validation {name}");
    }
}

#[test]
fn every_shared_canonical_value_checks_both_directions() {
    for entry in fixture()["values"].as_array().unwrap() {
        let kind = match entry["kind"].as_str().unwrap() {
            "toggle" => adult().kind,
            "integer" => Control::integer("i", "Integer", i64::MIN, i64::MAX).kind,
            "multi_select" => {
                Control::multi_select("m", "Multi", OptionSource::catalog("tags"), vec![]).kind
            }
            "integer_range" => Control::integer_range("r", "Range", i64::MIN, i64::MAX).kind,
            kind => panic!("unhandled value kind {kind}"),
        };
        let raw = entry["canonical"].as_str().unwrap();
        if entry["invalid"] == true {
            assert!(kind.parse_value(raw).is_err(), "accepted invalid {entry}");
            continue;
        }
        let expected = match entry["kind"].as_str().unwrap() {
            "toggle" => ControlValue::Toggle(entry["value"].as_bool().unwrap()),
            "integer" => ControlValue::Integer(entry["value"].as_i64()),
            "multi_select" => ControlValue::MultiSelect(
                entry["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| s.as_str().unwrap().into())
                    .collect(),
            ),
            "integer_range" => {
                ControlValue::IntegerRange(entry["lower"].as_i64(), entry["upper"].as_i64())
            }
            _ => unreachable!(),
        };
        if entry["runtime_invalid"] == true {
            assert_eq!(
                expected.canonical(),
                raw,
                "syntax-only canonical writer {entry}"
            );
            assert!(kind.parse_value(raw).is_err(), "runtime accepted {entry}");
            assert!(
                !kind.accepts(&expected),
                "runtime accepted typed value {entry}"
            );
            continue;
        }
        assert_eq!(
            kind.parse_value(raw).unwrap(),
            expected,
            "decoded value {entry}"
        );
        assert_eq!(expected.canonical(), raw, "canonical writer {entry}");
    }
}
