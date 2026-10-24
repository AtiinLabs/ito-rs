use crate::host;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebviewRequest {
    pub url: String,
    /// Reserved; hosts ignore it. Always None.
    pub script: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebviewResponse {
    pub url: String,
    pub html: String,
}

pub struct Webview;

impl Webview {
    pub fn load_url(url: &str) -> Result<WebviewResponse> {
        let req = WebviewRequest {
            url: url.to_string(),
            script: None,
        };
        let req_bytes = postcard::to_allocvec(&req).map_err(Error::Postcard)?;
        
        let len = unsafe { host::webview_load_url(req_bytes.as_ptr() as i32, req_bytes.len() as i32) };
        if len <= 0 {
            host::print("webview_load_url FFI returned len <= 0");
            return Err(Error::Unsupported);
        }
        
        let mut response_buf = Vec::<u8>::with_capacity(len as usize);
        let ptr = response_buf.as_mut_ptr();
        
        unsafe { 
            host::webview_read_result(ptr as i32);
            response_buf.set_len(len as usize);
        }

        postcard::from_bytes(&response_buf).map_err(Error::Postcard)
    }

    /// Runs `script` in the page last loaded by `load_url` and returns its result as text. A
    /// script that evaluates to nothing (`undefined` or `""`) returns an empty string; a
    /// cancelled call or a host without a web view is an error.
    pub fn execute_js(script: &str) -> Result<String> {
        let bytes = script.as_bytes();
        let len = unsafe { host::webview_execute_js(bytes.as_ptr() as i32, bytes.len() as i32) };
        if len < 0 {
            host::print("webview_execute_js FFI returned no result");
            return Err(Error::Unsupported);
        }
        if len == 0 {
            return Ok(String::new());
        }
        
        let mut response_buf = Vec::<u8>::with_capacity(len as usize);
        let ptr = response_buf.as_mut_ptr();
        
        unsafe { 
            host::webview_read_result(ptr as i32);
            response_buf.set_len(len as usize);
        }

        String::from_utf8(response_buf).map_err(|_| Error::Unsupported)
    }
}
