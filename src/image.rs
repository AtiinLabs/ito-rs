//! Native image request exports used by the provider macros.

use crate::Result;
use serde::Serialize;

/// Packs either native image export's result with the standard allocation protocol.
#[doc(hidden)]
pub fn __pack_result<T: Serialize>(export: &str, result: Result<T>) -> i64 {
    let bytes = match result.and_then(|value| postcard::to_allocvec(&value).map_err(crate::Error::from)) {
        Ok(bytes) => bytes.into_boxed_slice(),
        Err(error) => {
            crate::error::__report_failure(export, &error);
            return 0;
        }
    };
    let ptr = bytes.as_ptr() as u64;
    let len = bytes.len() as u64;
    let _ = Box::into_raw(bytes);
    ((ptr << 32) | len) as i64
}

/// Emits the optional image exports for one provider. Older hosts ignore them and continue using
/// the provider's byte handler; newer hosts resolve requests and fetch outside the WASM lane.
#[doc(hidden)]
#[macro_export]
macro_rules! __export_image_requests {
    ($type:ty, $provider:path) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn get_image_origins() -> i64 {
            $crate::image::__pack_result(
                "get_image_origins",
                <$type as $provider>::get_image_origins(),
            )
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn get_image_request(url_ptr: i32, url_len: i32) -> i64 {
            let bytes = if url_ptr == 0 || url_len <= 0 {
                &[]
            } else {
                unsafe { core::slice::from_raw_parts(url_ptr as *const u8, url_len as usize) }
            };
            let url = String::from_utf8_lossy(bytes);
            $crate::image::__pack_result(
                "get_image_request",
                <$type as $provider>::get_image_request(url.as_ref()),
            )
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::models::PluginImageRequest;
    use std::collections::HashMap;

    #[test]
    fn image_requests_have_a_url_then_optional_header_map_on_the_wire() -> Result<(), postcard::Error> {
        let request = PluginImageRequest {
            url: "http://x/p".into(),
            headers: Some(HashMap::from([("K".into(), "V".into())])),
        };
        let bytes = postcard::to_allocvec(&request)?;
        let mut expected = vec![10];
        expected.extend_from_slice(b"http://x/p");
        expected.extend_from_slice(&[1, 1, 1, b'K', 1, b'V']);
        assert_eq!(bytes, expected);
        let decoded: PluginImageRequest = postcard::from_bytes(&expected)?;
        assert_eq!(decoded, request);

        let without_headers = PluginImageRequest { url: request.url, headers: None };
        let mut expected = vec![10];
        expected.extend_from_slice(b"http://x/p");
        expected.push(0);
        assert_eq!(postcard::to_allocvec(&without_headers)?, expected);
        Ok(())
    }
}
