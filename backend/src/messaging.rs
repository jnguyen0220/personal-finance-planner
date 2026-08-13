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
    if crate::settings::get_bool(&st.pool, crate::settings::MESSAGING_ENABLED, true).await? {
        send_tenant_reminders(st).await?;
    } else {
        tracing::info!("tenant messaging disabled — skipping tenant reminders");
    }

    if crate::settings::get_bool(&st.pool, crate::settings::PROPERTY_MESSAGING_ENABLED, true)
        .await?
    {
        contact_reminders(st).await?;
    } else {
        tracing::info!("property messaging disabled — skipping contact reminders");
    }
    Ok(())
}

/// Sends the due tenant reminders, reserving each idempotently so a condition
/// already handled (same `dedup_key`) is skipped.
async fn send_tenant_reminders(st: &AppState) -> Result<(), sqlx::Error> {
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

/// Texts the operator's contact phones about lease and insurance expiry, reusing
/// the notifications already computed by `notify::reconcile` and sending each
/// condition once (idempotent via the `contact_reminders` ledger).
async fn contact_reminders(st: &AppState) -> Result<(), sqlx::Error> {
    let phones = crate::settings::get_list(&st.pool, crate::settings::CONTACT_PHONES).await?;
    if phones.is_empty() {
        return Ok(());
    }

    let alerts = sqlx::query_as::<_, (Option<String>, String, String, String)>(
        "SELECT dedup_key, kind, title, body FROM notifications \
         WHERE dismissed_at IS NULL AND dedup_key IS NOT NULL \
           AND kind IN ('lease_ending', 'lease_expired', 'insurance_expiring', 'insurance_expired')",
    )
    .fetch_all(&st.pool)
    .await?;

    let lease_template = crate::templates::body(&st.pool, "landlord_lease").await?;
    let insurance_template = crate::templates::body(&st.pool, "landlord_insurance").await?;

    for (dedup_key, kind, title, body) in alerts {
        let Some(dedup_key) = dedup_key else { continue };
        let key = format!("contact:{dedup_key}");
        let now = chrono::Utc::now().to_rfc3339();
        let reserved = sqlx::query(
            "INSERT OR IGNORE INTO contact_reminders (dedup_key, created_at) VALUES (?, ?)",
        )
        .bind(&key)
        .bind(&now)
        .execute(&st.pool)
        .await?
        .rows_affected();
        if reserved == 0 {
            continue; // already texted for this condition
        }
        let template = if kind.starts_with("insurance") {
            &insurance_template
        } else {
            &lease_template
        };
        let alert = format!("{title}\n{body}");
        let message = crate::templates::render(template, &[("alert", alert)]);
        for phone in &phones {
            if let Err(e) = sms::send(phone, &message).await {
                tracing::error!("contact reminder to {phone} failed: {e}");
            }
        }
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
    address: String,
    city: String,
    state: String,
    zip: String,
}

/// Reminds each rental's current tenant of any outstanding balance, at most once
/// per calendar month (the `dedup_key` carries the year-month).
async fn outstanding_messages(st: &AppState) -> Result<Vec<Pending>, sqlx::Error> {
    let rows = sqlx::query_as::<_, TenantRow>(
        "SELECT t.id, trim(t.first_name || ' ' || t.last_name) AS name, t.phone, t.property_id, p.address, p.city, p.state, p.zip \
         FROM tenants t JOIN properties p ON p.id = t.property_id \
         WHERE t.is_current = 1 AND p.reminders_enabled = 1 AND p.kind = 'rental' AND t.phone <> '' \
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
    let template = crate::templates::body(&st.pool, "outstanding_balance").await?;
    let signature = crate::templates::signature_value(&st.pool).await?;

    let mut out = Vec::new();
    for t in latest.into_values() {
        let balance = outstanding_for(st, &t.property_id, year)
            .await
            .map(|b| b.outstanding)
            .unwrap_or(0.0);
        if balance > 0.005 {
            out.push(Pending {
                kind: "outstanding_balance".into(),
                body: crate::templates::render(
                    &template,
                    &[
                        ("tenant_name", t.name.clone()),
                        ("address", t.address.clone()),
                        ("city", t.city.clone()),
                        ("state", t.state.clone()),
                        ("zip", t.zip.clone()),
                        ("balance", format!("${:.2}", balance)),
                        ("year", year.to_string()),
                        ("signature", signature.clone()),
                    ],
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
    address: String,
    city: String,
    state: String,
    zip: String,
    end_date: String,
}

/// Reminds current tenants whose latest lease is ending within its `notify_days`
/// window, once per lease.
async fn lease_expiring_messages(pool: &SqlitePool) -> Result<Vec<Pending>, sqlx::Error> {
    let notify_days = crate::settings::get_i64(
        pool,
        crate::settings::LEASE_NOTIFY_DAYS,
        crate::settings::NOTIFY_DAYS_DEFAULT,
    )
    .await?;
    let rows = sqlx::query_as::<_, LeaseRow>(
        "SELECT l.id, l.tenant_id, trim(t.first_name || ' ' || t.last_name) AS tenant_name, t.phone, t.property_id, p.address, p.city, p.state, p.zip, l.end_date \
         FROM leases l \
         JOIN tenants t ON t.id = l.tenant_id \
         JOIN properties p ON p.id = t.property_id \
         WHERE t.is_current = 1 AND p.reminders_enabled = 1 AND t.phone <> '' AND l.end_date IS NOT NULL AND l.end_date <> '' \
         ORDER BY l.end_date",
    )
    .fetch_all(pool)
    .await?;

    // Rows ascend by end date, so the last seen per tenant is their latest lease.
    let mut latest: HashMap<String, LeaseRow> = HashMap::new();
    for row in rows {
        latest.insert(row.tenant_id.clone(), row);
    }

    let template = crate::templates::body(pool, "lease_expiring").await?;
    let signature = crate::templates::signature_value(pool).await?;
    Ok(latest
        .into_values()
        .filter_map(|row| {
            let days = dates::days_until(&row.end_date);
            (days >= 0 && days <= notify_days).then(|| Pending {
                kind: "lease_expiring".into(),
                body: crate::templates::render(
                    &template,
                    &[
                        ("tenant_name", row.tenant_name.clone()),
                        ("address", row.address.clone()),
                        ("city", row.city.clone()),
                        ("state", row.state.clone()),
                        ("zip", row.zip.clone()),
                        ("end_date", row.end_date.clone()),
                        ("signature", signature.clone()),
                    ],
                ),
                dedup_key: format!("msg:lease_expiring:{}", row.id),
                to_phone: row.phone,
                tenant_id: row.tenant_id,
                property_id: row.property_id,
            })
        })
        .collect())
}
