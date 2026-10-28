use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SettingsForm {
    pub sections: Vec<ControlSection>,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ControlSection {
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
    pub flags: u32,
    pub controls: Vec<Control>,
}
impl ControlSection {
    pub const PAGE: u32 = 1;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Control {
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
    pub flags: u32,
    pub visible_when: Vec<ControlCondition>,
    pub kind: ControlKind,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ControlCondition {
    pub control: String,
    pub any_of: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ControlKind {
    Toggle(ToggleControl),
    Text(TextControl),
    Secret(SecretControl),
    Select(SelectControl),
    MultiSelect(MultiSelectControl),
    Integer(IntegerControl),
    IntegerRange(IntegerRangeControl),
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ToggleControl {
    pub default_value: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TextControl {
    pub default_value: String,
    pub placeholder: Option<String>,
    pub flags: u32,
}
impl TextControl {
    pub const URL: u32 = 1;
    pub const MULTILINE: u32 = 2;
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SecretControl {
    pub placeholder: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SelectControl {
    pub options: OptionSource,
    pub default_value: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MultiSelectControl {
    pub options: OptionSource,
    pub default_value: Vec<String>,
    pub max_selected: Option<u32>,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct IntegerControl {
    pub min: i64,
    pub max: i64,
    pub default_value: Option<i64>,
    pub unit: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct IntegerRangeControl {
    pub min: i64,
    pub max: i64,
    pub default_lower: Option<i64>,
    pub default_upper: Option<i64>,
    pub unit: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum OptionSource {
    Inline(Vec<ChoiceOption>),
    Catalog(OptionCatalog),
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct OptionCatalog {
    pub id: String,
    pub flags: u32,
}
impl OptionCatalog {
    pub const SEARCHABLE: u32 = 1;
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChoiceOption {
    pub value: String,
    pub label: String,
    pub group: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct OptionRequest {
    pub catalog: String,
    pub query: OptionQuery,
    pub limit: u32,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum OptionQuery {
    Search { text: String, offset: u32 },
    Values(Vec<String>),
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct OptionPage {
    pub options: Vec<ChoiceOption>,
    pub next_offset: Option<u32>,
    pub total: Option<u32>,
    pub revision: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SearchCapabilities {
    pub flags: u32,
    pub sort: Option<Control>,
    pub sections: Vec<ControlSection>,
}
impl SearchCapabilities {
    pub const BROWSE: u32 = 1;
    pub const SORT_WITH_QUERY: u32 = 2;
    pub const FILTERS_WITH_QUERY: u32 = 4;
    pub const SUGGESTIONS: u32 = 8;
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SearchSuggestion {
    pub query: String,
    pub subtitle: Option<String>,
}

impl ControlKind {
    pub const fn tag(&self) -> u32 {
        match self {
            Self::Toggle(_) => 0,
            Self::Text(_) => 1,
            Self::Secret(_) => 2,
            Self::Select(_) => 3,
            Self::MultiSelect(_) => 4,
            Self::Integer(_) => 5,
            Self::IntegerRange(_) => 6,
        }
    }
    pub const fn wire_name(&self) -> &'static str {
        match self {
            Self::Toggle(_) => "toggle",
            Self::Text(_) => "text",
            Self::Secret(_) => "secret",
            Self::Select(_) => "select",
            Self::MultiSelect(_) => "multi_select",
            Self::Integer(_) => "integer",
            Self::IntegerRange(_) => "integer_range",
        }
    }
    pub(crate) fn body(&self) -> Result<Vec<u8>, postcard::Error> {
        match self {
            Self::Toggle(v) => postcard::to_allocvec(v),
            Self::Text(v) => postcard::to_allocvec(v),
            Self::Secret(v) => postcard::to_allocvec(v),
            Self::Select(v) => postcard::to_allocvec(v),
            Self::MultiSelect(v) => postcard::to_allocvec(v),
            Self::Integer(v) => postcard::to_allocvec(v),
            Self::IntegerRange(v) => postcard::to_allocvec(v),
        }
    }
}
impl Serialize for Control {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let body = self.kind.body().map_err(serde::ser::Error::custom)?;
        (
            &self.id,
            &self.title,
            &self.summary,
            &self.flags,
            &self.visible_when,
            self.kind.tag(),
            serde_bytes::Bytes::new(&body),
        )
            .serialize(serializer)
    }
}
impl Serialize for ControlKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let body = self.body().map_err(serde::ser::Error::custom)?;
        (self.tag(), serde_bytes::Bytes::new(&body)).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for ControlKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (tag, body): (u32, serde_bytes::ByteBuf) = Deserialize::deserialize(deserializer)?;
        decode_kind(tag, &body)
    }
}
impl<'de> Deserialize<'de> for Control {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (id, title, summary, flags, visible_when, tag, body): (
            String,
            String,
            Option<String>,
            u32,
            Vec<ControlCondition>,
            u32,
            serde_bytes::ByteBuf,
        ) = Deserialize::deserialize(deserializer)?;
        let kind = decode_kind(tag, &body)?;
        Ok(Self {
            id,
            title,
            summary,
            flags,
            visible_when,
            kind,
        })
    }
}
fn decode_kind<E: serde::de::Error>(tag: u32, body: &[u8]) -> Result<ControlKind, E> {
    if body.len() > super::limits::MAX_BODY_BYTES {
        return Err(E::custom("length.body"));
    }
    match tag {
        0 => Ok(ControlKind::Toggle(decode(body)?)),
        1 => Ok(ControlKind::Text(decode(body)?)),
        2 => Ok(ControlKind::Secret(decode(body)?)),
        3 => Ok(ControlKind::Select(decode(body)?)),
        4 => Ok(ControlKind::MultiSelect(decode(body)?)),
        5 => Ok(ControlKind::Integer(decode(body)?)),
        6 => Ok(ControlKind::IntegerRange(decode(body)?)),
        _ => Err(E::custom("unknown control kind")),
    }
}
fn decode<T: serde::de::DeserializeOwned, E: serde::de::Error>(bytes: &[u8]) -> Result<T, E> {
    let (value, rest) = postcard::take_from_bytes(bytes).map_err(E::custom)?;
    if !rest.is_empty() {
        return Err(E::custom("trailing control body bytes"));
    }
    Ok(value)
}
impl Control {
    pub fn toggle(id: impl Into<String>, title: impl Into<String>, default: bool) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: None,
            flags: 0,
            visible_when: Vec::new(),
            kind: ControlKind::Toggle(ToggleControl {
                default_value: default,
            }),
        }
    }
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }
    pub fn visible_when(mut self, target: impl Into<String>, values: Vec<String>) -> Self {
        self.visible_when.push(ControlCondition {
            control: target.into(),
            any_of: values,
        });
        self
    }
}
impl OptionSource {
    pub fn inline(options: Vec<ChoiceOption>) -> Self {
        Self::Inline(options)
    }
    pub fn catalog(id: impl Into<String>) -> Self {
        Self::Catalog(OptionCatalog {
            id: id.into(),
            flags: 0,
        })
    }
    pub fn searchable_catalog(id: impl Into<String>) -> Self {
        Self::Catalog(OptionCatalog {
            id: id.into(),
            flags: OptionCatalog::SEARCHABLE,
        })
    }
}
impl ChoiceOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            group: None,
        }
    }
    pub fn grouped(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }
}
impl Control {
    pub fn text(
        id: impl Into<String>,
        title: impl Into<String>,
        default_value: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: None,
            flags: 0,
            visible_when: Vec::new(),
            kind: ControlKind::Text(TextControl {
                default_value: default_value.into(),
                placeholder: None,
                flags: 0,
            }),
        }
    }
    pub fn secret(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: None,
            flags: 0,
            visible_when: Vec::new(),
            kind: ControlKind::Secret(SecretControl { placeholder: None }),
        }
    }
    pub fn select(
        id: impl Into<String>,
        title: impl Into<String>,
        options: OptionSource,
        default_value: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: None,
            flags: 0,
            visible_when: Vec::new(),
            kind: ControlKind::Select(SelectControl {
                options,
                default_value,
            }),
        }
    }
    pub fn multi_select(
        id: impl Into<String>,
        title: impl Into<String>,
        options: OptionSource,
        default_value: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: None,
            flags: 0,
            visible_when: Vec::new(),
            kind: ControlKind::MultiSelect(MultiSelectControl {
                options,
                default_value,
                max_selected: None,
            }),
        }
    }
    pub fn integer(id: impl Into<String>, title: impl Into<String>, min: i64, max: i64) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: None,
            flags: 0,
            visible_when: Vec::new(),
            kind: ControlKind::Integer(IntegerControl {
                min,
                max,
                default_value: None,
                unit: None,
            }),
        }
    }
    pub fn integer_range(
        id: impl Into<String>,
        title: impl Into<String>,
        min: i64,
        max: i64,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: None,
            flags: 0,
            visible_when: Vec::new(),
            kind: ControlKind::IntegerRange(IntegerRangeControl {
                min,
                max,
                default_lower: None,
                default_upper: None,
                unit: None,
            }),
        }
    }
}
impl ControlSection {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: None,
            flags: 0,
            controls: Vec::new(),
        }
    }
    pub fn control(mut self, control: Control) -> Self {
        self.controls.push(control);
        self
    }
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }
}
impl SettingsForm {
    pub fn new(sections: Vec<ControlSection>) -> Self {
        Self { sections }
    }
}
impl SearchCapabilities {
    pub fn new() -> Self {
        Self {
            flags: 0,
            sort: None,
            sections: Vec::new(),
        }
    }
}

impl Default for SearchCapabilities {
    fn default() -> Self {
        Self::new()
    }
}
