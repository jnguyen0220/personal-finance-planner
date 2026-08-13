//! Automated tenant messaging. Run from the daily scheduler: renders reminder
//! texts server-side and sends them once per condition (idempotent via
//! `messages.dedup_key`). Add a new `*_messages` source to extend it.

use std::collections::HashMap;

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::dates;
use crate::handlers::properties::outstanding_for;
use crate::sms;
use crate::state::AppState;

struct Pending {
    tenant_id: String,
    property_id: String,
    kind: String,
    to_phone: String,
    body: String,
    dedup_key: String,
}

/// Generates and sends all due automated messages. Each message is reserved
/// idempotently; a condition already handled (same `dedup_key`) is skipped.
pub async fn run(st: &AppState) -> Result<(), sqlx::Error> {
    if !crate::settings::get_bool(&st.pool, crate::settings::MESSAGING_ENABLED, true).await? {
        tracing::info!("automated messaging disabled — skipping run");
        return Ok(());
    }
    for p in collect_pending(st).await? {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let reserved = sqlx::query(
            "INSERT OR IGNORE INTO messages \
             (id, tenant_id, property_id, kind, to_phone, body, status, error, dedup_key, created_at, sent_at) \
             VALUES (?, ?, ?, ?, ?, ?, 'queued', NULL, ?, ?, NULL)",
        )
        .bind(&id)
        .bind(&p.tenant_id)
        .bind(&p.property_id)
        .bind(&p.kind)
        .bind(&p.to_phone)
        .bind(&p.body)
        .bind(&p.dedup_key)
        .bind(&now)
        .execute(&st.pool)
        .await?
        .rows_affected();
        if reserved == 0 {
            continue; // already sent for this condition
        }

        let (status, error, sent_at) = match sms::send(&p.to_phone, &p.body).await {
            Ok(()) => ("sent", None, Some(now)),
            Err(e) => ("failed", Some(e), None),
        };
        sqlx::query("UPDATE messages SET status = ?, error = ?, sent_at = ? WHERE id = ?")
            .bind(status)
            .bind(&error)
            .bind(&sent_at)
            .bind(&id)
            .execute(&st.pool)
            .await?;
    }
    Ok(())
}

/// Registry of automated message sources. Add new reminders here.
async fn collect_pending(st: &AppState) -> Result<Vec<Pending>, sqlx::Error> {
    let mut out = Vec::new();
    out.extend(outstanding_messages(st).await?);
    out.extend(lease_expiring_messages(&st.pool).await?);
    Ok(out)
}

#[derive(sqlx::FromRow)]
struct TenantRow {
    id: String,
    name: String,
    phone: String,
    property_id: String,
    property_name: String,
}

/// Reminds each rental's current tenant of any outstanding balance, at most once
/// per calendar month (the `dedup_key` carries the year-month).
async fn outstanding_messages(st: &AppState) -> Result<Vec<Pending>, sqlx::Error> {
    let rows = sqlx::query_as::<_, TenantRow>(
        "SELECT t.id, trim(t.first_name || ' ' || t.last_name) AS name, t.phone, t.property_id, p.name AS property_name \
         FROM tenants t JOIN properties p ON p.id = t.property_id \
         WHERE t.is_current = 1 AND t.notifications_enabled = 1 AND p.kind = 'rental' AND t.phone <> '' \
         ORDER BY t.created_at",
    )
    .fetch_all(&st.pool)
    .await?;

    // The most recently added current tenant is the one balances are attributed to.
    let mut latest: HashMap<String, TenantRow> = HashMap::new();
    for row in rows {
        latest.insert(row.property_id.clone(), row);
    }

    let year: i32 = chrono::Utc::now()
        .format("%Y")
        .to_string()
        .parse()
        .unwrap_or(1970);
    let month = chrono::Utc::now().format("%Y-%m").to_string();

    let mut out = Vec::new();
    for t in latest.into_values() {
        let balance = outstanding_for(st, &t.property_id, year)
            .await
            .map(|b| b.outstanding)
            .unwrap_or(0.0);
        if balance > 0.005 {
            out.push(Pending {
                kind: "outstanding_balance".into(),
                body: format!(
                    "Hi {}, our records show an outstanding balance of ${:.2} for {} at {}. \
                     Please arrange payment at your earliest convenience. Thank you.",
                    t.name, balance, year, t.property_name
                ),
                dedup_key: format!("msg:outstanding_balance:{}:{}", t.id, month),
                to_phone: t.phone,
                tenant_id: t.id,
                property_id: t.property_id,
            });
        }
    }
    Ok(out)
}

#[derive(sqlx::FromRow)]
struct LeaseRow {
    id: String,
    tenant_id: String,
    tenant_name: String,
    phone: String,
    property_id: String,
    property_name: String,
    end_date: String,
    notify_days: i64,
}

/// Reminds current tenants whose latest lease is ending within its `notify_days`
/// window, once per lease.
async fn lease_expiring_messages(pool: &SqlitePool) -> Result<Vec<Pending>, sqlx::Error> {
    let rows = sqlx::query_as::<_, LeaseRow>(
        "SELECT l.id, l.tenant_id, trim(t.first_name || ' ' || t.last_name) AS tenant_name, t.phone, t.property_id, p.name AS property_name, l.end_date, l.notify_days \
         FROM leases l \
         JOIN tenants t ON t.id = l.tenant_id \
         JOIN properties p ON p.id = t.property_id \
         WHERE t.is_current = 1 AND t.notifications_enabled = 1 AND t.phone <> '' AND l.end_date IS NOT NULL AND l.end_date <> '' \
         ORDER BY l.end_date",
    )
    .fetch_all(pool)
    .await?;

    // Rows ascend by end date, so the last seen per tenant is their latest lease.
    let mut latest: HashMap<String, LeaseRow> = HashMap::new();
    for row in rows {
        latest.insert(row.tenant_id.clone(), row);
    }

    Ok(latest
        .into_values()
        .filter_map(|row| {
            let days = dates::days_until(&row.end_date);
            (days >= 0 && days <= row.notify_days).then(|| Pending {
                kind: "lease_expiring".into(),
                body: format!(
                    "Hi {}, your lease at {} is set to expire on {}. \
                     Please contact us to discuss renewal.",
                    row.tenant_name, row.property_name, row.end_date
                ),
                dedup_key: format!("msg:lease_expiring:{}", row.id),
                to_phone: row.phone,
                tenant_id: row.tenant_id,
                property_id: row.property_id,
            })
        })
        .collect())
}
