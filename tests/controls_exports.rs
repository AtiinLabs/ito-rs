use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

use ito_rs::{
    Error, Result,
    controls::{
        ControlSection, OptionPage, OptionQuery, OptionRequest, SearchCapabilities,
        SearchSuggestion, SettingsForm, export, validate_page, validate_request,
        validate_suggestions,
    },
    models::{FilterItem, Listing, MediaEntry, MediaKind, MediaPageResult},
    provider::SourceProvider,
};

static MODE: AtomicUsize = AtomicUsize::new(0);
static FORM_CALLS: AtomicUsize = AtomicUsize::new(0);
static CAPABILITY_CALLS: AtomicUsize = AtomicUsize::new(0);
static OPTION_CALLS: AtomicUsize = AtomicUsize::new(0);
static SUGGESTION_CALLS: AtomicUsize = AtomicUsize::new(0);
static TEST_LOCK: Mutex<()> = Mutex::new(());

struct ExportProvider;
impl SourceProvider for ExportProvider {
    const KINDS: &'static [MediaKind] = &[MediaKind::Manga];
    fn get_listing(_: Listing, _: i32) -> Result<MediaPageResult> {
        Ok(MediaPageResult {
            entries: Vec::new(),
            has_next_page: false,
        })
    }
    fn search(
        _: &str,
        _: i32,
        _: Option<MediaKind>,
        _: Vec<FilterItem>,
    ) -> Result<MediaPageResult> {
        Ok(MediaPageResult {
            entries: Vec::new(),
            has_next_page: false,
        })
    }
    fn get_media_update(entry: MediaEntry, _: bool, _: bool) -> Result<MediaEntry> {
        Ok(entry)
    }

    fn get_settings_form() -> Result<SettingsForm> {
        FORM_CALLS.fetch_add(1, Ordering::SeqCst);
        match MODE.load(Ordering::SeqCst) {
            0 => Err(Error::Unsupported),
            1 => Ok(SettingsForm::new(vec![ControlSection::new(
                "settings", "Settings",
            )])),
            _ => Ok(SettingsForm::new(vec![ControlSection::new(
                "not an id!",
                "Settings",
            )])),
        }
    }
    fn get_search_capabilities(_: Option<MediaKind>) -> Result<SearchCapabilities> {
        CAPABILITY_CALLS.fetch_add(1, Ordering::SeqCst);
        match MODE.load(Ordering::SeqCst) {
            0 => Err(Error::Unsupported),
            1 => Ok(SearchCapabilities::new()),
            _ => Ok(SearchCapabilities {
                flags: 1 << 31,
                sort: None,
                sections: Vec::new(),
            }),
        }
    }
    fn get_option_page(_: OptionRequest) -> Result<OptionPage> {
        OPTION_CALLS.fetch_add(1, Ordering::SeqCst);
        if MODE.load(Ordering::SeqCst) == 1 {
            Ok(OptionPage {
                options: Vec::new(),
                next_offset: None,
                total: None,
                revision: None,
            })
        } else {
            Err(Error::Unsupported)
        }
    }
    fn get_search_suggestions(query: &str, _: Option<MediaKind>) -> Result<Vec<SearchSuggestion>> {
        SUGGESTION_CALLS.fetch_add(1, Ordering::SeqCst);
        if MODE.load(Ordering::SeqCst) == 1 {
            Ok(vec![SearchSuggestion {
                query: query.into(),
                subtitle: None,
            }])
        } else {
            Err(Error::Unsupported)
        }
    }
}
ito_rs::export_plugin!(ExportProvider);

#[test]
fn optional_exports_handle_unsupported_and_validate_before_packing() {
    let _guard = TEST_LOCK.lock().expect("test lock is not poisoned");
    MODE.store(0, Ordering::SeqCst);
    FORM_CALLS.store(0, Ordering::SeqCst);
    CAPABILITY_CALLS.store(0, Ordering::SeqCst);
    OPTION_CALLS.store(0, Ordering::SeqCst);
    SUGGESTION_CALLS.store(0, Ordering::SeqCst);

    // These exports have no pointer arguments, so these calls are safe on native targets.
    assert_eq!(get_settings_form(), 0);
    assert_eq!(get_search_capabilities(MediaKind::Manga as i32), 0);
    assert_eq!(FORM_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(CAPABILITY_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(
        Error::Unsupported.report().category as i32,
        6,
        "Unsupported uses wire category 6"
    );

    let form = SettingsForm::new(vec![ControlSection::new("settings", "Settings")]);
    let bytes = export::__encode(
        "get_settings_form",
        Ok(form.clone()),
        export::validate_form_result,
    )
    .expect("valid form encodes without allocating a WASM pointer");
    assert_eq!(
        bytes,
        ito_rs::postcard::to_allocvec(&form).expect("form postcard bytes")
    );
    let capabilities = SearchCapabilities::new();
    assert!(
        export::__encode(
            "get_search_capabilities",
            Ok(capabilities),
            export::validate_capabilities_result
        )
        .is_ok()
    );

    let invalid_form = SettingsForm::new(vec![ControlSection::new("not an id!", "Settings")]);
    let error = export::__encode(
        "get_settings_form",
        Ok(invalid_form),
        export::validate_form_result,
    )
    .expect_err("invalid form rejected");
    assert!(matches!(error, Error::Host(message) if message.contains("chars.id")));
    MODE.store(2, Ordering::SeqCst);
    assert_eq!(
        get_settings_form(),
        0,
        "invalid form is refused before ABI packing"
    );
    assert_eq!(
        get_search_capabilities(MediaKind::Manga as i32),
        0,
        "undefined flags are refused before ABI packing"
    );
}

#[test]
fn option_dispatch_decodes_and_validates_before_provider_call() {
    let _guard = TEST_LOCK.lock().expect("test lock is not poisoned");
    OPTION_CALLS.store(0, Ordering::SeqCst);
    MODE.store(0, Ordering::SeqCst);
    let valid = OptionRequest {
        catalog: "tags".into(),
        query: OptionQuery::Search {
            text: "tag".into(),
            offset: 0,
        },
        limit: 1,
    };
    let valid_bytes = ito_rs::postcard::to_allocvec(&valid).expect("encode valid request");
    assert!(matches!(
        export::__option_page::<ExportProvider>(&valid_bytes),
        Err(Error::Unsupported)
    ));
    assert_eq!(
        OPTION_CALLS.load(Ordering::SeqCst),
        1,
        "valid request reaches provider"
    );
    OPTION_CALLS.store(0, Ordering::SeqCst);
    let invalid = OptionRequest {
        catalog: "tags".into(),
        query: OptionQuery::Search {
            text: "query".into(),
            offset: 0,
        },
        limit: 0,
    };
    assert!(validate_request(&invalid).is_err());
    let invalid_bytes = ito_rs::postcard::to_allocvec(&invalid).expect("encode invalid request");
    assert!(export::__option_page::<ExportProvider>(&invalid_bytes).is_err());
    assert_eq!(
        OPTION_CALLS.load(Ordering::SeqCst),
        0,
        "invalid request never reaches provider"
    );

    let mut trailing = ito_rs::postcard::to_allocvec(&valid).expect("encode valid request");
    trailing.push(0);
    assert!(
        export::__option_page::<ExportProvider>(&trailing).is_err(),
        "trailing request bytes are refused"
    );
    assert_eq!(
        OPTION_CALLS.load(Ordering::SeqCst),
        0,
        "malformed request never reaches provider"
    );
    assert_eq!(get_option_page(0, 0), 0, "null empty request is malformed");
    assert_eq!(OPTION_CALLS.load(Ordering::SeqCst), 0);

    MODE.store(1, Ordering::SeqCst);
    let request_bytes = ito_rs::postcard::to_allocvec(&valid).expect("encode request");
    let (page, decoded_request) =
        export::__option_page::<ExportProvider>(&request_bytes).expect("valid option request");
    assert_eq!(decoded_request, valid);
    validate_page(&page, &decoded_request).expect("provider page satisfies request");
    assert!(
        export::__encode("get_option_page", Ok(page), |page| {
            export::validate_page_result(page, &decoded_request)
        })
        .is_ok()
    );
    assert_eq!(OPTION_CALLS.load(Ordering::SeqCst), 1);
}

#[test]
fn suggestion_dispatch_rejects_invalid_queries_and_encodes_valid_results() {
    let _guard = TEST_LOCK.lock().expect("test lock is not poisoned");
    SUGGESTION_CALLS.store(0, Ordering::SeqCst);
    assert!(export::__suggestions::<ExportProvider>(&[], MediaKind::Manga as i32).is_err());
    assert!(export::__suggestions::<ExportProvider>(&[0xff], MediaKind::Manga as i32).is_err());
    assert_eq!(
        SUGGESTION_CALLS.load(Ordering::SeqCst),
        0,
        "invalid query never reaches provider"
    );

    MODE.store(0, Ordering::SeqCst);
    assert!(matches!(
        export::__suggestions::<ExportProvider>(b"q", MediaKind::Manga as i32),
        Err(Error::Unsupported)
    ));
    assert_eq!(
        SUGGESTION_CALLS.load(Ordering::SeqCst),
        1,
        "valid query reaches optional provider"
    );
    assert_eq!(
        get_search_suggestions(0, 0, MediaKind::Manga as i32),
        0,
        "empty query is rejected before provider call"
    );
    assert_eq!(SUGGESTION_CALLS.load(Ordering::SeqCst), 1);
    MODE.store(1, Ordering::SeqCst);
    let result = export::__suggestions::<ExportProvider>(b"manga", MediaKind::Manga as i32)
        .expect("valid suggestions");
    validate_suggestions(&result).expect("suggestions satisfy shared limits");
    assert!(
        export::__encode("get_search_suggestions", Ok(result), |suggestions| {
            export::validate_suggestions_result(suggestions)
        })
        .is_ok()
    );
    assert_eq!(SUGGESTION_CALLS.load(Ordering::SeqCst), 2);
}
