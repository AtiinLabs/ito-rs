use ito_rs::{
    Error,
    controls::*,
    models::{FilterItem, FilterValue},
};
use std::{cell::RefCell, collections::HashMap};

// These tests exercise refusal paths, never a WASM pointer on the native test host.
// A forbidden storage call fails immediately rather than silently succeeding.
#[cfg(not(target_arch = "wasm32"))]
#[unsafe(export_name = "get")]
extern "C" fn unexpected_host_get(_: i32, _: i32) -> i64 {
    panic!("hidden or non-secret lookup unexpectedly reached host defaults")
}
#[cfg(not(target_arch = "wasm32"))]
#[unsafe(export_name = "set")]
extern "C" fn unexpected_host_set(_: i32, _: i32, _: i32, _: i32) {
    panic!("refused setting write unexpectedly reached host defaults")
}

fn form() -> SettingsForm {
    SettingsForm::new(vec![
        ControlSection::new("content", "Content")
            .control(Control::toggle("enabled", "Enabled", false))
            .control(Control::multi_select(
                "tags",
                "Tags",
                OptionSource::inline(vec![ChoiceOption::new("a", "Alpha")]),
                vec!["a".into()],
            ))
            .control(Control::integer("year", "Year", 1900, 2100))
            .control(Control::text("note", "Note", "declared"))
            .control(Control::select(
                "choice",
                "Choice",
                OptionSource::inline(vec![ChoiceOption::new("a", "Alpha")]),
                Some("a".into()),
            ))
            .control(
                Control::text("conditional", "Conditional", "fallback")
                    .visible_when("enabled", vec!["true".into()]),
            )
            .control(
                Control::secret("credential", "Credential")
                    .visible_when("enabled", vec!["true".into()]),
            ),
    ])
}
fn caps() -> SearchCapabilities {
    let mut sections = form().sections;
    sections[0]
        .controls
        .retain(|c| !matches!(c.kind, ControlKind::Secret(_)));
    SearchCapabilities {
        flags: 15,
        sort: Some(Control::select(
            "sort",
            "Sort",
            OptionSource::inline(vec![
                ChoiceOption::new("title", "Title"),
                ChoiceOption::new("recent", "Recent"),
            ]),
            Some("title".into()),
        )),
        sections,
    }
}
fn item(name: &str, kind: &str, value: &str) -> FilterItem {
    FilterItem {
        name: name.into(),
        type_name: kind.into(),
        value: FilterValue::String(value.into()),
    }
}
#[test]
fn absent_empty_malformed_and_hidden_settings_keep_their_distinct_meanings() {
    let f = form();
    let empty = ControlValues::settings_with(&f, |_| None);
    assert!(empty.is_host_provided());
    assert_eq!(empty.multi_select("tags").unwrap(), &["a"]);
    assert_eq!(empty.integer("year").unwrap(), None);
    assert!(empty.issues().is_empty());
    let reads = RefCell::new(Vec::new());
    let stored = HashMap::from([
        ("tags", "[]"),
        ("year", "SECRET INVALID INPUT"),
        ("conditional", "hidden value"),
        ("note", ""),
    ]);
    let values = ControlValues::settings_with(&f, |key| {
        reads.borrow_mut().push(key.to_owned());
        stored.get(key).map(|v| (*v).into())
    });
    assert!(
        !reads.borrow().iter().any(|key| key == "credential"),
        "plain settings lookup must never read a secret"
    );
    assert!(
        values.multi_select("tags").unwrap().is_empty(),
        "explicit clear differs from absent default"
    );
    assert_eq!(values.integer("year").unwrap(), None);
    assert_eq!(values.text("note").unwrap(), "");
    assert_eq!(values.text("conditional").unwrap(), "fallback");
    assert_eq!(
        values.secret("credential").unwrap(),
        None,
        "hidden credentials never call host storage"
    );
    assert_eq!(values.issues().len(), 1);
    assert_eq!(values.issues()[0].key, "year");
    assert_eq!(
        values.issues()[0].message,
        "Saved value isn't valid; the source uses the default"
    );
    assert!(!format!("{:?}", values.issues()).contains("SECRET"));
    assert!(!f.visible_control_ids(&values).contains("conditional"));
    let visible = ControlValues::settings_with(&f, |key| match key {
        "enabled" => Some("true".into()),
        "conditional" => Some("saved".into()),
        _ => None,
    });
    assert_eq!(visible.text("conditional").unwrap(), "saved");
    assert!(f.visible_control_ids(&visible).contains("credential"));
}
#[test]
fn option_identity_is_independent_of_label_and_inline_membership() {
    let mut f = form();
    let values = ControlValues::settings_with(&f, |key| match key {
        "tags" => Some("[\"removed\",\"a\"]".into()),
        "choice" => Some("removed".into()),
        _ => None,
    });
    assert!(values.issues().is_empty());
    assert_eq!(values.multi_select("tags").unwrap(), &["removed", "a"]);
    assert_eq!(values.select("choice").unwrap(), Some("removed"));
    if let ControlKind::MultiSelect(m) = &mut f.sections[0].controls[1].kind
        && let OptionSource::Inline(o) = &mut m.options
    {
        o[0].label = "Renamed".into();
    }
    let renamed = ControlValues::settings_with(&f, |key| {
        (key == "tags").then(|| "[\"removed\",\"a\"]".into())
    });
    assert_eq!(
        renamed.multi_select("tags").unwrap(),
        values.multi_select("tags").unwrap()
    );
}
#[test]
fn dense_search_defaults_visibility_order_and_invalid_inputs() {
    let cap = caps();
    let old_host = ControlValues::search(&cap, &[]).unwrap();
    assert!(!old_host.is_host_provided());
    assert_eq!(old_host.select("sort").unwrap(), Some("title"));
    let dense = cap.filter_items(&HashMap::from([
        ("enabled".into(), ControlValue::Toggle(false)),
        ("conditional".into(), ControlValue::Text("hidden".into())),
        ("year".into(), ControlValue::Integer(Some(2101))),
        ("unknown".into(), ControlValue::Toggle(true)),
    ]));
    assert_eq!(
        dense.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        vec![
            "sort",
            "enabled",
            "tags",
            "year",
            "note",
            "choice",
            "conditional"
        ]
    );
    let values = ControlValues::search(&cap, &dense).unwrap();
    assert!(values.is_host_provided());
    assert_eq!(values.integer("year").unwrap(), None);
    assert_eq!(values.text("conditional").unwrap(), "fallback");
    let values = ControlValues::search(
        &cap,
        &[
            item("enabled", "toggle", "true"),
            item("conditional", "text", "saved"),
            item("unknown", "arbitrary", "garbage"),
        ],
    )
    .unwrap();
    assert_eq!(values.text("conditional").unwrap(), "saved");
    for filter in [
        item("enabled", "text", "true"),
        item("enabled", "toggle", "True"),
        item("year", "integer", "1800"),
        FilterItem {
            name: "enabled".into(),
            type_name: "toggle".into(),
            value: FilterValue::Boolean(true),
        },
    ] {
        assert!(matches!(
            ControlValues::search(&cap, &[filter]),
            Err(Error::Host(_))
        ));
    }
    assert!(matches!(
        ControlValues::search(
            &cap,
            &[
                item("enabled", "toggle", "true"),
                item("enabled", "toggle", "false")
            ]
        ),
        Err(Error::Host(_))
    ));
    for result in [
        values.text("missing").map(|_| ()),
        values.text("enabled").map(|_| ()),
        values.secret("enabled").map(|_| ()),
    ] {
        assert!(matches!(result, Err(Error::Host(_))));
    }
}
#[test]
fn runtime_constraints_kind_identity_and_strict_ranges() {
    let toggle = Control::toggle("t", "T", false).kind;
    assert!(!toggle.accepts(&ControlValue::Text("true".into())));
    let text = Control::text("t", "T", "").kind;
    for raw in ["\0", "\t", "\u{7f}", "\u{85}", "line\nline"] {
        assert!(text.parse_value(raw).is_err());
    }
    let mut multiline = Control::text("t", "T", "");
    if let ControlKind::Text(t) = &mut multiline.kind {
        t.flags = TextControl::MULTILINE;
    }
    assert!(multiline.kind.parse_value("line\nline").is_ok());
    let mut multi = Control::multi_select("m", "M", OptionSource::catalog("c"), vec![]);
    if let ControlKind::MultiSelect(m) = &mut multi.kind {
        m.max_selected = Some(2);
    }
    assert!(multi.kind.parse_value("[\"a\",\"b\"]").is_ok());
    assert!(multi.kind.parse_value("[\"a\",\"b\",\"c\"]").is_err());
    let range = Control::integer_range("r", "R", 0, 10).kind;
    for invalid in [
        "[,1]",
        "[1,]",
        "[, ]",
        "[null,]",
        "[11,null]",
        "[null,-1]",
        "[2,1]",
        "[0,1,2]",
    ] {
        assert!(range.parse_value(invalid).is_err(), "accepted {invalid}");
    }
    assert_eq!(
        range.parse_value("[0,10]").unwrap(),
        ControlValue::IntegerRange(Some(0), Some(10))
    );
    let select = Control::select("s", "S", OptionSource::catalog("c"), None).kind;
    assert!(!select.accepts(&ControlValue::Text("raw".into())));
    assert!(matches!(
        write_setting(
            &Control::secret("secret", "Secret"),
            &ControlValue::Text("do not persist".into())
        ),
        Err(Error::Host(_))
    ));
    assert!(matches!(
        write_setting(
            &Control::toggle("t", "T", false),
            &ControlValue::Text("true".into())
        ),
        Err(Error::Host(_))
    ));
}
#[test]
fn envelopes_round_trip_nested_lengths_and_reject_trailing_bodies() {
    let controls = [
        Control::toggle("t", "Toggle", false),
        Control::text("text", "Text", "a".repeat(130)),
        Control::secret("s", "Secret"),
        Control::select("s", "Select", OptionSource::catalog("c"), None),
        Control::multi_select("m", "Multi", OptionSource::catalog("c"), vec![]),
        Control::integer("i", "Integer", i64::MIN, i64::MAX),
        Control::integer_range("r", "Range", 0, 10),
    ];
    for (tag, c) in controls.iter().enumerate() {
        assert_eq!(c.kind.tag(), tag as u32);
        let bytes = ito_rs::postcard::to_allocvec(&c.kind).unwrap();
        let decoded: ControlKind = ito_rs::postcard::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, c.kind);
        let (encoded_tag, body): (u32, serde_bytes::ByteBuf) =
            ito_rs::postcard::from_bytes(&bytes).unwrap();
        assert_eq!(encoded_tag, tag as u32);
        let mut bad = body.to_vec();
        bad.push(0);
        let bytes =
            ito_rs::postcard::to_allocvec(&(encoded_tag, serde_bytes::Bytes::new(&bad))).unwrap();
        assert!(
            ito_rs::postcard::from_bytes::<ControlKind>(&bytes).is_err(),
            "trailing kind tag {tag}"
        );
        let bytes = ito_rs::postcard::to_allocvec(c).unwrap();
        let decoded: Control = ito_rs::postcard::from_bytes(&bytes).unwrap();
        assert_eq!(&decoded, c);
    }
}

#[test]
fn runtime_option_controls_reject_all_c0_c1_characters_across_helpers() {
    let multi = Control::multi_select(
        "tags",
        "Tags",
        OptionSource::catalog("tags"),
        vec!["a".into()],
    );
    let select = Control::select("choice", "Choice", OptionSource::catalog("tags"), None);
    let f = SettingsForm::new(vec![
        ControlSection::new("section", "Section").control(multi.clone()),
    ]);
    let cap = SearchCapabilities {
        flags: 0,
        sort: None,
        sections: f.sections.clone(),
    };
    for code in (0..=31).chain(127..=159) {
        let invalid = format!("a{}", char::from_u32(code).unwrap());
        let value = ControlValue::MultiSelect(vec![invalid.clone()]);
        let raw = value.canonical();
        assert!(
            multi.kind.parse_value(&raw).is_err(),
            "accepted U+{code:04X}"
        );
        assert!(!multi.kind.accepts(&value), "accepted U+{code:04X}");
        assert!(write_setting(&multi, &value).is_err(), "wrote U+{code:04X}");
        assert!(
            select.kind.parse_value(&invalid).is_err(),
            "select accepted U+{code:04X}"
        );
        let saved = ControlValues::settings_with(&f, |_| Some(raw.clone()));
        assert_eq!(saved.multi_select("tags").unwrap(), &["a"]);
        assert_eq!(saved.issues().len(), 1);
        assert_eq!(saved.issues()[0].key, "tags");
        assert!(ControlValues::search(&cap, &[item("tags", "multi_select", &raw)]).is_err());
        let dense = cap.filter_items(&HashMap::from([("tags".into(), value)]));
        let effective = ControlValues::search(&cap, &dense).unwrap();
        assert_eq!(effective.multi_select("tags").unwrap(), &["a"]);
    }
}

#[test]
fn visible_search_controls_use_accepted_values_or_defaults_before_conditions() {
    let cap = SearchCapabilities {
        flags: 0,
        sort: None,
        sections: vec![
            ControlSection::new("section", "Section")
                .control(Control::toggle("enabled", "Enabled", false))
                .control(
                    Control::text("dependent", "Dependent", "")
                        .visible_when("enabled", vec!["true".into()]),
                )
                .control(Control::select(
                    "choice",
                    "Choice",
                    OptionSource::catalog("tags"),
                    Some("é".into()),
                ))
                .control(
                    Control::text("exact", "Exact", "").visible_when("choice", vec!["é".into()]),
                )
                .control(
                    Control::text("missing", "Missing", "")
                        .visible_when("undeclared", vec!["true".into()]),
                ),
        ],
    };
    for supplied in [
        HashMap::new(),
        HashMap::from([("enabled".into(), ControlValue::Text("true".into()))]),
        HashMap::from([("enabled".into(), ControlValue::Select(Some("true".into())))]),
    ] {
        let visible = cap.visible_control_ids(&supplied);
        assert!(!visible.contains("dependent"));
        assert!(
            visible.contains("exact"),
            "default select must precede conditions"
        );
        assert!(
            !visible.contains("missing"),
            "absent target cannot satisfy a condition"
        );
        let dense = cap.filter_items(&supplied);
        let effective = ControlValues::search(&cap, &dense).unwrap();
        assert!(!effective.toggle("enabled").unwrap());
    }
    let supplied = HashMap::from([
        ("enabled".into(), ControlValue::Toggle(true)),
        (
            "choice".into(),
            ControlValue::Select(Some("e\u{301}".into())),
        ),
    ]);
    let visible = cap.visible_control_ids(&supplied);
    assert!(visible.contains("dependent"));
    assert!(
        !visible.contains("exact"),
        "opaque values must not normalize"
    );
}

#[test]
fn opaque_multi_values_preserve_distinct_utf8_through_settings_and_search() {
    let expected = vec!["é".to_owned(), "e\u{301}".to_owned()];
    let control = Control::multi_select(
        "tags",
        "Tags",
        OptionSource::inline(vec![
            ChoiceOption::new("é", "Composed"),
            ChoiceOption::new("e\u{301}", "Decomposed"),
        ]),
        expected.clone(),
    );
    let f = SettingsForm::new(vec![
        ControlSection::new("section", "Section").control(control.clone()),
    ]);
    assert!(validate_form(&f).is_ok());
    let value = ControlValue::MultiSelect(expected.clone());
    assert!(control.kind.accepts(&value));
    let saved = ControlValues::settings_with(&f, |_| Some(value.canonical()));
    assert!(saved.issues().is_empty());
    assert_eq!(saved.multi_select("tags").unwrap(), expected);
    let cap = SearchCapabilities {
        flags: 0,
        sort: None,
        sections: f.sections,
    };
    let dense = cap.filter_items(&HashMap::from([("tags".into(), value)]));
    let effective = ControlValues::search(&cap, &dense).unwrap();
    assert_eq!(effective.multi_select("tags").unwrap(), expected);
}
