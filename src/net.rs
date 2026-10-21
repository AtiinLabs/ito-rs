use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::host;
use crate::{Result, Error};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetRequest {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetResponse {
    pub status: i32,
    pub headers: HashMap<String, String>,
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct NetOptions {
    pub rate_limit: i32,
    pub persist_cookies: bool,
}

pub struct Request {
    req: NetRequest,
    opts: NetOptions,
}

impl Request {
    pub fn new<S: Into<String>, M: Into<String>>(url: S, method: M) -> Self 
    {
        Self {
            req: NetRequest {
                url: url.into(),
                method: method.into(),
                headers: HashMap::new(),
                body: None,
            },
            opts: NetOptions::default(),
        }
    }

    pub fn get<S: Into<String>>(url: S) -> Self {
        Self::new(url, "GET")
    }

    pub fn post<S: Into<String>>(url: S) -> Self {
        Self::new(url, "POST")
    }

    pub fn header(&mut self, key: &str, value: &str) -> &mut Self {
        self.req.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn body(&mut self, body: &[u8]) -> &mut Self {
        self.req.body = Some(body.to_vec());
        self
    }
    
    pub fn rate_limit(&mut self, rate_limit: i32) -> &mut Self {
        self.opts.rate_limit = rate_limit;
        self
    }

    pub fn persist_cookies(&mut self, persist: bool) -> &mut Self {
        self.opts.persist_cookies = persist;
        self
    }

    pub fn send(&self) -> Result<NetResponse> {
        let req_bytes = postcard::to_allocvec(&self.req)?;
        let opt_bytes = postcard::to_allocvec(&self.opts)?;
        
        // fetch_v2 is always used. A host without it can't load the plugin at all (hosts check a
        // plugin's imports before loading it), so this never fails mid-call.
        let len = unsafe { 
            host::fetch_v2(
                req_bytes.as_ptr() as i32, req_bytes.len() as i32,
                opt_bytes.as_ptr() as i32, opt_bytes.len() as i32,
            ) 
        };
        
        if len <= 0 {
            return postcard::from_bytes(&[]).map_err(|e| {
                let msg = format!("NetResponse FFI Error (len {}): {}", len, e);
                host::print(&msg);
                Error::Postcard(e)
            });
        }
        
        let mut response_buf = Vec::<u8>::with_capacity(len as usize);
        let ptr = response_buf.as_mut_ptr();
        
        unsafe { 
            host::fetch_read(ptr as i32);
            response_buf.set_len(len as usize);
        }

        let res: NetResponse = postcard::from_bytes(&response_buf)
            .map_err(|e| {
                 let msg = format!("NetResponse Decoding Error (len {}): {}", len, e);
                 host::print(&msg);
                 Error::Postcard(e)
            })?;

        if let Some(reason) = transport_failure(&res) {
            return Err(Error::Net(reason));
        }
        Ok(res)
    }
}

/// The header a host's 500 response carries when the request never reached the server.
pub const TRANSPORT_ERROR_HEADER: &str = "X-Ito-Transport-Error";

/// The reason the host gave, when `response` is its answer to a request that never reached the
/// server (no connection, cancelled, timed out) rather than the server's. Hosts mark those 500s
/// with `X-Ito-Transport-Error`; older hosts only with an "FFI Error in <import>: " body.
fn transport_failure(response: &NetResponse) -> Option<String> {
    // Hosts only answer with a 500; a header on anything else came from a server.
    if response.status != 500 {
        return None;
    }
    if let Some((_, reason)) = response
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(TRANSPORT_ERROR_HEADER))
    {
        return Some(reason.clone());
    }
    let body = String::from_utf8_lossy(&response.body);
    let rest = body.strip_prefix("FFI Error in ")?;
    Some(rest.split_once(": ").map_or(rest, |(_, reason)| reason).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(status: i32, headers: &[(&str, &str)], body: &str) -> NetResponse {
        NetResponse {
            status,
            headers: headers.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            body: body.as_bytes().to_vec(),
        }
    }

    #[test]
    fn a_request_that_never_reached_the_server_is_a_network_failure() {
        let marked = response(500, &[("x-ito-transport-error", "The request timed out.")], "");
        assert_eq!(transport_failure(&marked).as_deref(), Some("The request timed out."));

        let older_host = response(500, &[], "FFI Error in net_fetch_v2: The Internet connection appears to be offline.");
        assert_eq!(
            transport_failure(&older_host).as_deref(),
            Some("The Internet connection appears to be offline.")
        );
    }

    #[test]
    fn a_servers_own_error_is_not() {
        assert_eq!(transport_failure(&response(500, &[], "Internal Server Error")), None);
        assert_eq!(transport_failure(&response(404, &[], "FFI Error in net_fetch: x")), None);
        assert_eq!(transport_failure(&response(200, &[], "ok")), None);
        // A server's header (older hosts pass it through) on a success is not the host's.
        assert_eq!(transport_failure(&response(200, &[(TRANSPORT_ERROR_HEADER, "fake")], "ok")), None);
    }
}
