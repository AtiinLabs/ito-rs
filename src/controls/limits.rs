//! Shared rich-control budgets and deterministic pre-pack validation.
use super::types::*;
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

pub const FORM_BYTES: usize = 262_144;
pub const CAPABILITIES_BYTES: usize = 262_144;
pub const OPTION_PAGE_BYTES: usize = 131_072;
pub const SUGGESTIONS_BYTES: usize = 16_384;
pub const MAX_SECTIONS: usize = 32;
pub const MAX_CONTROLS: usize = 256;
pub const MAX_SECTION_CONTROLS: usize = 64;
pub const MAX_CONDITIONS: usize = 4;
pub const MAX_BODY_BYTES: usize = 65_536;

macro_rules! rules {
    ($($variant:ident => $code:literal),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum Rule { $($variant),+ }
        impl Rule { pub const fn code(self) -> &'static str { match self { $(Self::$variant => $code),+ } } }
    };
}
rules! {
    RequestInvalid => "request.invalid", CountSections => "count.sections", CountControls => "count.controls",
    CountSectionControls => "count.section_controls", CountConditions => "count.conditions",
    CountConditionValues => "count.condition_values", CountOptions => "count.options", CountFormOptions => "count.form_options",
    CountDefaultValues => "count.default_values", CountPageOptions => "count.page_options", CountSuggestions => "count.suggestions",
    LengthId => "length.id", LengthTitle => "length.title", LengthSummary => "length.summary", LengthLabel => "length.label",
    LengthGroup => "length.group", LengthPlaceholder => "length.placeholder", LengthUnit => "length.unit",
    LengthDefault => "length.default", LengthValue => "length.value", LengthRevision => "length.revision",
    LengthQuery => "length.query", LengthSubtitle => "length.subtitle", LengthBody => "length.body",
    CharsId => "chars.id", CharsText => "chars.text", CharsValue => "chars.value",
    DuplicateSection => "duplicate.section", DuplicateControl => "duplicate.control", DuplicateOption => "duplicate.option",
    DuplicateValue => "duplicate.value", FlagsUndefined => "flags.undefined", ControlBody => "control.body",
    ControlBounds => "control.bounds", ControlDefault => "control.default", ControlMaxSelected => "control.max_selected",
    ControlContext => "control.context", ConditionTarget => "condition.target", ConditionValue => "condition.value",
    PageProgress => "page.progress", PageValues => "page.values"
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub rule: Rule,
    pub id: Option<String>,
}
impl Violation {
    pub fn new(rule: Rule) -> Self {
        Self { rule, id: None }
    }
    /// Includes only validated identifiers, never plugin values or human text.
    pub fn with_id(rule: Rule, id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            rule,
            id: valid_id(&id).then_some(id),
        }
    }
}
impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.id {
            Some(id) => write!(f, "{} ({id})", self.rule.code()),
            None => f.write_str(self.rule.code()),
        }
    }
}
impl std::error::Error for Violation {}
fn fail(rule: Rule, id: Option<&str>) -> Violation {
    Violation {
        rule,
        id: id.filter(|s| valid_id(s)).map(str::to_owned),
    }
}
fn valid_id(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
fn id(s: &str) -> Result<(), Violation> {
    if !(1..=64).contains(&s.len()) {
        return Err(Violation::new(Rule::LengthId));
    }
    if !valid_id(s) {
        return Err(Violation::new(Rule::CharsId));
    }
    Ok(())
}
fn text(
    s: &str,
    max: usize,
    rule: Rule,
    newline: bool,
    context: Option<&str>,
) -> Result<(), Violation> {
    if s.is_empty() || s.len() > max {
        return Err(fail(rule, context));
    }
    if !human_chars(s, newline) {
        return Err(fail(Rule::CharsText, context));
    }
    Ok(())
}
pub(crate) fn human_chars(s: &str, newline: bool) -> bool {
    s.chars()
        .all(|c| (!c.is_control()) || (newline && c == '\n'))
}
fn optional_text(
    s: &Option<String>,
    max: usize,
    rule: Rule,
    newline: bool,
    context: Option<&str>,
) -> Result<(), Violation> {
    if let Some(s) = s {
        text(s, max, rule, newline, context)?;
    }
    Ok(())
}
fn value(s: &str, rule: Rule, context: Option<&str>) -> Result<(), Violation> {
    if !(1..=256).contains(&s.len()) {
        return Err(fail(rule, context));
    }
    if !human_chars(s, false) {
        return Err(fail(Rule::CharsValue, context));
    }
    Ok(())
}
fn values(v: &[String], rule: Rule, context: Option<&str>) -> Result<(), Violation> {
    let mut seen = HashSet::new();
    for s in v {
        value(s, rule, context)?;
        if !seen.insert(s) {
            return Err(fail(Rule::DuplicateValue, context));
        }
    }
    Ok(())
}
fn choice(o: &ChoiceOption, context: Option<&str>) -> Result<(), Violation> {
    value(&o.value, Rule::LengthValue, context)?;
    text(&o.label, 256, Rule::LengthLabel, false, context)?;
    optional_text(&o.group, 128, Rule::LengthGroup, false, context)
}
fn source(s: &OptionSource, total: &mut usize, context: Option<&str>) -> Result<(), Violation> {
    match s {
        OptionSource::Inline(options) => {
            if options.is_empty() || options.len() > 256 {
                return Err(fail(Rule::CountOptions, context));
            }
            let mut seen = HashSet::new();
            for o in options {
                *total += 1;
                if *total > 2048 {
                    return Err(fail(Rule::CountFormOptions, context));
                }
                value(&o.value, Rule::LengthValue, context)?;
                if !seen.insert(&o.value) {
                    return Err(fail(Rule::DuplicateOption, context));
                }
                text(&o.label, 256, Rule::LengthLabel, false, context)?;
                optional_text(&o.group, 128, Rule::LengthGroup, false, context)?;
            }
        }
        OptionSource::Catalog(catalog) => {
            id(&catalog.id)?;
            if catalog.flags & !OptionCatalog::SEARCHABLE != 0 {
                return Err(fail(Rule::FlagsUndefined, Some(&catalog.id)));
            }
        }
    }
    Ok(())
}
fn contains(s: &OptionSource, v: &str) -> bool {
    match s {
        OptionSource::Inline(o) => o.iter().any(|o| o.value == v),
        OptionSource::Catalog(_) => true,
    }
}
/// Validates one known control's fields. Cross-control conditions are checked by form validation.
pub fn validate_control(c: &Control) -> Result<(), Violation> {
    control(c, true, &mut 0)
}
fn control(c: &Control, settings: bool, options: &mut usize) -> Result<(), Violation> {
    id(&c.id)?;
    let ctx = Some(c.id.as_str());
    text(&c.title, 256, Rule::LengthTitle, false, ctx)?;
    optional_text(&c.summary, 1024, Rule::LengthSummary, true, ctx)?;
    if c.flags != 0 {
        return Err(fail(Rule::FlagsUndefined, ctx));
    }
    if c.visible_when.len() > MAX_CONDITIONS {
        return Err(fail(Rule::CountConditions, ctx));
    }
    for condition in &c.visible_when {
        id(&condition.control)?;
        if !(1..=16).contains(&condition.any_of.len()) {
            return Err(fail(Rule::CountConditionValues, ctx));
        }
        values(&condition.any_of, Rule::LengthValue, ctx)?;
    }
    // Body length precedes its fields in the wire envelope.
    if c.kind
        .body()
        .map_err(|_| fail(Rule::ControlBody, ctx))?
        .len()
        > MAX_BODY_BYTES
    {
        return Err(fail(Rule::LengthBody, ctx));
    }
    match &c.kind {
        ControlKind::Toggle(_) => {}
        ControlKind::Text(t) => {
            if t.default_value.len() > 4096 {
                return Err(fail(Rule::LengthDefault, ctx));
            }
            if !human_chars(&t.default_value, t.flags & TextControl::MULTILINE != 0) {
                return Err(fail(Rule::CharsText, ctx));
            }
            optional_text(&t.placeholder, 128, Rule::LengthPlaceholder, false, ctx)?;
            if t.flags & !(TextControl::URL | TextControl::MULTILINE) != 0 {
                return Err(fail(Rule::FlagsUndefined, ctx));
            }
        }
        ControlKind::Secret(s) => {
            if !settings {
                return Err(fail(Rule::ControlContext, ctx));
            }
            optional_text(&s.placeholder, 128, Rule::LengthPlaceholder, false, ctx)?;
        }
        ControlKind::Select(s) => {
            source(&s.options, options, ctx)?;
            if let Some(v) = &s.default_value {
                value(v, Rule::LengthDefault, ctx)?;
                if !contains(&s.options, v) {
                    return Err(fail(Rule::ControlDefault, ctx));
                }
            }
        }
        ControlKind::MultiSelect(s) => {
            source(&s.options, options, ctx)?;
            if s.default_value.len() > 64 {
                return Err(fail(Rule::CountDefaultValues, ctx));
            }
            values(&s.default_value, Rule::LengthDefault, ctx)?;
            if s.default_value.iter().any(|v| !contains(&s.options, v)) {
                return Err(fail(Rule::ControlDefault, ctx));
            }
            if s.max_selected
                .is_some_and(|n| n == 0 || n > 500 || (n as usize) < s.default_value.len())
            {
                return Err(fail(Rule::ControlMaxSelected, ctx));
            }
        }
        ControlKind::Integer(s) => {
            if s.min > s.max {
                return Err(fail(Rule::ControlBounds, ctx));
            }
            if s.default_value.is_some_and(|n| n < s.min || n > s.max) {
                return Err(fail(Rule::ControlDefault, ctx));
            }
            optional_text(&s.unit, 32, Rule::LengthUnit, false, ctx)?;
        }
        ControlKind::IntegerRange(s) => {
            if s.min > s.max {
                return Err(fail(Rule::ControlBounds, ctx));
            }
            if [s.default_lower, s.default_upper]
                .into_iter()
                .flatten()
                .any(|n| n < s.min || n > s.max)
                || s.default_lower
                    .zip(s.default_upper)
                    .is_some_and(|(a, b)| a > b)
            {
                return Err(fail(Rule::ControlDefault, ctx));
            }
            optional_text(&s.unit, 32, Rule::LengthUnit, false, ctx)?;
        }
    }
    Ok(())
}
fn controls<'a>(
    sections: &'a [ControlSection],
    sort: Option<&'a Control>,
    settings: bool,
) -> Result<(), Violation> {
    let mut ids = HashMap::new();
    let mut section_ids = HashSet::new();
    let mut count = 0;
    let mut options = 0;
    let mut visit = |c: &'a Control| -> Result<(), Violation> {
        count += 1;
        if count > MAX_CONTROLS {
            return Err(Violation::new(Rule::CountControls));
        }
        id(&c.id)?;
        if ids.contains_key(c.id.as_str()) {
            return Err(fail(Rule::DuplicateControl, Some(&c.id)));
        }
        control(c, settings, &mut options)?;
        ids.insert(c.id.as_str(), c);
        Ok(())
    };
    if let Some(s) = sort {
        visit(s)?;
        if !s.visible_when.is_empty()
            || !matches!(
                &s.kind,
                ControlKind::Select(SelectControl {
                    options: OptionSource::Inline(_),
                    default_value: Some(_),
                    ..
                })
            )
        {
            return Err(fail(Rule::ControlContext, Some(&s.id)));
        }
    }
    if sections.len() > MAX_SECTIONS {
        return Err(Violation::new(Rule::CountSections));
    }
    for s in sections {
        id(&s.id)?;
        if !section_ids.insert(&s.id) {
            return Err(fail(Rule::DuplicateSection, Some(&s.id)));
        }
        text(&s.title, 256, Rule::LengthTitle, false, Some(&s.id))?;
        optional_text(&s.summary, 1024, Rule::LengthSummary, true, Some(&s.id))?;
        if s.flags & !ControlSection::PAGE != 0 {
            return Err(fail(Rule::FlagsUndefined, Some(&s.id)));
        }
        if s.controls.len() > MAX_SECTION_CONTROLS {
            return Err(fail(Rule::CountSectionControls, Some(&s.id)));
        }
        for c in &s.controls {
            visit(c)?;
        }
    }
    // Pass two follows document order, never HashMap iteration order.
    for c in sort
        .into_iter()
        .chain(sections.iter().flat_map(|s| &s.controls))
    {
        for condition in &c.visible_when {
            let target = ids
                .get(condition.control.as_str())
                .ok_or_else(|| fail(Rule::ConditionTarget, Some(&c.id)))?;
            if target.id == c.id
                || !target.visible_when.is_empty()
                || !matches!(
                    target.kind,
                    ControlKind::Toggle(_) | ControlKind::Select(_) | ControlKind::MultiSelect(_)
                )
            {
                return Err(fail(Rule::ConditionTarget, Some(&c.id)));
            }
            if matches!(target.kind, ControlKind::Toggle(_))
                && condition.any_of.iter().any(|s| s != "true" && s != "false")
            {
                return Err(fail(Rule::ConditionValue, Some(&c.id)));
            }
        }
    }
    Ok(())
}
pub fn validate_form(v: &SettingsForm) -> Result<(), Violation> {
    controls(&v.sections, None, true)
}
pub fn validate_capabilities(v: &SearchCapabilities) -> Result<(), Violation> {
    if v.flags & !15 != 0 {
        return Err(Violation::new(Rule::FlagsUndefined));
    }
    controls(&v.sections, v.sort.as_ref(), false)
}
pub fn validate_request(r: &OptionRequest) -> Result<(), Violation> {
    let valid = valid_id(&r.catalog)
        && match &r.query {
            OptionQuery::Search { text, .. } => text.len() <= 256 && (1..=100).contains(&r.limit),
            OptionQuery::Values(v) => {
                (1..=100).contains(&v.len())
                    && r.limit as usize == v.len()
                    && values(v, Rule::LengthValue, None).is_ok()
            }
        };
    if valid {
        Ok(())
    } else {
        Err(Violation::new(Rule::RequestInvalid))
    }
}
pub fn validate_suggestion_query(query: &str) -> Result<(), Violation> {
    if (1..=256).contains(&query.len()) {
        Ok(())
    } else {
        Err(Violation::new(Rule::RequestInvalid))
    }
}
pub fn validate_page(page: &OptionPage, request: &OptionRequest) -> Result<(), Violation> {
    validate_request(request)?;
    if page.options.len() > request.limit as usize {
        return Err(Violation::new(Rule::CountPageOptions));
    }
    let mut seen = HashSet::new();
    for o in &page.options {
        value(&o.value, Rule::LengthValue, None)?;
        if !seen.insert(&o.value) {
            return Err(Violation::new(Rule::DuplicateOption));
        }
        choice(o, None)?;
    }
    match &request.query {
        OptionQuery::Search { offset, .. } if page.next_offset.is_some_and(|n| n <= *offset) => {
            return Err(Violation::new(Rule::PageProgress));
        }
        OptionQuery::Values(v)
            if page.next_offset.is_some() || page.options.iter().any(|o| !v.contains(&o.value)) =>
        {
            return Err(Violation::new(Rule::PageValues));
        }
        _ => {}
    }
    if let Some(r) = &page.revision {
        if !(1..=64).contains(&r.len()) {
            return Err(Violation::new(Rule::LengthRevision));
        }
        if !r.bytes().all(|b| (0x21..=0x7e).contains(&b)) {
            return Err(Violation::new(Rule::CharsText));
        }
    }
    Ok(())
}
pub fn validate_suggestions(v: &[SearchSuggestion]) -> Result<(), Violation> {
    if v.len() > 20 {
        return Err(Violation::new(Rule::CountSuggestions));
    }
    for s in v {
        text(&s.query, 256, Rule::LengthQuery, false, None)?;
        optional_text(&s.subtitle, 256, Rule::LengthSubtitle, false, None)?;
    }
    Ok(())
}
