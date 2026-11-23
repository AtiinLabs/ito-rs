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

/// Options sent alongside every request. Both fields are reserved: hosts don't act on them, and
/// the SDK always sends the defaults. They stay on the wire so plugins and hosts built before
/// they were retired keep decoding each other's requests.
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

    /// Sends `requests` and returns one result per request, in the same order.
    ///
    /// Requests are sent concurrently, up to [`MAX_BATCH`] at a time, so a paged chapter list or
    /// a home page with several sections takes about one round trip instead of one per request.
    /// Only `GET`, `HEAD` and `OPTIONS` can be batched; any other method fails that element with
    /// `Error::Net` and is not sent. An empty slice returns an empty `Vec` without calling the host.
    ///
    /// Requests in one call run concurrently and don't see each other's cookies: a request can't
    /// rely on a cookie another request in the same call sets. Don't batch requests that depend on
    /// an earlier response's cookies, and don't batch against sites that rate-limit.
    ///
    /// A plugin that calls this needs a host that provides `ito:core/net.fetch_batch`; a host
    /// without it won't load the plugin, so set `min_app_version` to the first app release that
    /// has it.
    pub fn send_all(requests: &[Request]) -> Vec<Result<NetResponse>> {
        send_all_with(&HostTransport, requests)
    }
}

/// The most requests one `fetch_batch` host call carries. The host treats a larger batch as a
/// contract violation, so `send_all` splits longer lists into chunks of at most this many.
pub const MAX_BATCH: usize = 4;

/// The reason the host gives for a batch element it dropped to stay within its response budget.
/// Such an element was never sent for the plugin to see, so `send_all` retries it alone.
const BATCH_BUDGET_REASON: &str = "batch response budget exceeded";

/// The two host calls `send_all` is built from. Host imports don't exist in native builds, so the
/// chunking, ordering and retry logic runs against this seam in the SDK's own tests.
trait Transport {
    /// Sends one chunk of at most [`MAX_BATCH`] requests, returning the host's responses.
    fn fetch_batch(&self, chunk: &[&NetRequest]) -> Result<Vec<NetResponse>>;
    /// Sends one request on its own.
    fn send(&self, request: &Request) -> Result<NetResponse>;
}

struct HostTransport;

impl Transport for HostTransport {
    fn fetch_batch(&self, chunk: &[&NetRequest]) -> Result<Vec<NetResponse>> {
        let req_bytes = postcard::to_allocvec(chunk)?;
        let req_len = i32::try_from(req_bytes.len())
            .map_err(|_| Error::Host("batch request is too large".to_string()))?;

        let len = unsafe { host::fetch_batch(req_bytes.as_ptr() as i32, req_len) };
        if len <= 0 {
            let msg = format!("NetResponse batch FFI Error (len {})", len);
            host::print(&msg);
            return Err(Error::Host(msg));
        }

        let mut response_buf = Vec::<u8>::with_capacity(len as usize);
        let ptr = response_buf.as_mut_ptr();
        unsafe {
            host::fetch_read(ptr as i32);
            response_buf.set_len(len as usize);
        }

        postcard::from_bytes(&response_buf).map_err(|e| {
            let msg = format!("NetResponse batch Decoding Error (len {}): {}", len, e);
            host::print(&msg);
            Error::Postcard(e)
        })
    }

    fn send(&self, request: &Request) -> Result<NetResponse> {
        request.send()
    }
}

/// Whether a method is read-only, so sending it concurrently with others can't reorder effects.
fn is_batchable_method(method: &str) -> bool {
    ["GET", "HEAD", "OPTIONS"].iter().any(|safe| method.eq_ignore_ascii_case(safe))
}

fn send_all_with(transport: &impl Transport, requests: &[Request]) -> Vec<Result<NetResponse>> {
    let mut results: Vec<Option<Result<NetResponse>>> = requests.iter().map(|_| None).collect();

    let mut batchable = Vec::with_capacity(requests.len());
    for (index, request) in requests.iter().enumerate() {
        if is_batchable_method(&request.req.method) {
            batchable.push(index);
        } else {
            results[index] = Some(Err(Error::Net(format!(
                "send_all only sends GET, HEAD and OPTIONS requests, not {}",
                request.req.method
            ))));
        }
    }

    for chunk in batchable.chunks(MAX_BATCH) {
        let wire: Vec<&NetRequest> = chunk.iter().map(|&index| &requests[index].req).collect();
        match transport.fetch_batch(&wire) {
            Ok(responses) if responses.len() == chunk.len() => {
                for (&index, response) in chunk.iter().zip(responses) {
                    results[index] = Some(match transport_failure(&response) {
                        Some(reason) if reason == BATCH_BUDGET_REASON => {
                            transport.send(&requests[index])
                        }
                        Some(reason) => Err(Error::Net(reason)),
                        None => Ok(response),
                    });
                }
            }
            outcome => {
                // An answer that doesn't line up with the request can't be matched to its
                // elements, so none of the chunk's responses are trusted.
                let msg = match outcome {
                    Ok(responses) => format!(
                        "batch answered {} responses for {} requests",
                        responses.len(),
                        chunk.len()
                    ),
                    Err(e) => e.to_string(),
                };
                for &index in chunk {
                    results[index] = Some(Err(Error::Host(msg.clone())));
                }
            }
        }
    }

    results
        .into_iter()
        .map(|result| result.unwrap_or_else(|| Err(Error::Host("batch element was not sent".to_string()))))
        .collect()
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
    use std::cell::RefCell;

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

    /// Answers each request with its own URL as the body, except the URLs that name a host
    /// transport failure, and records which calls the logic made.
    #[derive(Default)]
    struct Scripted {
        batches: RefCell<Vec<Vec<String>>>,
        singles: RefCell<Vec<String>>,
        drop_last_response: bool,
        fail_batch: bool,
    }

    impl Scripted {
        fn answer(url: &str) -> NetResponse {
            match url {
                "offline" => response(500, &[(TRANSPORT_ERROR_HEADER, "offline")], ""),
                "over-budget" => response(500, &[(TRANSPORT_ERROR_HEADER, BATCH_BUDGET_REASON)], ""),
                _ => response(200, &[], url),
            }
        }
    }

    impl Transport for Scripted {
        fn fetch_batch(&self, chunk: &[&NetRequest]) -> Result<Vec<NetResponse>> {
            self.batches.borrow_mut().push(chunk.iter().map(|r| r.url.clone()).collect());
            if self.fail_batch {
                return Err(Error::Host("batch failed".to_string()));
            }
            let mut responses: Vec<NetResponse> = chunk.iter().map(|r| Self::answer(&r.url)).collect();
            if self.drop_last_response {
                responses.pop();
            }
            Ok(responses)
        }

        fn send(&self, request: &Request) -> Result<NetResponse> {
            self.singles.borrow_mut().push(request.req.url.clone());
            Ok(response(200, &[], &format!("single {}", request.req.url)))
        }
    }

    fn gets(urls: &[&str]) -> Vec<Request> {
        urls.iter().map(|url| Request::get(*url)).collect()
    }

    fn body(result: &Result<NetResponse>) -> String {
        String::from_utf8_lossy(&result.as_ref().expect("response").body).into_owned()
    }

    #[test]
    fn empty_input_makes_no_host_call() {
        let host = Scripted::default();
        assert!(send_all_with(&host, &[]).is_empty());
        assert!(host.batches.borrow().is_empty());
        assert!(host.singles.borrow().is_empty());
    }

    #[test]
    fn nine_requests_go_out_as_4_4_1_in_order() {
        let host = Scripted::default();
        let urls: Vec<String> = (0..9).map(|i| format!("u{i}")).collect();
        let requests: Vec<Request> = urls.iter().map(Request::get).collect();

        let results = send_all_with(&host, &requests);

        let sizes: Vec<usize> = host.batches.borrow().iter().map(Vec::len).collect();
        assert_eq!(sizes, [4, 4, 1]);
        assert_eq!(host.batches.borrow().concat(), urls);
        let bodies: Vec<String> = results.iter().map(body).collect();
        assert_eq!(bodies, urls);
        assert!(host.singles.borrow().is_empty());
    }

    #[test]
    fn only_read_only_methods_are_sent() {
        let host = Scripted::default();
        let mut requests = gets(&["a"]);
        requests.push(Request::post("posted"));
        requests.push(Request::new("b", "head"));
        requests.push(Request::new("c", "Options"));
        requests.push(Request::new("deleted", "DELETE"));

        let results = send_all_with(&host, &requests);

        assert_eq!(host.batches.borrow().concat(), ["a", "b", "c"]);
        assert_eq!(results.len(), 5);
        assert_eq!(body(&results[0]), "a");
        assert!(matches!(&results[1], Err(Error::Net(_))));
        assert_eq!(body(&results[2]), "b");
        assert_eq!(body(&results[3]), "c");
        assert!(matches!(&results[4], Err(Error::Net(_))));
    }

    #[test]
    fn a_transport_failure_fails_only_its_element() {
        let host = Scripted::default();
        let results = send_all_with(&host, &gets(&["a", "offline", "c"]));

        assert_eq!(body(&results[0]), "a");
        assert!(matches!(&results[1], Err(Error::Net(reason)) if reason == "offline"));
        assert_eq!(body(&results[2]), "c");
        assert!(host.singles.borrow().is_empty());
    }

    #[test]
    fn only_budget_dropped_elements_are_retried_alone() {
        let host = Scripted::default();
        let results = send_all_with(&host, &gets(&["a", "over-budget", "offline", "over-budget"]));

        assert_eq!(*host.singles.borrow(), ["over-budget", "over-budget"]);
        assert_eq!(body(&results[0]), "a");
        assert_eq!(body(&results[1]), "single over-budget");
        assert!(matches!(&results[2], Err(Error::Net(reason)) if reason == "offline"));
        assert_eq!(body(&results[3]), "single over-budget");
    }

    #[test]
    fn a_response_count_mismatch_fails_the_whole_chunk() {
        let host = Scripted { drop_last_response: true, ..Scripted::default() };
        let results = send_all_with(&host, &gets(&["a", "b", "c", "d", "e"]));

        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| matches!(r, Err(Error::Host(_)))));
        assert!(host.singles.borrow().is_empty());
    }

    #[test]
    fn a_failed_host_call_fails_its_chunk() {
        let host = Scripted { fail_batch: true, ..Scripted::default() };
        let results = send_all_with(&host, &gets(&["a", "b"]));

        assert!(results.iter().all(|r| matches!(r, Err(Error::Host(_)))));
    }
}
