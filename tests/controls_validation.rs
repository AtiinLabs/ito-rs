use ito_rs::controls::*;

fn c() -> Control {
    Control::toggle("control", "Title", false)
}
fn form(c: Control) -> SettingsForm {
    SettingsForm::new(vec![ControlSection::new("section", "Section").control(c)])
}
fn rule(result: Result<(), Violation>, expected: Rule) {
    assert_eq!(result.unwrap_err().rule, expected);
}
fn options(n: usize) -> OptionSource {
    OptionSource::inline(
        (0..n)
            .map(|n| ChoiceOption::new(format!("v{n}"), "Label"))
            .collect(),
    )
}
fn request() -> OptionRequest {
    OptionRequest {
        catalog: "tags".into(),
        query: OptionQuery::Search {
            text: String::new(),
            offset: 5,
        },
        limit: 100,
    }
}
fn page() -> OptionPage {
    OptionPage {
        options: vec![ChoiceOption::new("a", "Alpha")],
        next_offset: Some(6),
        total: None,
        revision: Some("v1".into()),
    }
}

#[test]
fn identifiers_validate_bytes_charset_and_never_leak_invalid_ids() {
    for valid in ["a".into(), "A0._-".into(), "a".repeat(64)] {
        let mut v = c();
        v.id = valid;
        assert!(validate_form(&form(v)).is_ok());
    }
    for invalid in [String::new(), "a".repeat(65)] {
        let mut v = c();
        v.id = invalid;
        let err = validate_form(&form(v)).unwrap_err();
        assert_eq!(err.rule, Rule::LengthId);
        assert_eq!(err.id, None);
    }
    for invalid in ["with space", "秘密", "bad\n"] {
        let mut v = c();
        v.id = invalid.into();
        let err = validate_form(&form(v)).unwrap_err();
        assert_eq!(err.rule, Rule::CharsId);
        assert_eq!(err.id, None);
        assert!(!err.to_string().contains(invalid));
    }
    assert_eq!(
        Violation::with_id(Rule::CharsId, "credential secret").id,
        None
    );
}
#[test]
fn text_limits_distinguish_length_from_character_errors() {
    for (max, expected, field) in [(256, Rule::LengthTitle, 0), (1024, Rule::LengthSummary, 1)] {
        for (n, ok) in [(1, true), (max, true), (0, false), (max + 1, false)] {
            let mut v = c();
            let s = "a".repeat(n);
            if field == 0 {
                v.title = s;
            } else {
                v.summary = Some(s);
            }
            let result = validate_form(&form(v));
            if ok {
                assert!(result.is_ok());
            } else {
                rule(result, expected);
            }
        }
    }
    for value in ["\0", "\t", "\u{7f}", "\u{85}", "\u{9f}"] {
        let mut v = c();
        v.title = value.into();
        rule(validate_form(&form(v)), Rule::CharsText);
    }
    let mut v = c();
    v.summary = Some("a\nb".into());
    assert!(validate_form(&form(v)).is_ok());
    let mut v = c();
    v.title = "é".repeat(129);
    rule(validate_form(&form(v)), Rule::LengthTitle);
}
#[test]
fn collection_counts_and_running_totals_are_enforced() {
    let make = |n| {
        SettingsForm::new(
            (0..n)
                .map(|n| ControlSection::new(format!("s{n}"), "S"))
                .collect(),
        )
    };
    assert!(validate_form(&make(32)).is_ok());
    rule(validate_form(&make(33)), Rule::CountSections);
    let make = |n| {
        SettingsForm::new(vec![ControlSection {
            id: "s".into(),
            title: "S".into(),
            summary: None,
            flags: 0,
            controls: (0..n)
                .map(|n| Control::toggle(format!("c{n}"), "C", false))
                .collect(),
        }])
    };
    assert!(validate_form(&make(64)).is_ok());
    rule(validate_form(&make(65)), Rule::CountSectionControls);
    let make = |n: usize| {
        SettingsForm::new(
            (0..n.div_ceil(64))
                .map(|s| ControlSection {
                    id: format!("s{s}"),
                    title: "S".into(),
                    summary: None,
                    flags: 0,
                    controls: (s * 64..((s + 1) * 64).min(n))
                        .map(|n| Control::toggle(format!("c{n}"), "C", false))
                        .collect(),
                })
                .collect(),
        )
    };
    assert!(validate_form(&make(256)).is_ok());
    rule(validate_form(&make(257)), Rule::CountControls);
    let make = |n| form(Control::select("select", "Select", options(n), None));
    assert!(validate_form(&make(1)).is_ok());
    assert!(validate_form(&make(256)).is_ok());
    rule(validate_form(&make(0)), Rule::CountOptions);
    rule(validate_form(&make(257)), Rule::CountOptions);
    let mut f = form(c());
    f.sections[0].controls = (0..8)
        .map(|n| Control::select(format!("s{n}"), "S", options(256), None))
        .collect();
    assert!(validate_form(&f).is_ok());
    f.sections[0]
        .controls
        .push(Control::select("extra", "Extra", options(1), None));
    rule(validate_form(&f), Rule::CountFormOptions);
    let mut f = form(c());
    f.sections.push(f.sections[0].clone());
    rule(validate_form(&f), Rule::DuplicateSection);
    let mut f = form(c());
    f.sections[0].controls.push(c());
    rule(validate_form(&f), Rule::DuplicateControl);
}
#[test]
fn option_fields_and_duplicate_values_are_checked_in_field_order() {
    for (field, max, expected) in [
        (0, 256, Rule::LengthValue),
        (1, 256, Rule::LengthLabel),
        (2, 128, Rule::LengthGroup),
    ] {
        for (n, ok) in [(1, true), (max, true), (0, false), (max + 1, false)] {
            let mut o = ChoiceOption::new("v", "Label");
            let s = "a".repeat(n);
            match field {
                0 => o.value = s,
                1 => o.label = s,
                _ => o.group = Some(s),
            }
            let f = form(Control::select(
                "select",
                "Select",
                OptionSource::inline(vec![o]),
                None,
            ));
            if ok {
                assert!(validate_form(&f).is_ok());
            } else {
                rule(validate_form(&f), expected);
            }
        }
    }
    let make = |o| {
        form(Control::select(
            "s",
            "Select",
            OptionSource::inline(o),
            None,
        ))
    };
    rule(
        validate_form(&make(vec![
            ChoiceOption::new("a", "A"),
            ChoiceOption::new("a", "B"),
        ])),
        Rule::DuplicateOption,
    );
    rule(
        validate_form(&make(vec![ChoiceOption::new("\n", "A")])),
        Rule::CharsValue,
    );
    rule(
        validate_form(&make(vec![ChoiceOption::new("a", "\n")])),
        Rule::CharsText,
    );
    rule(
        validate_form(&make(vec![ChoiceOption::new("a", "A").grouped("\t")])),
        Rule::CharsText,
    );
    assert!(
        validate_form(&make(vec![ChoiceOption::new("\u{85}", "A")])).is_ok(),
        "value characters intentionally allow C1"
    );
}
#[test]
fn optional_fields_defaults_and_flags_obey_their_own_limits() {
    for (max, rule_expected, field) in
        [(128, Rule::LengthPlaceholder, 0), (32, Rule::LengthUnit, 1)]
    {
        for (n, ok) in [(1, true), (max, true), (0, false), (max + 1, false)] {
            let mut v = if field == 0 {
                Control::secret("s", "Secret")
            } else {
                Control::integer("i", "Integer", 0, 1)
            };
            match &mut v.kind {
                ControlKind::Secret(s) => s.placeholder = Some("x".repeat(n)),
                ControlKind::Integer(i) => i.unit = Some("x".repeat(n)),
                _ => unreachable!(),
            }
            if ok {
                assert!(validate_form(&form(v)).is_ok());
            } else {
                rule(validate_form(&form(v)), rule_expected);
            }
        }
    }
    let mut text = Control::text("t", "Text", "x".repeat(4096));
    assert!(validate_form(&form(text.clone())).is_ok());
    if let ControlKind::Text(t) = &mut text.kind {
        t.default_value.push('x');
    }
    rule(validate_form(&form(text)), Rule::LengthDefault);
    let mut text = Control::text("t", "Text", "line\nline");
    rule(validate_form(&form(text.clone())), Rule::CharsText);
    if let ControlKind::Text(t) = &mut text.kind {
        t.flags = TextControl::URL | TextControl::MULTILINE;
    }
    assert!(validate_form(&form(text.clone())).is_ok());
    if let ControlKind::Text(t) = &mut text.kind {
        t.flags |= 4;
    }
    rule(validate_form(&form(text)), Rule::FlagsUndefined);
    let mut v = c();
    v.flags = 1;
    rule(validate_form(&form(v)), Rule::FlagsUndefined);
    let mut f = form(c());
    f.sections[0].flags = 2;
    rule(validate_form(&f), Rule::FlagsUndefined);
    rule(
        validate_form(&form(Control::select(
            "s",
            "S",
            OptionSource::Catalog(OptionCatalog {
                id: "catalog".into(),
                flags: 2,
            }),
            None,
        ))),
        Rule::FlagsUndefined,
    );
    rule(
        validate_capabilities(&SearchCapabilities {
            flags: 16,
            sort: None,
            sections: vec![],
        }),
        Rule::FlagsUndefined,
    );
    let mut text = Control::text("t", "Text", "a");
    if let ControlKind::Text(t) = &mut text.kind {
        t.default_value = "a".repeat(65_537);
    }
    rule(validate_form(&form(text)), Rule::LengthBody);
}
#[test]
fn defaults_selection_caps_bounds_and_order_are_validated() {
    for (n, ok) in [(1, true), (256, true), (0, false), (257, false)] {
        let v = Control::select(
            "s",
            "Select",
            OptionSource::catalog("cat"),
            Some("a".repeat(n)),
        );
        if ok {
            assert!(validate_form(&form(v)).is_ok());
        } else {
            rule(validate_form(&form(v)), Rule::LengthDefault);
        }
    }
    let mut multi = Control::multi_select(
        "m",
        "Multi",
        OptionSource::catalog("cat"),
        (0..64).map(|n| format!("v{n}")).collect(),
    );
    assert!(validate_form(&form(multi.clone())).is_ok());
    if let ControlKind::MultiSelect(m) = &mut multi.kind {
        m.default_value.push("extra".into());
    }
    rule(validate_form(&form(multi)), Rule::CountDefaultValues);
    for cap in [0, 501] {
        let mut v = Control::multi_select("m", "M", options(1), vec![]);
        if let ControlKind::MultiSelect(m) = &mut v.kind {
            m.max_selected = Some(cap);
        }
        rule(validate_form(&form(v)), Rule::ControlMaxSelected);
    }
    for cap in [1, 500] {
        let mut v = Control::multi_select("m", "M", options(1), vec!["v0".into()]);
        if let ControlKind::MultiSelect(m) = &mut v.kind {
            m.max_selected = Some(cap);
        }
        assert!(validate_form(&form(v)).is_ok());
    }
    let mut v = Control::multi_select("m", "M", options(2), vec!["v0".into(), "v1".into()]);
    if let ControlKind::MultiSelect(m) = &mut v.kind {
        m.max_selected = Some(1);
    }
    rule(validate_form(&form(v)), Rule::ControlMaxSelected);
    rule(
        validate_form(&form(Control::multi_select(
            "m",
            "M",
            options(1),
            vec!["missing".into()],
        ))),
        Rule::ControlDefault,
    );
    rule(
        validate_form(&form(Control::multi_select(
            "m",
            "M",
            options(1),
            vec!["v0".into(), "v0".into()],
        ))),
        Rule::DuplicateValue,
    );
    for default in [0, 10] {
        let mut v = Control::integer("i", "I", 0, 10);
        if let ControlKind::Integer(i) = &mut v.kind {
            i.default_value = Some(default);
        }
        assert!(validate_form(&form(v)).is_ok());
    }
    for default in [-1, 11] {
        let mut v = Control::integer("i", "I", 0, 10);
        if let ControlKind::Integer(i) = &mut v.kind {
            i.default_value = Some(default);
        }
        rule(validate_form(&form(v)), Rule::ControlDefault);
    }
    rule(
        validate_form(&form(Control::integer("i", "I", 10, 0))),
        Rule::ControlBounds,
    );
    for (lower, upper, ok) in [
        (None, None, true),
        (Some(0), Some(10), true),
        (Some(5), Some(5), true),
        (Some(6), Some(5), false),
        (Some(-1), None, false),
        (None, Some(11), false),
    ] {
        let mut v = Control::integer_range("r", "R", 0, 10);
        if let ControlKind::IntegerRange(r) = &mut v.kind {
            r.default_lower = lower;
            r.default_upper = upper;
        }
        if ok {
            assert!(validate_form(&form(v)).is_ok());
        } else {
            rule(validate_form(&form(v)), Rule::ControlDefault);
        }
    }
    rule(
        validate_form(&form(Control::integer_range("r", "R", 1, 0))),
        Rule::ControlBounds,
    );
}
#[test]
fn conditions_counts_targets_and_values_are_two_pass_document_order() {
    let target = Control::toggle("target", "Target", false);
    for n in [4, 5] {
        let mut child = c();
        child.visible_when = vec![
            ControlCondition {
                control: "target".into(),
                any_of: vec!["true".into()]
            };
            n
        ];
        let mut f = form(target.clone());
        f.sections[0].controls.push(child);
        if n == 4 {
            assert!(validate_form(&f).is_ok());
        } else {
            rule(validate_form(&f), Rule::CountConditions);
        }
    }
    for n in [0, 1, 16, 17] {
        let child = c().visible_when("target", (0..n).map(|n| format!("v{n}")).collect());
        let mut f = form(Control::select(
            "target",
            "Target",
            OptionSource::catalog("catalog"),
            None,
        ));
        f.sections[0].controls.push(child);
        if (1..=16).contains(&n) {
            assert!(validate_form(&f).is_ok());
        } else {
            rule(validate_form(&f), Rule::CountConditionValues);
        }
    }
    for target_id in ["missing", "control"] {
        rule(
            validate_form(&form(c().visible_when(target_id, vec!["true".into()]))),
            Rule::ConditionTarget,
        );
    }
    let mut f = form(target.clone());
    f.sections[0]
        .controls
        .push(c().visible_when("target", vec!["yes".into()]));
    rule(validate_form(&f), Rule::ConditionValue);
    let mut f = form(target.clone());
    f.sections[0]
        .controls
        .push(c().visible_when("target", vec!["true".into(), "true".into()]));
    rule(validate_form(&f), Rule::DuplicateValue);
    let mut f = form(Control::integer("target", "Target", 0, 10));
    f.sections[0]
        .controls
        .push(c().visible_when("target", vec!["1".into()]));
    rule(validate_form(&f), Rule::ConditionTarget);
    let mut f = form(target);
    f.sections[0]
        .controls
        .push(c().visible_when("bad id", vec!["true".into()]));
    let err = validate_form(&f).unwrap_err();
    assert_eq!(err.rule, Rule::CharsId);
    assert_eq!(err.id, None);
    let mut f = form(c().visible_when("missing", vec!["true".into()]));
    f.sections[0]
        .controls
        .push(Control::toggle("later", "", false));
    rule(validate_form(&f), Rule::LengthTitle);
}
#[test]
fn request_page_and_suggestion_relationships_are_checked() {
    for limit in [1, 100] {
        let mut r = request();
        r.limit = limit;
        assert!(validate_request(&r).is_ok());
    }
    for limit in [0, 101] {
        let mut r = request();
        r.limit = limit;
        rule(validate_request(&r), Rule::RequestInvalid);
    }
    for n in [0, 256, 257] {
        let mut r = request();
        r.query = OptionQuery::Search {
            text: "x".repeat(n),
            offset: u32::MAX,
        };
        if n <= 256 {
            assert!(validate_request(&r).is_ok());
        } else {
            rule(validate_request(&r), Rule::RequestInvalid);
        }
    }
    for n in [1, 100, 101] {
        let r = OptionRequest {
            catalog: "tags".into(),
            query: OptionQuery::Values((0..n).map(|n| format!("v{n}")).collect()),
            limit: n,
        };
        if n <= 100 {
            assert!(validate_request(&r).is_ok());
        } else {
            rule(validate_request(&r), Rule::RequestInvalid);
        }
    }
    for values in [
        vec![],
        vec!["a".into(), "a".into()],
        vec!["".into()],
        vec!["\n".into()],
    ] {
        let r = OptionRequest {
            catalog: "tags".into(),
            limit: values.len() as u32,
            query: OptionQuery::Values(values),
        };
        rule(validate_request(&r), Rule::RequestInvalid);
    }
    let mut p = page();
    p.options = (0..100)
        .map(|n| ChoiceOption::new(format!("v{n}"), "V"))
        .collect();
    assert!(validate_page(&p, &request()).is_ok());
    p.options.push(ChoiceOption::new("extra", "V"));
    rule(validate_page(&p, &request()), Rule::CountPageOptions);
    for next in [None, Some(6), Some(u32::MAX)] {
        let mut p = page();
        p.next_offset = next;
        assert!(validate_page(&p, &request()).is_ok());
    }
    for next in [0, 5] {
        let mut p = page();
        p.next_offset = Some(next);
        rule(validate_page(&p, &request()), Rule::PageProgress);
    }
    let r = OptionRequest {
        catalog: "tags".into(),
        query: OptionQuery::Values(vec!["a".into()]),
        limit: 1,
    };
    let mut p = page();
    p.next_offset = None;
    assert!(validate_page(&p, &r).is_ok());
    p.next_offset = Some(6);
    rule(validate_page(&p, &r), Rule::PageValues);
    p.next_offset = None;
    p.options[0].value = "unknown".into();
    rule(validate_page(&p, &r), Rule::PageValues);
    let mut p = page();
    p.options[0].group = Some("".into());
    rule(validate_page(&p, &request()), Rule::LengthGroup);
    for (n, ok) in [(1, true), (64, true), (0, false), (65, false)] {
        let mut p = page();
        p.revision = Some("!".repeat(n));
        if ok {
            assert!(validate_page(&p, &request()).is_ok());
        } else {
            rule(validate_page(&p, &request()), Rule::LengthRevision);
        }
    }
    let mut p = page();
    p.revision = Some("has space".into());
    rule(validate_page(&p, &request()), Rule::CharsText);
    let make = |n| {
        vec![
            SearchSuggestion {
                query: "Q".into(),
                subtitle: None
            };
            n
        ]
    };
    assert!(validate_suggestions(&make(20)).is_ok());
    rule(validate_suggestions(&make(21)), Rule::CountSuggestions);
    for (n, ok) in [(1, true), (256, true), (0, false), (257, false)] {
        let v = vec![SearchSuggestion {
            query: "q".repeat(n),
            subtitle: None,
        }];
        if ok {
            assert!(validate_suggestions(&v).is_ok());
        } else {
            rule(validate_suggestions(&v), Rule::LengthQuery);
        }
        let v = vec![SearchSuggestion {
            query: "Q".into(),
            subtitle: Some("s".repeat(n)),
        }];
        if ok {
            assert!(validate_suggestions(&v).is_ok());
        } else {
            rule(validate_suggestions(&v), Rule::LengthSubtitle);
        }
    }
    rule(
        validate_suggestions(&[SearchSuggestion {
            query: "Q\n".into(),
            subtitle: None,
        }]),
        Rule::CharsText,
    );
}

#[test]
fn host_query_and_values_requests_use_byte_limits_and_exact_counts() {
    for (n, valid) in [(0, false), (1, true), (256, true), (257, false)] {
        let result = limits::validate_suggestion_query(&"x".repeat(n));
        if valid {
            assert!(result.is_ok());
        } else {
            rule(result, Rule::RequestInvalid);
        }
    }
    for (value, valid) in [("é".repeat(128), true), ("é".repeat(129), false)] {
        let r = OptionRequest {
            catalog: "tags".into(),
            query: OptionQuery::Values(vec![value]),
            limit: 1,
        };
        if valid {
            assert!(validate_request(&r).is_ok());
        } else {
            rule(validate_request(&r), Rule::RequestInvalid);
        }
    }
    let r = OptionRequest {
        catalog: "tags".into(),
        query: OptionQuery::Values(vec!["a".into(), "b".into()]),
        limit: 1,
    };
    rule(validate_request(&r), Rule::RequestInvalid);
    let r = OptionRequest {
        catalog: "unsafe id".into(),
        query: OptionQuery::Search {
            text: String::new(),
            offset: 0,
        },
        limit: 1,
    };
    let error = validate_request(&r).unwrap_err();
    assert_eq!(error.rule, Rule::RequestInvalid);
    assert_eq!(error.id, None);
}
