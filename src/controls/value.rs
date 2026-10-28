//! Canonical strings shared by settings storage and dense search filters.
use super::types::*;
use crate::models::{FilterItem, FilterValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlValue {
    Toggle(bool),
    Text(String),
    Select(Option<String>),
    MultiSelect(Vec<String>),
    Integer(Option<i64>),
    IntegerRange(Option<i64>, Option<i64>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueError;
impl std::fmt::Display for ValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid canonical control value")
    }
}
impl std::error::Error for ValueError {}
impl ControlValue {
    pub fn canonical(&self) -> String {
        match self {
            Self::Toggle(v) => v.to_string(),
            Self::Text(v) => v.clone(),
            Self::Select(v) => v.clone().unwrap_or_default(),
            Self::Integer(v) => v.map(|n| n.to_string()).unwrap_or_default(),
            Self::MultiSelect(v) => {
                let mut out = String::from("[");
                for (i, s) in v.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    quote(s, &mut out);
                }
                out.push(']');
                out
            }
            Self::IntegerRange(a, b) => format!("[{},{}]", bound(*a), bound(*b)),
        }
    }
}
fn bound(v: Option<i64>) -> String {
    v.map(|n| n.to_string()).unwrap_or_else(|| "null".into())
}
fn quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c <= '\u{1f}' => {
                const HEX: &[u8; 16] = b"0123456789abcdef";
                let n = c as usize;
                out.push_str("\\u00");
                out.push(HEX[n >> 4] as char);
                out.push(HEX[n & 15] as char);
            }
            _ => out.push(c),
        }
    }
    out.push('"');
}
fn hex(b: u8) -> Result<u8, ValueError> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        _ => Err(ValueError),
    }
}
fn parse_strings(s: &str, max_items: usize) -> Result<Vec<String>, ValueError> {
    if s == "[]" {
        return Ok(Vec::new());
    }
    if !s.starts_with('[') || !s.ends_with(']') {
        return Err(ValueError);
    }
    let b = s.as_bytes();
    let mut i = 1;
    let end = b.len() - 1;
    let mut out = Vec::new();
    while i < end {
        if out.len() == max_items {
            return Err(ValueError);
        }
        if b[i] != b'"' {
            return Err(ValueError);
        }
        i += 1;
        let mut v = String::new();
        while i < end && b[i] != b'"' {
            if b[i] == b'\\' {
                i += 1;
                if i >= end {
                    return Err(ValueError);
                }
                match b[i] {
                    b'"' => v.push('"'),
                    b'\\' => v.push('\\'),
                    b'u' if i + 4 < end && b[i + 1] == b'0' && b[i + 2] == b'0' => {
                        let n = hex(b[i + 3])? * 16 + hex(b[i + 4])?;
                        if n > 31 {
                            return Err(ValueError);
                        }
                        v.push(char::from(n));
                        i += 4;
                    }
                    _ => return Err(ValueError),
                }
                i += 1;
            } else {
                if b[i] < 32 {
                    return Err(ValueError);
                }
                let c = s[i..].chars().next().ok_or(ValueError)?;
                v.push(c);
                i += c.len_utf8();
            }
            if v.len() > 256 {
                return Err(ValueError);
            }
        }
        if i >= end {
            return Err(ValueError);
        }
        i += 1;
        if v.is_empty() || v.len() > 256 || out.contains(&v) {
            return Err(ValueError);
        }
        out.push(v);
        if i < end {
            if b[i] != b',' {
                return Err(ValueError);
            }
            i += 1;
            if i == end {
                return Err(ValueError);
            }
        }
    }
    Ok(out)
}
fn parse_int(s: &str) -> Result<Option<i64>, ValueError> {
    if s.is_empty() {
        return Ok(None);
    }
    let digits = s.strip_prefix('-').unwrap_or(s);
    if digits.is_empty()
        || !digits.bytes().all(|b| b.is_ascii_digit())
        || (digits.len() > 1 && digits.starts_with('0'))
        || s == "-0"
    {
        return Err(ValueError);
    }
    s.parse().map(Some).map_err(|_| ValueError)
}
fn parse_bound(s: &str) -> Result<Option<i64>, ValueError> {
    if s == "null" {
        Ok(None)
    } else {
        parse_int(s)?.map(Some).ok_or(ValueError)
    }
}
impl ControlKind {
    pub fn default_value(&self) -> Option<ControlValue> {
        match self {
            Self::Toggle(v) => Some(ControlValue::Toggle(v.default_value)),
            Self::Text(v) => Some(ControlValue::Text(v.default_value.clone())),
            Self::Secret(_) => None,
            Self::Select(v) => Some(ControlValue::Select(v.default_value.clone())),
            Self::MultiSelect(v) => Some(ControlValue::MultiSelect(v.default_value.clone())),
            Self::Integer(v) => Some(ControlValue::Integer(v.default_value)),
            Self::IntegerRange(v) => {
                Some(ControlValue::IntegerRange(v.default_lower, v.default_upper))
            }
        }
    }
    /// Parses canonical runtime values and bounds, without checking inline option membership.
    pub fn parse_value(&self, s: &str) -> Result<ControlValue, ValueError> {
        Ok(match self {
            Self::Toggle(_) => ControlValue::Toggle(match s {
                "true" => true,
                "false" => false,
                _ => return Err(ValueError),
            }),
            Self::Text(c) => {
                if s.len() > 4096
                    || !super::limits::human_chars(s, c.flags & TextControl::MULTILINE != 0)
                {
                    return Err(ValueError);
                }
                ControlValue::Text(s.to_owned())
            }
            Self::Secret(_) => return Err(ValueError),
            Self::Select(_) => {
                if s.len() > 256 || !super::limits::human_chars(s, false) {
                    return Err(ValueError);
                }
                ControlValue::Select((!s.is_empty()).then(|| s.to_owned()))
            }
            Self::MultiSelect(c) => {
                let values = parse_strings(s, c.max_selected.unwrap_or(500).min(500) as usize)?;
                if values.iter().any(|v| !super::limits::human_chars(v, false)) {
                    return Err(ValueError);
                }
                ControlValue::MultiSelect(values)
            }
            Self::Integer(c) => {
                let n = parse_int(s)?;
                if n.is_some_and(|n| n < c.min || n > c.max) {
                    return Err(ValueError);
                }
                ControlValue::Integer(n)
            }
            Self::IntegerRange(c) => {
                let inner = s
                    .strip_prefix('[')
                    .and_then(|s| s.strip_suffix(']'))
                    .ok_or(ValueError)?;
                let (a, b) = inner.split_once(',').ok_or(ValueError)?;
                let (a, b) = (parse_bound(a)?, parse_bound(b)?);
                if a.is_some_and(|n| n < c.min || n > c.max)
                    || b.is_some_and(|n| n < c.min || n > c.max)
                    || a.zip(b).is_some_and(|(a, b)| a > b)
                {
                    return Err(ValueError);
                }
                ControlValue::IntegerRange(a, b)
            }
        })
    }
    /// Checks both the value variant and this declaration's constraints.
    pub fn accepts(&self, value: &ControlValue) -> bool {
        self.parse_value(&value.canonical())
            .is_ok_and(|parsed| parsed == *value)
    }
}
pub(crate) fn filter_item(control: &Control, value: &ControlValue) -> FilterItem {
    FilterItem {
        type_name: control.kind.wire_name().into(),
        name: control.id.clone(),
        value: FilterValue::String(value.canonical()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_string_syntax_can_round_trip_runtime_invalid_controls() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../fixtures/controls-v1.json")).unwrap();
        for entry in fixture["values"].as_array().unwrap() {
            if entry["runtime_invalid"] != true {
                continue;
            }
            let raw = entry["canonical"].as_str().unwrap();
            let strings = parse_strings(raw, 500).expect("valid canonical string syntax");
            let expected: Vec<String> = entry["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect();
            assert_eq!(strings, expected, "syntax fixture {entry}");
            assert_eq!(ControlValue::MultiSelect(strings).canonical(), raw);
            let kind = Control::multi_select("m", "Multi", OptionSource::catalog("c"), vec![]).kind;
            assert!(kind.parse_value(raw).is_err(), "runtime accepted {raw:?}");
        }
    }
}
