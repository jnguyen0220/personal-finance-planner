use axum::extract::Query;
use axum::http::HeaderMap;
use axum::response::Response;
use serde::Deserialize;

use crate::categories::resolved_for;
use crate::etag;

#[derive(Deserialize)]
pub struct CategoriesQuery {
    pub kind: Option<String>,
}

pub async fn list(headers: HeaderMap, Query(q): Query<CategoriesQuery>) -> Response {
    let kind = q.kind.as_deref().unwrap_or("rental");
    let body = serde_json::to_string(&resolved_for(kind)).expect("serialize categories");
    etag::respond_json(&headers, body)
}
