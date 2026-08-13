use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::templates;

#[derive(Serialize)]
struct PlaceholderView {
    token: String,
    description: String,
}

#[derive(Serialize)]
pub struct TemplateView {
    kind: String,
    label: String,
    description: String,
    group: String,
    placeholders: Vec<PlaceholderView>,
    /// Effective body: the stored override, or the built-in default.
    body: String,
    default_body: String,
    /// Whether a non-default override is currently stored.
    is_custom: bool,
}

#[derive(Deserialize)]
pub struct TemplateUpdate {
    pub body: String,
}

async fn view(st: &AppState, def: &templates::TemplateDef) -> AppResult<TemplateView> {
    Ok(TemplateView {
        kind: def.kind.to_string(),
        label: def.label.to_string(),
        description: def.description.to_string(),
        group: def.group.to_string(),
        placeholders: def
            .placeholders
            .iter()
            .map(|p| PlaceholderView {
                token: p.token.to_string(),
                description: p.description.to_string(),
            })
            .collect(),
        body: templates::body(&st.pool, def.kind).await?,
        default_body: def.default_body.to_string(),
        is_custom: templates::is_custom(&st.pool, def.kind).await?,
    })
}

pub async fn list(State(st): State<AppState>) -> AppResult<Json<Vec<TemplateView>>> {
    let mut out = Vec::with_capacity(templates::TEMPLATES.len());
    for def in templates::TEMPLATES {
        out.push(view(&st, def).await?);
    }
    Ok(Json(out))
}

pub async fn update(
    State(st): State<AppState>,
    Path(kind): Path<String>,
    Json(input): Json<TemplateUpdate>,
) -> AppResult<Json<TemplateView>> {
    let def = templates::find(&kind).ok_or(AppError::NotFound)?;
    templates::set_body(&st.pool, def.kind, &input.body).await?;
    Ok(Json(view(&st, def).await?))
}
