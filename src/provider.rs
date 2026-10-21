use crate::models::{
    anime::{self, Anime, Episode, Video},
    manga::{self, Chapter, Manga},
    novel::{self, Novel},
    FilterItem, HomeLayout, Listing, Page,
};
use crate::Result;

pub trait MangaProvider {
    fn get_home() -> Result<HomeLayout> {
        Ok(HomeLayout {
            components: Vec::new(),
        })
    }

    fn get_settings() -> Option<crate::models::SettingsSchema> {
        None
    }

    fn get_home_stream() -> Result<bool> {
        Ok(false)
    }

    fn get_manga_list(listing: Listing, page: i32) -> Result<manga::PageResult>;

    fn get_search_manga_list(
        query: &str,
        page: i32,
        filters: Vec<FilterItem>,
    ) -> Result<manga::PageResult>;

    fn get_manga_update(manga: Manga, needs_details: bool, needs_chapters: bool) -> Result<Manga>;

    fn get_page_list(manga: Manga, chapter: Chapter) -> Result<Vec<Page>>;

    fn handle_url(_url: &str) -> Result<crate::models::LinkValue> {
        Err(crate::Error::Unsupported)
    }

    fn handle_image(_url: &str) -> Result<Vec<u8>> {
        Err(crate::Error::Unsupported)
    }
}

pub trait AnimeProvider {
    fn get_home() -> Result<HomeLayout> {
        Ok(HomeLayout {
            components: Vec::new(),
        })
    }

    fn get_settings() -> Option<crate::models::SettingsSchema> {
        None
    }

    fn get_home_stream() -> Result<bool> {
        Ok(false)
    }

    fn get_anime_list(listing: Listing, page: i32) -> Result<anime::PageResult>;

    fn get_search_anime_list(
        query: &str,
        page: i32,
        filters: Vec<FilterItem>,
    ) -> Result<anime::PageResult>;

    fn get_anime_update(anime: Anime, needs_details: bool, needs_episodes: bool) -> Result<Anime>;

    fn get_video_list(anime: Anime, episode: Episode) -> Result<Vec<Video>>;

    fn handle_url(_url: &str) -> Result<crate::models::LinkValue> {
        Err(crate::Error::Unsupported)
    }
}

pub trait NovelProvider {
    fn get_home() -> Result<HomeLayout> {
        Ok(HomeLayout {
            components: Vec::new(),
        })
    }

    fn get_settings() -> Option<crate::models::SettingsSchema> {
        None
    }

    fn get_home_stream() -> Result<bool> {
        Ok(false)
    }

    fn get_novel_list(listing: Listing, page: i32) -> Result<novel::PageResult>;

    fn get_search_novel_list(
        query: &str,
        page: i32,
        filters: Vec<FilterItem>,
    ) -> Result<novel::PageResult>;

    fn get_novel_update(novel: Novel, needs_details: bool, needs_chapters: bool) -> Result<Novel>;

    fn get_chapter_content(novel: Novel, chapter: novel::Chapter) -> Result<Vec<Page>>;

    fn handle_url(_url: &str) -> Result<crate::models::LinkValue> {
        Err(crate::Error::Unsupported)
    }
}

#[macro_export]
macro_rules! export_manga_plugin {
    ($type:ty) => {
        $crate::__export_abi_version!();
        $crate::__export_last_error!();

        #[unsafe(no_mangle)]
        pub extern "C" fn handle_url(url_ptr: i32, url_len: i32) -> i64 {
            let slice = unsafe { core::slice::from_raw_parts(url_ptr as *const u8, url_len as usize) };
            let url = String::from_utf8_lossy(slice);
            let url = url.as_ref();

            match <$type as $crate::provider::MangaProvider>::handle_url(url) {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("handle_url", &e);
                    0
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn handle_image(url_ptr: i32, url_len: i32) -> i64 {
            let slice = unsafe { core::slice::from_raw_parts(url_ptr as *const u8, url_len as usize) };
            let url = String::from_utf8_lossy(slice);
            let url = url.as_ref();

            match <$type as $crate::provider::MangaProvider>::handle_image(url) {
                Ok(res) => {
                    let bytes = res.into_boxed_slice();
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    if !matches!(e, $crate::Error::Unsupported) {
                        $crate::error::__report_failure("handle_image", &e);
                    }
                    0
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_settings() -> i64 {
            match <$type as $crate::provider::MangaProvider>::get_settings() {
                Some(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                None => 0
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_home_stream() -> i32 {
            match <$type as $crate::provider::MangaProvider>::get_home_stream() {
                Ok(true) => 1,
                Ok(false) => 0,
                Err(e) => {
                    $crate::error::__report_failure("get_home_stream", &e);
                    0
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_home() -> i64 {
            match <$type as $crate::provider::MangaProvider>::get_home() {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("get_home", &e);
                    return 0;
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_manga_list(listing_ptr: i32, listing_len: i32, page: i32) -> i64 {
            let slice =
                if listing_ptr == 0 || listing_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(listing_ptr as *const u8, listing_len as usize) } };
            let listing: $crate::models::Listing = match $crate::postcard::from_bytes(slice) {
                Ok(v) => v,
                Err(e) => {
                    $crate::host::print(&format!("Deserialization error: {}", e));
                    return 0;
                }
            };
            match <$type as $crate::provider::MangaProvider>::get_manga_list(listing, page) {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes); // Allow Swift to safely read without deallocation
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("get_manga_list", &e);
                    return 0;
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_search_manga_list(
            query_ptr: i32,
            query_len: i32,
            page: i32,
            filters_ptr: i32,
            filters_len: i32,
        ) -> i64 {
            let q_slice =
                if query_ptr == 0 || query_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(query_ptr as *const u8, query_len as usize) } };
            let query = String::from_utf8_lossy(q_slice);
            let query = query.as_ref();

            let f_slice =
                if filters_ptr == 0 || filters_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(filters_ptr as *const u8, filters_len as usize) } };
            let filters: Vec<$crate::models::FilterItem> = if filters_len == 0 {
                Vec::new()
            } else {
                match $crate::postcard::from_bytes(f_slice) {
                    Ok(v) => v,
                    Err(e) => {
                        $crate::host::print(&format!("Deserialization error: {}", e));
                        return 0;
                    }
                }
            };

            match <$type as $crate::provider::MangaProvider>::get_search_manga_list(query, page, filters) {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("get_search_manga_list", &e);
                    return 0;
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_manga_update(
            manga_ptr: i32,
            manga_len: i32,
            needs_details: i32,
            needs_chapters: i32,
        ) -> i64 {
            let slice =
                if manga_ptr == 0 || manga_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(manga_ptr as *const u8, manga_len as usize) } };

            let manga: $crate::models::manga::Manga = match $crate::postcard::from_bytes(slice) {
                Ok(m) => m,
                Err(e) => {
                    let msg = format!("Postcard decoding error in get_manga_update: {}", e);
                    $crate::host::print(&msg);
                    return 0;
                }
            };
            match <$type as $crate::provider::MangaProvider>::get_manga_update(
                manga,
                needs_details != 0,
                needs_chapters != 0,
            ) {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("get_manga_update", &e);
                    return 0;
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_page_list(
            manga_ptr: i32,
            manga_len: i32,
            chapter_ptr: i32,
            chapter_len: i32,
        ) -> i64 {
            let m_slice =
                if manga_ptr == 0 || manga_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(manga_ptr as *const u8, manga_len as usize) } };
            let manga: $crate::models::manga::Manga = match $crate::postcard::from_bytes(m_slice) {
                Ok(v) => v,
                Err(e) => {
                    $crate::host::print(&format!("Deserialization error: {}", e));
                    return 0;
                }
            };

            let c_slice =
                if chapter_ptr == 0 || chapter_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(chapter_ptr as *const u8, chapter_len as usize) } };
            let chapter: $crate::models::manga::Chapter = match $crate::postcard::from_bytes(c_slice) {
                Ok(v) => v,
                Err(e) => {
                    $crate::host::print(&format!("Deserialization error: {}", e));
                    return 0;
                }
            };

            match <$type as $crate::provider::MangaProvider>::get_page_list(manga, chapter) {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("get_page_list", &e);
                    return 0;
                }
            }
        }
    };
}

#[macro_export]
macro_rules! export_novel_plugin {
    ($type:ty) => {
        $crate::__export_abi_version!();
        $crate::__export_last_error!();

        #[unsafe(no_mangle)]
        pub extern "C" fn handle_url(url_ptr: i32, url_len: i32) -> i64 {
            let slice = unsafe { core::slice::from_raw_parts(url_ptr as *const u8, url_len as usize) };
            let url = String::from_utf8_lossy(slice);
            let url = url.as_ref();

            match <$type as $crate::provider::NovelProvider>::handle_url(url) {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("handle_url", &e);
                    0
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_settings() -> i64 {
            match <$type as $crate::provider::NovelProvider>::get_settings() {
                Some(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                None => 0
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_home_stream() -> i32 {
            match <$type as $crate::provider::NovelProvider>::get_home_stream() {
                Ok(true) => 1,
                Ok(false) => 0,
                Err(e) => {
                    $crate::error::__report_failure("get_home_stream", &e);
                    0
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_home() -> i64 {
            match <$type as $crate::provider::NovelProvider>::get_home() {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("get_home", &e);
                    return 0;
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_novel_list(listing_ptr: i32, listing_len: i32, page: i32) -> i64 {
            let slice =
                if listing_ptr == 0 || listing_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(listing_ptr as *const u8, listing_len as usize) } };
            let listing: $crate::models::Listing = match $crate::postcard::from_bytes(slice) {
                Ok(v) => v,
                Err(e) => {
                    $crate::host::print(&format!("Deserialization error: {}", e));
                    return 0;
                }
            };
            match <$type as $crate::provider::NovelProvider>::get_novel_list(listing, page) {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes); 
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("get_novel_list", &e);
                    return 0;
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_search_novel_list(
            query_ptr: i32,
            query_len: i32,
            page: i32,
            filters_ptr: i32,
            filters_len: i32,
        ) -> i64 {
            let q_slice =
                if query_ptr == 0 || query_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(query_ptr as *const u8, query_len as usize) } };
            let query = String::from_utf8_lossy(q_slice);
            let query = query.as_ref();

            let f_slice =
                if filters_ptr == 0 || filters_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(filters_ptr as *const u8, filters_len as usize) } };
            let filters: Vec<$crate::models::FilterItem> = if filters_len == 0 {
                Vec::new()
            } else {
                match $crate::postcard::from_bytes(f_slice) {
                    Ok(v) => v,
                    Err(e) => {
                        $crate::host::print(&format!("Deserialization error: {}", e));
                        return 0;
                    }
                }
            };

            match <$type as $crate::provider::NovelProvider>::get_search_novel_list(query, page, filters) {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("get_search_novel_list", &e);
                    return 0;
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_novel_update(
            novel_ptr: i32,
            novel_len: i32,
            needs_details: i32,
            needs_chapters: i32,
        ) -> i64 {
            let slice =
                if novel_ptr == 0 || novel_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(novel_ptr as *const u8, novel_len as usize) } };

            let novel: $crate::models::novel::Novel = match $crate::postcard::from_bytes(slice) {
                Ok(n) => n,
                Err(e) => {
                    let msg = format!("Postcard decoding error in get_novel_update: {}", e);
                    $crate::host::print(&msg);
                    return 0;
                }
            };
            match <$type as $crate::provider::NovelProvider>::get_novel_update(
                novel,
                needs_details != 0,
                needs_chapters != 0,
            ) {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("get_novel_update", &e);
                    return 0;
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_chapter_content(
            novel_ptr: i32,
            novel_len: i32,
            chapter_ptr: i32,
            chapter_len: i32,
        ) -> i64 {
            let n_slice =
                if novel_ptr == 0 || novel_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(novel_ptr as *const u8, novel_len as usize) } };
            let novel: $crate::models::novel::Novel = match $crate::postcard::from_bytes(n_slice) {
                Ok(v) => v,
                Err(e) => {
                    $crate::host::print(&format!("Deserialization error: {}", e));
                    return 0;
                }
            };

            let c_slice =
                if chapter_ptr == 0 || chapter_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(chapter_ptr as *const u8, chapter_len as usize) } };
            let chapter: $crate::models::novel::Chapter = match $crate::postcard::from_bytes(c_slice) {
                Ok(v) => v,
                Err(e) => {
                    $crate::host::print(&format!("Deserialization error: {}", e));
                    return 0;
                }
            };

            match <$type as $crate::provider::NovelProvider>::get_chapter_content(novel, chapter) {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("get_chapter_content", &e);
                    return 0;
                }
            }
        }
    };
}

#[macro_export]
macro_rules! export_anime_plugin {
    ($type:ty) => {
        $crate::__export_abi_version!();
        $crate::__export_last_error!();

        #[unsafe(no_mangle)]
        pub extern "C" fn handle_url(url_ptr: i32, url_len: i32) -> i64 {
            let slice = unsafe { core::slice::from_raw_parts(url_ptr as *const u8, url_len as usize) };
            let url = String::from_utf8_lossy(slice);
            let url = url.as_ref();

            match <$type as $crate::provider::AnimeProvider>::handle_url(url) {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("handle_url", &e);
                    0
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_settings() -> i64 {
            match <$type as $crate::provider::AnimeProvider>::get_settings() {
                Some(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                None => 0
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_home_stream() -> i32 {
            match <$type as $crate::provider::AnimeProvider>::get_home_stream() {
                Ok(true) => 1,
                Ok(false) => 0,
                Err(e) => {
                    $crate::error::__report_failure("get_home_stream", &e);
                    0
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_home() -> i64 {
            match <$type as $crate::provider::AnimeProvider>::get_home() {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("get_home", &e);
                    return 0;
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_anime_list(listing_ptr: i32, listing_len: i32, page: i32) -> i64 {
            let slice =
                if listing_ptr == 0 || listing_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(listing_ptr as *const u8, listing_len as usize) } };
            let listing: $crate::models::Listing = match $crate::postcard::from_bytes(slice) {
                Ok(v) => v,
                Err(e) => {
                    $crate::host::print(&format!("Deserialization error: {}", e));
                    return 0;
                }
            };
            match <$type as $crate::provider::AnimeProvider>::get_anime_list(listing, page) {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes); 
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("get_anime_list", &e);
                    return 0;
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_search_anime_list(
            query_ptr: i32,
            query_len: i32,
            page: i32,
            filters_ptr: i32,
            filters_len: i32,
        ) -> i64 {
            let q_slice =
                if query_ptr == 0 || query_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(query_ptr as *const u8, query_len as usize) } };
            let query = String::from_utf8_lossy(q_slice);
            let query = query.as_ref();

            let f_slice =
                if filters_ptr == 0 || filters_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(filters_ptr as *const u8, filters_len as usize) } };
            let filters: Vec<$crate::models::FilterItem> = if filters_len == 0 {
                Vec::new()
            } else {
                match $crate::postcard::from_bytes(f_slice) {
                    Ok(v) => v,
                    Err(e) => {
                        $crate::host::print(&format!("Deserialization error: {}", e));
                        return 0;
                    }
                }
            };

            match <$type as $crate::provider::AnimeProvider>::get_search_anime_list(query, page, filters) {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("get_search_anime_list", &e);
                    return 0;
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_anime_update(
            anime_ptr: i32,
            anime_len: i32,
            needs_details: i32,
            needs_episodes: i32,
        ) -> i64 {
            let slice =
                if anime_ptr == 0 || anime_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(anime_ptr as *const u8, anime_len as usize) } };

            let anime: $crate::models::anime::Anime = match $crate::postcard::from_bytes(slice) {
                Ok(a) => a,
                Err(e) => {
                    let msg = format!("Postcard decoding error in get_anime_update: {}", e);
                    $crate::host::print(&msg);
                    return 0;
                }
            };
            match <$type as $crate::provider::AnimeProvider>::get_anime_update(
                anime,
                needs_details != 0,
                needs_episodes != 0,
            ) {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("get_anime_update", &e);
                    return 0;
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_video_list(
            anime_ptr: i32,
            anime_len: i32,
            episode_ptr: i32,
            episode_len: i32,
        ) -> i64 {
            let a_slice =
                if anime_ptr == 0 || anime_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(anime_ptr as *const u8, anime_len as usize) } };
            let anime: $crate::models::anime::Anime = match $crate::postcard::from_bytes(a_slice) {
                Ok(v) => v,
                Err(e) => {
                    $crate::host::print(&format!("Deserialization error: {}", e));
                    return 0;
                }
            };

            let e_slice =
                if episode_ptr == 0 || episode_len <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(episode_ptr as *const u8, episode_len as usize) } };
            let episode: $crate::models::anime::Episode = match $crate::postcard::from_bytes(e_slice) {
                Ok(v) => v,
                Err(e) => {
                    $crate::host::print(&format!("Deserialization error: {}", e));
                    return 0;
                }
            };

            match <$type as $crate::provider::AnimeProvider>::get_video_list(anime, episode) {
                Ok(res) => {
                    let bytes = match $crate::postcard::to_allocvec(&res) {
                        Ok(b) => b.into_boxed_slice(),
                        Err(e) => {
                            $crate::host::print(&format!("Serialization error: {}", e));
                            return 0;
                        }
                    };
                    let ptr = bytes.as_ptr() as u64;
                    let len = bytes.len() as u64;
                    let _ = Box::into_raw(bytes);
                    ((ptr << 32) | len) as i64
                }
                Err(e) => {
                    $crate::error::__report_failure("get_video_list", &e);
                    return 0;
                }
            }
        }
    };
}
