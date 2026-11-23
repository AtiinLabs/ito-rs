# Sending several requests at once

`Request::send_all` sends up to four read-only requests concurrently, so a paged chapter list or a
home page with several sections takes about one round trip instead of one per request. It needs a
host that provides the `ito:core/net.fetch_batch` import. A host checks a plugin's imports before
loading it, so a plugin that calls `send_all` must set `min_app_version` to the first app release
that has it. A plugin that never calls `send_all` doesn't import `fetch_batch` and keeps loading on
older hosts.

```rust,ignore
use ito_rs::net::Request;

let pages: Vec<Request> = (1..=9)
    .map(|page| Request::get(format!("https://example.org/chapters?page={page}")))
    .collect();

// One result per request, in the same order as `pages`.
for result in Request::send_all(&pages) {
    let response = result?;
    // parse `response.body`
}
```

## Rules

- **Order.** Result `i` answers request `i`, never completion order. An empty slice returns an
  empty `Vec` without calling the host.
- **Safe methods only.** `GET`, `HEAD` and `OPTIONS` (any case). Any other method fails that
  element with `Error::Net` and is not sent; the rest of the call is unaffected.
- **Chunking.** The SDK sends at most `MAX_BATCH` (4) requests per host call, so nine requests
  go out as 4 + 4 + 1.
- **Failures are per element.** A request that never reached the server (no connection, timed
  out, cancelled, refused by the plugin's network policy) is that element's `Error::Net`; the other
  elements keep their responses. If the host's answer can't be decoded or doesn't hold one response
  per request, every element of that chunk is an `Error::Host`.
- **Response budget.** The host keeps one batch's response bodies within 32 MiB. An element that
  would exceed it is retried by the SDK on its own with `send()`, so callers never see that failure.
- **Cookies.** Requests in one call run concurrently and don't see each other's cookies. Don't
  batch requests that depend on an earlier response's cookies.
- **Rate limits.** Don't batch against sites that rate-limit; four requests arrive together.

## Wire layout

Import `ito:core/net.fetch_batch(ptr: i32, len: i32) -> i32`. The input is a postcard
`Vec<NetRequest>` (the `fetch_v2` request layout, without `NetOptions`). The return value is the
byte length of a postcard `Vec<NetResponse>` that the plugin copies with `fetch_read`. A host
traps on an empty batch or one larger than `MAX_BATCH`.
