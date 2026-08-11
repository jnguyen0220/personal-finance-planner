use axum::http::HeaderMap;
use axum::response::Response;

use crate::etag;
use crate::states::STATES;

pub async fn list(headers: HeaderMap) -> Response {
    let body = serde_json::to_string(STATES).expect("serialize states");
    etag::respond_json(&headers, body)
}
