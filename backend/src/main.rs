mod categories;
mod dates;
mod db;
mod error;
mod etag;
mod gmail;
mod handlers;
mod http;
mod logs;
mod messaging;
mod models;
mod notify;
mod options;
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

    let state = AppState {
        pool,
        uploads,
        started_at: chrono::Utc::now(),
    };

    // Regenerate expiry notifications and poll Gmail for inbound invoices on
    // startup and once a day thereafter. Each job can be disabled at runtime.
    let scheduler_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(24 * 60 * 60));
        loop {
            ticker.tick().await;
            let reminders_on = settings::get_bool(
                &scheduler_state.pool,
                settings::DAILY_REMINDERS_ENABLED,
                true,
            )
            .await
            .unwrap_or(true);
            if reminders_on {
                match notify::reconcile(&scheduler_state.pool).await {
                    Ok(()) => logs::clear_failure(&scheduler_state.pool, "notifications").await,
                    Err(e) => {
                        logs::record_failure(
                            &scheduler_state.pool,
                            "notifications",
                            &format!("daily notification reconcile failed: {e}"),
                        )
                        .await
                    }
                }
                match messaging::run(&scheduler_state).await {
                    Ok(()) => logs::clear_failure(&scheduler_state.pool, "messaging").await,
                    Err(e) => {
                        logs::record_failure(
                            &scheduler_state.pool,
                            "messaging",
                            &format!("daily automated messaging failed: {e}"),
                        )
                        .await
                    }
                }
            }
            let email_on =
                settings::get_bool(&scheduler_state.pool, settings::DAILY_EMAIL_ENABLED, true)
                    .await
                    .unwrap_or(true);
            if email_on && gmail::configured() {
                match handlers::inbox::poll_and_ingest(&scheduler_state).await {
                    Ok(_) => logs::clear_failure(&scheduler_state.pool, "gmail_poll").await,
                    Err(e) => {
                        logs::record_failure(
                            &scheduler_state.pool,
                            "gmail_poll",
                            &format!("daily gmail poll failed: {e}"),
                        )
                        .await
                    }
                }
            }
        }
    });

    if !gmail::configured() {
        tracing::info!(
            "Gmail polling disabled — set GMAIL_CLIENT_ID, GMAIL_CLIENT_SECRET and GMAIL_REFRESH_TOKEN to enable"
        );
    }

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/categories", get(handlers::categories::list))
        .route(
            "/api/admin/categories",
            get(handlers::categories::list_all).post(handlers::categories::create),
        )
        .route(
            "/api/admin/categories/:name",
            put(handlers::categories::update).delete(handlers::categories::delete),
        )
        .route(
            "/api/option-lists/:list",
            get(handlers::options::get).put(handlers::options::put),
        )
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
        .route(
            "/api/admin/logs",
            get(handlers::logs::list).delete(handlers::logs::clear),
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
        .route("/api/inbox", get(handlers::inbox::list))
        .route("/api/inbox/status", get(handlers::inbox::status))
        .route("/api/inbox/poll", post(handlers::inbox::poll))
        .route("/api/inbox/:id/assign", post(handlers::inbox::assign))
        .route("/api/inbox/:id/dismiss", post(handlers::inbox::dismiss))
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
