mod categories;
mod dates;
mod db;
mod error;
mod etag;
mod handlers;
mod messaging;
mod models;
mod notify;
mod settings;
mod sms;
mod state;
mod states;
mod templates;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post, put};
use axum::Router;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::trace::TraceLayer;

use state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,tower_http=info".into()),
        )
        .init();

    let data_dir = PathBuf::from(std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string()));
    let uploads = data_dir.join("uploads");
    std::fs::create_dir_all(&uploads).expect("failed to create uploads dir");

    let db_path = data_dir.join("app.db");
    let pool = db::init_pool(db_path.to_str().expect("invalid db path"))
        .await
        .expect("failed to initialise database");

    let state = AppState { pool, uploads };

    // Regenerate expiry notifications on startup and once a day thereafter.
    let scheduler_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(24 * 60 * 60));
        loop {
            ticker.tick().await;
            if let Err(e) = notify::reconcile(&scheduler_state.pool).await {
                tracing::error!("daily notification reconcile failed: {e}");
            }
            if let Err(e) = messaging::run(&scheduler_state).await {
                tracing::error!("daily automated messaging failed: {e}");
            }
        }
    });

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/categories", get(handlers::categories::list))
        .route("/api/states", get(handlers::states::list))
        .route(
            "/api/settings",
            get(handlers::settings::get).put(handlers::settings::update),
        )
        .route("/api/message-templates", get(handlers::templates::list))
        .route(
            "/api/message-templates/:kind",
            put(handlers::templates::update),
        )
        .route("/api/notifications", get(handlers::notifications::list))
        .route(
            "/api/notifications/:id/dismiss",
            post(handlers::notifications::dismiss),
        )
        .route("/api/properties", post(handlers::properties::create))
        .route("/api/overview", get(handlers::properties::overview))
        .route("/api/years", get(handlers::properties::years))
        .route("/api/tax-report", get(handlers::properties::tax_report))
        .route(
            "/api/properties/:id",
            get(handlers::properties::get)
                .put(handlers::properties::update)
                .delete(handlers::properties::delete),
        )
        .route(
            "/api/properties/:id/summary",
            get(handlers::properties::summary),
        )
        .route(
            "/api/properties/:id/breakdown",
            get(handlers::properties::breakdown),
        )
        .route(
            "/api/properties/:id/outstanding",
            get(handlers::properties::outstanding),
        )
        .route(
            "/api/properties/:id/tenants",
            get(handlers::tenants::list).post(handlers::tenants::create),
        )
        .route(
            "/api/tenants/:id",
            put(handlers::tenants::update).delete(handlers::tenants::delete),
        )
        .route("/api/tenants/:id/leases", post(handlers::leases::create))
        .route(
            "/api/leases/:id",
            put(handlers::leases::update).delete(handlers::leases::delete),
        )
        .route(
            "/api/properties/:id/transactions",
            get(handlers::transactions::list_for_property).post(handlers::transactions::create),
        )
        .route(
            "/api/transactions/:id",
            put(handlers::transactions::update).delete(handlers::transactions::delete),
        )
        .route(
            "/api/properties/:id/insurance",
            get(handlers::insurance::list_for_property).post(handlers::insurance::create),
        )
        .route(
            "/api/insurance/:id",
            put(handlers::insurance::update).delete(handlers::insurance::delete),
        )
        .route(
            "/api/properties/:id/messages",
            get(handlers::messages::list_for_property),
        )
        .route("/api/broadcast", post(handlers::messages::broadcast))
        .route(
            "/api/broadcast/recipients",
            get(handlers::messages::recipients),
        )
        .route(
            "/api/tenants/:id/messages",
            post(handlers::messages::create),
        )
        .route(
            "/api/tenants/:id/messages/providers",
            post(handlers::messages::send_providers),
        )
        .route(
            "/api/properties/:id/providers/message",
            get(handlers::messages::preview_providers),
        )
        .route(
            "/api/properties/:id/providers",
            get(handlers::providers::list_for_property).post(handlers::providers::create),
        )
        .route(
            "/api/providers/:id",
            put(handlers::providers::update).delete(handlers::providers::delete),
        )
        .route(
            "/api/attachments",
            post(handlers::attachments::upload).layer(DefaultBodyLimit::max(20 * 1024 * 1024)),
        )
        .route("/api/attachments/:id", get(handlers::attachments::download))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let host: std::net::IpAddr = std::env::var("HOST")
        .ok()
        .and_then(|h| h.parse().ok())
        .unwrap_or_else(|| [0, 0, 0, 0].into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::new(host, port);
    tracing::info!("backend listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "status": "ok" }))
}
