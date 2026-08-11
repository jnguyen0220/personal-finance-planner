use axum::body::Body;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;

/// Computes a weak ETag (content hash) for a response body. Uses FNV-1a so the
/// value is stable across restarts and dependency-free.
pub fn weak_etag(body: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in body.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("W/\"{hash:016x}\"")
}

/// Serves slow-changing JSON with ETag revalidation: returns 304 when the
/// client's `If-None-Match` matches the current hash, otherwise 200 with the
/// body. Clients therefore refetch only when the hash changes.
pub fn respond_json(headers: &HeaderMap, body: String) -> Response {
    let etag = weak_etag(&body);

    let matched = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(',').any(|tag| tag.trim() == etag))
        .unwrap_or(false);

    if matched {
        return Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, &etag)
            .header(header::CACHE_CONTROL, "no-cache")
            .body(Body::empty())
            .expect("build 304 response");
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ETAG, &etag)
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(body))
        .expect("build json response")
}
