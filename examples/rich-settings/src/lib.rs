use ito_rs::{
    Error, Result,
    controls::{
        ChoiceOption, Control, ControlSection, ControlValues, IntegerRangeControl,
        MultiSelectControl, OptionPage, OptionQuery, OptionRequest, OptionSource,
        SearchCapabilities, SearchSuggestion, SecretControl, SelectControl, SettingsForm,
        TextControl,
    },
    models::{
        FilterItem, Listing, MediaEntry, MediaKind, MediaPageResult,
        manga::{ContentRating, Manga, Status, Viewer},
    },
    provider::SourceProvider,
};

const TAG_COUNT: u32 = 6_000;
const TAG_GROUPS: u32 = 19;
const PAGE_SIZE: usize = 12;

pub struct Example;

impl SourceProvider for Example {
    const KINDS: &'static [MediaKind] = &[MediaKind::Manga];

    fn get_listing(_: Listing, page: i32) -> Result<MediaPageResult> {
        Self::search("", page, Some(MediaKind::Manga), Vec::new())
    }

    fn search(
        query: &str,
        page: i32,
        _: Option<MediaKind>,
        filters: Vec<FilterItem>,
    ) -> Result<MediaPageResult> {
        let capabilities = Self::get_search_capabilities(None)?;
        let values = ControlValues::search(&capabilities, &filters)?;
        let sort = values.select("sort")?.unwrap_or("popular");
        let adult = values.toggle("adult")?;
        let minimum_year = values.integer("minimum_year")?.unwrap_or(1900);
        let year_bounds = values.integer_range("release_year")?;
        let statuses = values.select("status")?;
        let selected_tags = values.multi_select("tags")?;
        let query = query.to_lowercase();

        // Source listing and search page numbers are one-based; nonpositive values select page one.
        let page_start = (page.saturating_sub(1).max(0) as usize).saturating_mul(PAGE_SIZE);
        let mut matching = (0..100_u32)
            .filter(|&index| {
                let title = format!("Example {index:03}");
                let year = 1900 + i64::from(index % 121);
                let is_adult = index.is_multiple_of(7);
                let status = if index.is_multiple_of(2) {
                    "ongoing"
                } else {
                    "complete"
                };
                let tag_id = (index % TAG_COUNT).to_string();
                title.to_lowercase().contains(query.as_str())
                    && (adult || !is_adult)
                    && year >= minimum_year
                    && year_bounds.0.is_none_or(|lower| year >= lower)
                    && year_bounds.1.is_none_or(|upper| year <= upper)
                    && statuses.is_none_or(|wanted| wanted == status)
                    && (selected_tags.is_empty() || selected_tags.contains(&tag_id))
            })
            .collect::<Vec<_>>();
        if sort == "title" {
            matching.sort_unstable();
        } else {
            matching.sort_unstable_by(|left, right| right.cmp(left));
        }
        let has_next_page = matching.len() > page_start.saturating_add(PAGE_SIZE);
        let entries = matching
            .into_iter()
            .skip(page_start)
            .take(PAGE_SIZE)
            .map(sample_entry)
            .collect();
        Ok(MediaPageResult {
            entries,
            has_next_page,
        })
    }

    fn get_media_update(entry: MediaEntry, _: bool, _: bool) -> Result<MediaEntry> {
        let MediaEntry::Manga(manga) = entry else {
            return Err(Error::Unsupported);
        };
        let Some(index) = manga
            .key
            .strip_prefix("example-")
            .and_then(|value| value.parse::<u32>().ok())
        else {
            return Err(Error::NotFound("unknown sample media key".into()));
        };
        if index >= 100 {
            return Err(Error::NotFound("unknown sample media key".into()));
        }
        Ok(sample_entry(index))
    }

    fn get_settings_form() -> Result<SettingsForm> {
        Ok(SettingsForm {
            sections: vec![
                ControlSection {
                    id: "content".into(),
                    title: "Content".into(),
                    summary: Some("Example controls for source settings.".into()),
                    flags: 0,
                    controls: vec![
                        Control::toggle("adult", "Include adult content", false),
                        Control {
                            id: "query_hint".into(),
                            title: "Search hint".into(),
                            summary: Some("URL keyboard behavior.".into()),
                            flags: 0,
                            visible_when: Vec::new(),
                            kind: ito_rs::controls::ControlKind::Text(TextControl {
                                default_value: String::new(),
                                placeholder: Some("Search query".into()),
                                flags: TextControl::URL,
                            }),
                        },
                        Control {
                            id: "notes".into(),
                            title: "Notes".into(),
                            summary: Some("A multiline preference.".into()),
                            flags: 0,
                            visible_when: Vec::new(),
                            kind: ito_rs::controls::ControlKind::Text(TextControl {
                                default_value: String::new(),
                                placeholder: None,
                                flags: TextControl::MULTILINE,
                            }),
                        },
                        Control {
                            id: "credential".into(),
                            title: "Credential".into(),
                            summary: None,
                            flags: 0,
                            visible_when: Vec::new(),
                            kind: ito_rs::controls::ControlKind::Secret(SecretControl {
                                placeholder: Some("Stored securely by the host".into()),
                            }),
                        },
                        Control {
                            id: "status".into(),
                            title: "Status".into(),
                            summary: None,
                            flags: 0,
                            visible_when: Vec::new(),
                            kind: ito_rs::controls::ControlKind::Select(SelectControl {
                                options: OptionSource::inline(vec![
                                    ChoiceOption::new("ongoing", "Ongoing"),
                                    ChoiceOption::new("complete", "Complete"),
                                ]),
                                default_value: None,
                            }),
                        },
                        Control {
                            id: "minimum_year".into(),
                            title: "Minimum year".into(),
                            summary: None,
                            flags: 0,
                            visible_when: Vec::new(),
                            kind: ito_rs::controls::ControlKind::Integer(
                                ito_rs::controls::IntegerControl {
                                    min: 1900,
                                    max: 2100,
                                    default_value: Some(1900),
                                    unit: Some("year".into()),
                                },
                            ),
                        },
                        Control {
                            id: "release_year".into(),
                            title: "Release year".into(),
                            summary: None,
                            flags: 0,
                            visible_when: vec![ito_rs::controls::ControlCondition {
                                control: "adult".into(),
                                any_of: vec!["true".into()],
                            }],
                            kind: ito_rs::controls::ControlKind::IntegerRange(
                                IntegerRangeControl {
                                    min: 1900,
                                    max: 2020,
                                    default_lower: None,
                                    default_upper: None,
                                    unit: Some("year".into()),
                                },
                            ),
                        },
                    ],
                },
                ControlSection {
                    id: "tags".into(),
                    title: "Tags".into(),
                    summary: None,
                    flags: ControlSection::PAGE,
                    controls: vec![Control {
                        id: "blocked_tags".into(),
                        title: "Blocked tags".into(),
                        summary: None,
                        flags: 0,
                        visible_when: Vec::new(),
                        kind: ito_rs::controls::ControlKind::MultiSelect(MultiSelectControl {
                            options: OptionSource::searchable_catalog("tags"),
                            default_value: Vec::new(),
                            max_selected: Some(500),
                        }),
                    }],
                },
            ],
        })
    }

    fn get_search_capabilities(_: Option<MediaKind>) -> Result<SearchCapabilities> {
        Ok(SearchCapabilities {
            flags: SearchCapabilities::BROWSE
                | SearchCapabilities::SORT_WITH_QUERY
                | SearchCapabilities::FILTERS_WITH_QUERY
                | SearchCapabilities::SUGGESTIONS,
            sort: Some(Control::select(
                "sort",
                "Sort",
                OptionSource::inline(vec![
                    ChoiceOption::new("popular", "Popular"),
                    ChoiceOption::new("title", "Title"),
                ]),
                Some("popular".into()),
            )),
            sections: vec![ControlSection {
                id: "filters".into(),
                title: "Filters".into(),
                summary: None,
                flags: 0,
                controls: vec![
                    Control::toggle("adult", "Include adult content", false),
                    Control {
                        id: "minimum_year".into(),
                        title: "Minimum year".into(),
                        summary: None,
                        flags: 0,
                        visible_when: Vec::new(),
                        kind: ito_rs::controls::ControlKind::Integer(
                            ito_rs::controls::IntegerControl {
                                min: 1900,
                                max: 2100,
                                default_value: Some(1900),
                                unit: Some("year".into()),
                            },
                        ),
                    },
                    Control {
                        id: "status".into(),
                        title: "Status".into(),
                        summary: None,
                        flags: 0,
                        visible_when: Vec::new(),
                        kind: ito_rs::controls::ControlKind::Select(SelectControl {
                            options: OptionSource::inline(vec![
                                ChoiceOption::new("ongoing", "Ongoing"),
                                ChoiceOption::new("complete", "Complete"),
                            ]),
                            default_value: None,
                        }),
                    },
                    Control {
                        id: "release_year".into(),
                        title: "Release year".into(),
                        summary: None,
                        flags: 0,
                        visible_when: Vec::new(),
                        kind: ito_rs::controls::ControlKind::IntegerRange(IntegerRangeControl {
                            min: 1900,
                            max: 2020,
                            default_lower: None,
                            default_upper: None,
                            unit: Some("year".into()),
                        }),
                    },
                    Control::multi_select(
                        "tags",
                        "Tags",
                        OptionSource::searchable_catalog("tags"),
                        Vec::new(),
                    ),
                ],
            }],
        })
    }

    fn get_option_page(request: OptionRequest) -> Result<OptionPage> {
        if request.catalog != "tags" {
            return Err(Error::NotFound("unknown option catalog".into()));
        }
        match request.query {
            OptionQuery::Values(values) => {
                let options = values
                    .into_iter()
                    .filter_map(|value| value.parse::<u32>().ok())
                    .filter(|id| *id < TAG_COUNT)
                    .map(tag_option)
                    .collect();
                Ok(OptionPage {
                    options,
                    next_offset: None,
                    total: None,
                    revision: Some("tags-v1".into()),
                })
            }
            OptionQuery::Search { text, offset } => {
                let needle = text.to_lowercase();
                let total = (0..TAG_COUNT)
                    .filter(|id| tag_label(*id).to_lowercase().contains(&needle))
                    .count() as u32;
                let options = (0..TAG_COUNT)
                    .filter(|id| tag_label(*id).to_lowercase().contains(&needle))
                    .skip(offset as usize)
                    .take(request.limit as usize)
                    .map(tag_option)
                    .collect::<Vec<_>>();
                let next = offset
                    .checked_add(options.len() as u32)
                    .filter(|next| *next < total);
                Ok(OptionPage {
                    options,
                    next_offset: next,
                    total: Some(total),
                    revision: Some("tags-v1".into()),
                })
            }
        }
    }

    fn get_search_suggestions(query: &str, _: Option<MediaKind>) -> Result<Vec<SearchSuggestion>> {
        let normalized = query.trim();
        if normalized.is_empty()
            || normalized.len() > 245
            || normalized.chars().any(|ch| {
                let value = ch as u32;
                value < 32 || (0x7f..=0x9f).contains(&value)
            })
        {
            return Ok(Vec::new());
        }
        let query = normalized.to_lowercase();
        Ok((0..100_u32)
            .filter(|index| !index.is_multiple_of(7))
            .map(|index| format!("Example {index:03}"))
            .filter(|title| title.to_lowercase().contains(&query))
            .take(5)
            .map(|query| SearchSuggestion {
                query,
                subtitle: Some("Local catalog".into()),
            })
            .collect())
    }
}
fn sample_entry(index: u32) -> MediaEntry {
    let year = 1900 + i64::from(index % 121);
    let is_adult = index.is_multiple_of(7);
    MediaEntry::Manga(Manga {
        key: format!("example-{index}"),
        title: format!(
            "Example {index:03} · {year} · {}",
            if is_adult { "adult" } else { "all ages" },
        ),
        authors: None,
        artist: None,
        description: None,
        tags: Some(vec![(index % TAG_COUNT).to_string()]),
        cover: None,
        url: None,
        status: if index.is_multiple_of(2) {
            Status::Ongoing
        } else {
            Status::Completed
        },
        content_rating: if is_adult {
            ContentRating::Nsfw
        } else {
            ContentRating::Safe
        },
        nsfw: if is_adult { 1 } else { 0 },
        viewer: Viewer::Default,
        chapters: None,
    })
}

fn tag_label(id: u32) -> String {
    format!("Tag {id:04}")
}
fn tag_option(id: u32) -> ChoiceOption {
    ChoiceOption::new(id.to_string(), tag_label(id))
        .grouped(format!("Group {}", id * TAG_GROUPS / TAG_COUNT + 1))
}

ito_rs::export_plugin!(Example);

#[cfg(test)]
mod tests {
    use super::*;
    use ito_rs::models::FilterValue;

    #[test]
    fn catalog_contains_six_thousand_options_in_nineteen_groups() {
        let mut values = std::collections::HashSet::new();
        let mut groups = std::collections::HashSet::new();
        for offset in (0..TAG_COUNT).step_by(100) {
            let page = Example::get_option_page(OptionRequest {
                catalog: "tags".into(),
                query: OptionQuery::Search {
                    text: String::new(),
                    offset,
                },
                limit: 100,
            })
            .expect("catalog page");
            for option in page.options {
                values.insert(option.value);
                groups.insert(option.group.expect("catalog groups every option"));
            }
        }
        assert_eq!(values.len(), TAG_COUNT as usize);
        assert_eq!(groups.len(), TAG_GROUPS as usize);
    }

    #[test]
    fn values_mode_returns_only_known_requested_ids() {
        let page = Example::get_option_page(OptionRequest {
            catalog: "tags".into(),
            query: OptionQuery::Values(vec!["2".into(), "9000".into(), "4".into()]),
            limit: 3,
        })
        .expect("label resolution");
        assert_eq!(
            page.options
                .iter()
                .map(|option| option.value.as_str())
                .collect::<Vec<_>>(),
            ["2", "4"]
        );
        assert_eq!(page.options[0].label, "Tag 0002");
        assert_eq!(page.next_offset, None);
    }

    #[test]
    fn search_filters_and_sort_change_catalog_results() {
        let filters = vec![
            FilterItem {
                type_name: "select".into(),
                name: "sort".into(),
                value: FilterValue::String("title".into()),
            },
            FilterItem {
                type_name: "toggle".into(),
                name: "adult".into(),
                value: FilterValue::String("true".into()),
            },
        ];
        let result = Example::search("Example", 1, None, filters).expect("filtered search");
        let keys = result
            .entries
            .iter()
            .map(|entry| entry.key().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(keys.first().map(String::as_str), Some("example-0"));
        let titles = result
            .entries
            .into_iter()
            .map(|entry| match entry {
                MediaEntry::Manga(manga) => manga.title,
                _ => String::new(),
            })
            .collect::<Vec<_>>();
        assert!(titles.windows(2).all(|pair| pair[0] <= pair[1]));
    }
    #[test]
    fn listing_is_populated_and_media_updates_validate_keys() {
        let page = Example::get_listing(
            Listing {
                id: "all".into(),
                name: "All".into(),
                kind: 0,
            },
            1,
        )
        .expect("sample listing");
        assert!(!page.entries.is_empty());
        let next = Example::get_listing(
            Listing {
                id: "all".into(),
                name: "All".into(),
                kind: 0,
            },
            2,
        )
        .expect("second sample listing page");
        assert_ne!(next.entries[0].key(), page.entries[0].key());
        assert!(page.has_next_page);
        let updated = Example::get_media_update(page.entries[0].clone(), true, true)
            .expect("known sample key updates");
        match updated {
            MediaEntry::Manga(manga) => assert!(manga.title.starts_with("Example ")),
            _ => panic!("the sample only returns manga entries"),
        }
        let mut unknown = match sample_entry(0) {
            MediaEntry::Manga(manga) => manga,
            _ => panic!("sample entry must be manga"),
        };
        unknown.key = "unknown".into();
        assert!(matches!(
            Example::get_media_update(MediaEntry::Manga(unknown), true, true),
            Err(Error::NotFound(_))
        ));
    }

    #[test]
    fn suggestions_resolve_to_catalog_results() {
        let suggestions = Example::get_search_suggestions("EXAMPLE 01", None).expect("suggestions");
        assert!(!suggestions.is_empty());
        assert!(suggestions.len() <= 5);
        for suggestion in suggestions {
            let results =
                Example::search(&suggestion.query, 1, None, Vec::new()).expect("matching search");
            assert_eq!(results.entries.len(), 1);
        }
        assert!(
            Example::get_search_suggestions("no such title", None)
                .expect("no matches")
                .is_empty()
        );
    }
}
