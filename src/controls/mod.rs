//! Rich settings, option catalogs, and search filters for compatible hosts.
//!
//! These controls require ito-runner with rich-controls support and the corresponding Ito host.
//! Stored settings use canonical strings; secrets remain in the host credential store. Keep legacy
//! `get_settings` implementations where compatibility with older hosts is required.
//!
//! # Defaults and compatibility
//!
//! Rich forms do not inject defaults into host storage. Read them through [`ControlValues`]:
//! an absent or malformed plain setting uses the declaration's default, and hidden controls
//! also apply their defaults without deleting saved values. [`ControlValues::issues`] reports
//! malformed saved values using fixed messages. Secret controls are never parsed as plain
//! settings; [`ControlValues::secret`] reads a visible credential on demand.
//!
//! ```
//! use ito_rs::controls::{Control, ControlSection, ControlValues, SettingsForm};
//!
//! let form = SettingsForm::new(vec![
//!     ControlSection::new("content", "Content")
//!         .control(Control::toggle("adult", "Include adult content", false)),
//! ]);
//! // Native tests can inject storage without importing the host defaults module.
//! let values = ControlValues::settings_with(&form, |_| None);
//! assert!(!values.toggle("adult")?);
//! # Ok::<(), ito_rs::Error>(())
//! ```
//!
//! Search filters reuse the existing `FilterItem` string variant in document order.
//! [`ControlValues::search`] validates transported values, ignores undeclared names and
//! applies visibility. An empty filter list means the host did not provide a capabilities
//! view; [`ControlValues::is_host_provided`] lets a source keep its legacy search behavior.
//! Inline option membership is deliberately not a parsing constraint: removed options
//! remain selected by stable identity until the user removes them.
//!
//! # Catalogs and validation
//!
//! Use [`OptionSource::searchable_catalog`] for large option sets. Search requests use
//! stable offset paging; Values requests resolve the labels of selected identities.
//! Return only requested values in Values mode, never a next offset. A changed catalog
//! ordering should change [`OptionPage::revision`]. Option values are plain persisted
//! identifiers and must never contain credentials or tokenized URLs.
//!
//! The four optional exports validate responses before packing and reject malformed
//! requests before invoking source code. [`limits`] exposes the shared budgets and
//! deterministic validators for native authoring tests. Unknown control tags cannot be
//! authored by this SDK; compatible hosts can preserve them as unavailable controls.
//! A complete paged catalog and filter-aware provider lives in `examples/rich-settings`.
#[doc(hidden)]
pub mod export;
pub mod limits;
mod types;
mod value;
mod values;
#[doc(hidden)]
pub use export::{
    validate_capabilities_result, validate_form_result, validate_page_result,
    validate_suggestions_result,
};
pub use limits::{
    Rule, Violation, validate_capabilities, validate_form, validate_page, validate_request,
    validate_suggestions,
};
pub use types::*;
pub use value::{ControlValue, ValueError};
pub use values::{ControlValues, SettingIssue, write_setting};
