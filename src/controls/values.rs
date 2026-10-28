//! Effective settings and search values, with visibility applied after defaults.
use super::{
    types::*,
    value::{ControlValue, filter_item},
};
pub use crate::models::SettingIssue;
use crate::{
    defaults,
    models::{FilterItem, FilterValue},
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ControlValues {
    values: HashMap<String, ControlValue>,
    controls: HashMap<String, ControlKind>,
    visible: HashSet<String>,
    host_provided: bool,
    issues: Vec<SettingIssue>,
}
impl ControlValues {
    pub fn settings(form: &SettingsForm) -> Self {
        Self::settings_with(form, |key| defaults::get(key).ok().flatten())
    }
    /// Reads plain controls only. Secrets are read on demand by [`Self::secret`].
    pub fn settings_with(form: &SettingsForm, get: impl Fn(&str) -> Option<String>) -> Self {
        let all: Vec<_> = form.sections.iter().flat_map(|s| &s.controls).collect();
        let mut out = Self::new(true);
        for c in &all {
            out.controls.insert(c.id.clone(), c.kind.clone());
            if matches!(c.kind, ControlKind::Secret(_)) {
                continue;
            }
            let value = match get(&c.id) {
                Some(raw) => match c.kind.parse_value(&raw) {
                    Ok(v) => Some(v),
                    Err(_) => {
                        out.issues.push(SettingIssue {
                            key: c.id.clone(),
                            message: "Saved value isn't valid; the source uses the default".into(),
                        });
                        c.kind.default_value()
                    }
                },
                None => c.kind.default_value(),
            };
            if let Some(v) = value {
                out.values.insert(c.id.clone(), v);
            }
        }
        out.apply_visibility(&all);
        out
    }
    /// Unknown names are ignored; malformed or duplicate declared filters are errors.
    pub fn search(cap: &SearchCapabilities, filters: &[FilterItem]) -> crate::Result<Self> {
        let all: Vec<_> = cap
            .sort
            .iter()
            .chain(cap.sections.iter().flat_map(|s| &s.controls))
            .collect();
        let mut out = Self::new(!filters.is_empty());
        let mut supplied = HashMap::new();
        for f in filters {
            if let Some(c) = all.iter().find(|c| c.id == f.name) {
                if supplied.insert(c.id.as_str(), f).is_some() {
                    return Err(host("duplicate rich control filter"));
                }
                if matches!(c.kind, ControlKind::Secret(_)) || f.type_name != c.kind.wire_name() {
                    return Err(host("rich control filter kind mismatch"));
                }
            }
        }
        for c in &all {
            out.controls.insert(c.id.clone(), c.kind.clone());
            let value = match supplied.get(c.id.as_str()) {
                Some(item) => {
                    let FilterValue::String(raw) = &item.value else {
                        return Err(host("rich control filter is not a string"));
                    };
                    Some(
                        c.kind
                            .parse_value(raw)
                            .map_err(|_| host("invalid rich control filter"))?,
                    )
                }
                None => c.kind.default_value(),
            };
            if let Some(v) = value {
                out.values.insert(c.id.clone(), v);
            }
        }
        out.apply_visibility(&all);
        Ok(out)
    }
    fn new(host_provided: bool) -> Self {
        Self {
            values: HashMap::new(),
            controls: HashMap::new(),
            visible: HashSet::new(),
            host_provided,
            issues: Vec::new(),
        }
    }
    fn apply_visibility(&mut self, all: &[&Control]) {
        self.visible = visible(all.iter().copied(), &self.values);
        for c in all {
            if !self.visible.contains(&c.id)
                && let Some(v) = c.kind.default_value()
            {
                self.values.insert(c.id.clone(), v);
            }
        }
    }
    pub fn is_host_provided(&self) -> bool {
        self.host_provided
    }
    fn value(&self, id: &str) -> crate::Result<(&ControlKind, &ControlValue)> {
        let kind = self
            .controls
            .get(id)
            .ok_or_else(|| host("unknown rich control id"))?;
        let value = self
            .values
            .get(id)
            .ok_or_else(|| host("rich control has no value"))?;
        Ok((kind, value))
    }
    pub fn toggle(&self, id: &str) -> crate::Result<bool> {
        match self.value(id)? {
            (ControlKind::Toggle(_), ControlValue::Toggle(v)) => Ok(*v),
            _ => Err(host("rich control kind mismatch")),
        }
    }
    pub fn text(&self, id: &str) -> crate::Result<&str> {
        match self.value(id)? {
            (ControlKind::Text(_), ControlValue::Text(v)) => Ok(v),
            _ => Err(host("rich control kind mismatch")),
        }
    }
    pub fn select(&self, id: &str) -> crate::Result<Option<&str>> {
        match self.value(id)? {
            (ControlKind::Select(_), ControlValue::Select(v)) => Ok(v.as_deref()),
            _ => Err(host("rich control kind mismatch")),
        }
    }
    pub fn multi_select(&self, id: &str) -> crate::Result<&[String]> {
        match self.value(id)? {
            (ControlKind::MultiSelect(_), ControlValue::MultiSelect(v)) => Ok(v),
            _ => Err(host("rich control kind mismatch")),
        }
    }
    pub fn integer(&self, id: &str) -> crate::Result<Option<i64>> {
        match self.value(id)? {
            (ControlKind::Integer(_), ControlValue::Integer(v)) => Ok(*v),
            _ => Err(host("rich control kind mismatch")),
        }
    }
    pub fn integer_range(&self, id: &str) -> crate::Result<(Option<i64>, Option<i64>)> {
        match self.value(id)? {
            (ControlKind::IntegerRange(_), ControlValue::IntegerRange(a, b)) => Ok((*a, *b)),
            _ => Err(host("rich control kind mismatch")),
        }
    }
    /// Reads the credential path only when a declared secret is visible.
    pub fn secret(&self, id: &str) -> crate::Result<Option<String>> {
        match self.controls.get(id) {
            Some(ControlKind::Secret(_)) if self.visible.contains(id) => defaults::get(id),
            Some(ControlKind::Secret(_)) => Ok(None),
            Some(_) => Err(host("rich control kind mismatch")),
            None => Err(host("unknown rich control id")),
        }
    }
    pub fn issues(&self) -> &[SettingIssue] {
        &self.issues
    }
}
fn host(message: &str) -> crate::Error {
    crate::Error::Host(message.into())
}
fn visible<'a>(
    controls: impl Iterator<Item = &'a Control>,
    values: &HashMap<String, ControlValue>,
) -> HashSet<String> {
    controls
        .filter(|c| {
            c.visible_when
                .iter()
                .all(|cond| match values.get(&cond.control) {
                    Some(ControlValue::Toggle(v)) => cond
                        .any_of
                        .iter()
                        .any(|s| s == if *v { "true" } else { "false" }),
                    Some(ControlValue::Select(Some(v))) => cond.any_of.contains(v),
                    Some(ControlValue::MultiSelect(v)) => v.iter().any(|v| cond.any_of.contains(v)),
                    None => true,
                    _ => false,
                })
        })
        .map(|c| c.id.clone())
        .collect()
}
impl SettingsForm {
    pub fn controls(&self) -> Vec<Control> {
        self.sections
            .iter()
            .flat_map(|s| s.controls.iter().cloned())
            .collect()
    }
    pub fn visible_control_ids(&self, values: &ControlValues) -> HashSet<String> {
        values.visible.clone()
    }
}
impl SearchCapabilities {
    pub fn controls(&self) -> Vec<Control> {
        self.sort
            .iter()
            .cloned()
            .chain(
                self.sections
                    .iter()
                    .flat_map(|s| s.controls.iter().cloned()),
            )
            .collect()
    }
    pub fn visible_control_ids(&self, values: &HashMap<String, ControlValue>) -> HashSet<String> {
        visible(
            self.sort
                .iter()
                .chain(self.sections.iter().flat_map(|s| &s.controls)),
            values,
        )
    }
    /// Builds dense transport in document order, including defaults for hidden controls.
    pub fn filter_items(&self, values: &HashMap<String, ControlValue>) -> Vec<FilterItem> {
        let all: Vec<_> = self
            .sort
            .iter()
            .chain(self.sections.iter().flat_map(|s| &s.controls))
            .collect();
        let effective: HashMap<_, _> = all
            .iter()
            .filter_map(|c| {
                values
                    .get(&c.id)
                    .filter(|v| c.kind.accepts(v))
                    .cloned()
                    .or_else(|| c.kind.default_value())
                    .map(|v| (c.id.clone(), v))
            })
            .collect();
        let shown = visible(all.iter().copied(), &effective);
        all.into_iter()
            .filter_map(|c| {
                if matches!(c.kind, ControlKind::Secret(_)) {
                    return None;
                }
                if shown.contains(&c.id) {
                    effective.get(&c.id).map(|v| filter_item(c, v))
                } else {
                    c.kind.default_value().map(|v| filter_item(c, &v))
                }
            })
            .collect()
    }
}
/// Writes a checked canonical plain setting. Credentials must use the host secret API.
pub fn write_setting(control: &Control, value: &ControlValue) -> crate::Result<()> {
    if matches!(control.kind, ControlKind::Secret(_)) {
        return Err(host("secret controls cannot be written as plain settings"));
    }
    if !control.kind.accepts(value) {
        return Err(host("value does not match control"));
    }
    defaults::set(&control.id, &value.canonical());
    Ok(())
}
