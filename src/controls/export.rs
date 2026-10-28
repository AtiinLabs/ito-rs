//! Validated dispatch behind the four optional rich-control exports.
use super::{limits::*, types::*};
use crate::{Error, Result, provider::SourceProvider};
use serde::{Serialize, de::DeserializeOwned};

#[doc(hidden)]
pub fn __encode<T: Serialize>(
    name: &str,
    result: Result<T>,
    validate: impl FnOnce(&T) -> std::result::Result<(), Violation>,
) -> Result<Vec<u8>> {
    let value = result?;
    validate(&value).map_err(|v| Error::Host(format!("{name}: invalid payload: {v}")))?;
    let bytes = postcard::to_allocvec(&value)?;
    let limit = match name {
        "get_settings_form" => FORM_BYTES,
        "get_search_capabilities" => CAPABILITIES_BYTES,
        "get_option_page" => OPTION_PAGE_BYTES,
        "get_search_suggestions" => SUGGESTIONS_BYTES,
        _ => return Err(Error::Host("unknown rich-control export".into())),
    };
    if bytes.len() > limit {
        return Err(Error::Host(format!(
            "{name}: invalid payload: response.too_large"
        )));
    }
    Ok(bytes)
}
#[doc(hidden)]
pub fn __pack<T: Serialize>(
    name: &str,
    result: Result<T>,
    validate: impl FnOnce(&T) -> std::result::Result<(), Violation>,
) -> i64 {
    match __encode(name, result, validate) {
        Ok(bytes) => {
            let bytes = bytes.into_boxed_slice();
            let ptr = bytes.as_ptr() as u64;
            let len = bytes.len() as u64;
            let _ = Box::into_raw(bytes);
            ((ptr << 32) | len) as i64
        }
        Err(error) => {
            crate::error::__report_failure(name, &error);
            0
        }
    }
}
#[doc(hidden)]
pub fn __decode_request<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let (value, rest) = postcard::take_from_bytes(bytes).map_err(|_| invalid_request())?;
    if !rest.is_empty() {
        return Err(invalid_request());
    }
    Ok(value)
}
fn invalid_request() -> Error {
    Error::Host("request.invalid".into())
}
/// Keeps malformed requests away from the provider, including trailing postcard bytes.
#[doc(hidden)]
pub fn __option_page<T: SourceProvider>(bytes: &[u8]) -> Result<(OptionPage, OptionRequest)> {
    let request: OptionRequest = __decode_request(bytes)?;
    validate_request(&request).map_err(|_| invalid_request())?;
    let page = T::get_option_page(request.clone())?;
    Ok((page, request))
}
#[doc(hidden)]
pub fn __suggestions<T: SourceProvider>(bytes: &[u8], kind: i32) -> Result<Vec<SearchSuggestion>> {
    let query = std::str::from_utf8(bytes).map_err(|_| invalid_request())?;
    validate_suggestion_query(query).map_err(|_| invalid_request())?;
    T::get_search_suggestions(query, crate::export::__search_kind(kind)?)
}
pub fn validate_form_result(value: &SettingsForm) -> std::result::Result<(), Violation> {
    validate_form(value)
}
pub fn validate_capabilities_result(
    value: &SearchCapabilities,
) -> std::result::Result<(), Violation> {
    validate_capabilities(value)
}
pub fn validate_page_result(
    value: &OptionPage,
    request: &OptionRequest,
) -> std::result::Result<(), Violation> {
    validate_page(value, request)
}
pub fn validate_suggestions_result(
    value: &[SearchSuggestion],
) -> std::result::Result<(), Violation> {
    validate_suggestions(value)
}
#[macro_export]
macro_rules! __export_controls {
    ($type:ty, $provider:path) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn get_settings_form() -> i64 {
            $crate::controls::export::__pack(
                "get_settings_form",
                <$type as $provider>::get_settings_form(),
                $crate::controls::validate_form_result,
            )
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn get_search_capabilities(kind: i32) -> i64 {
            let result = $crate::export::__search_kind(kind)
                .and_then(<$type as $provider>::get_search_capabilities);
            $crate::controls::export::__pack(
                "get_search_capabilities",
                result,
                $crate::controls::validate_capabilities_result,
            )
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn get_option_page(ptr: i32, len: i32) -> i64 {
            // SAFETY: the host owns a readable argument buffer for this call.
            let bytes = unsafe { $crate::export::__bytes(ptr, len) };
            match $crate::controls::export::__option_page::<$type>(bytes) {
                Ok((page, request)) => {
                    $crate::controls::export::__pack("get_option_page", Ok(page), |p| {
                        $crate::controls::validate_page(p, &request)
                    })
                }
                Err(e) => $crate::controls::export::__pack::<$crate::controls::OptionPage>(
                    "get_option_page",
                    Err(e),
                    |_| Ok(()),
                ),
            }
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn get_search_suggestions(ptr: i32, len: i32, kind: i32) -> i64 {
            // SAFETY: the host owns a readable argument buffer for this call.
            let bytes = unsafe { $crate::export::__bytes(ptr, len) };
            $crate::controls::export::__pack(
                "get_search_suggestions",
                $crate::controls::export::__suggestions::<$type>(bytes, kind),
                |v| $crate::controls::validate_suggestions(v),
            )
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_packing_records_the_real_one_shot_error_report() {
        let _ = crate::error::take_last_report();
        assert_eq!(
            __pack::<SettingsForm>("get_settings_form", Err(Error::Unsupported), validate_form),
            0
        );
        let bytes =
            crate::error::take_last_report().expect("unsupported export stores an error report");
        let report: crate::error::ErrorReport =
            postcard::from_bytes(&bytes).expect("decode actual last error report");
        assert_eq!(report.category, crate::error::ErrorCategory::Unsupported);
        assert_eq!(report.category as i32, 6);
        assert!(
            crate::error::take_last_report().is_none(),
            "reports are consumed once"
        );
    }

    #[test]
    fn serialized_response_budget_applies_after_structural_validation() {
        let form = SettingsForm::new(
            (0..4)
                .map(|section| ControlSection {
                    id: format!("s{section}"),
                    title: "Section".into(),
                    summary: None,
                    flags: 0,
                    controls: (0..64)
                        .map(|index| {
                            let mut c = Control::toggle(
                                format!("c{section}_{index}"),
                                "t".repeat(256),
                                false,
                            );
                            c.summary = Some("s".repeat(1024));
                            c
                        })
                        .collect(),
                })
                .collect(),
        );
        validate_form(&form).expect("the form is individually and structurally valid");
        let error = __encode("get_settings_form", Ok(form), validate_form)
            .expect_err("aggregate serialized response exceeds the byte budget");
        assert!(
            matches!(error, Error::Host(message) if message == "get_settings_form: invalid payload: response.too_large")
        );
    }
}
