use std::collections::HashMap;

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::dates;
use crate::models::Notification;

/// A notification to persist. `dedup_key` keeps derived alerts idempotent, and
/// `auto_resolve` lets reconciliation remove it once its condition clears.
pub struct NewNotification {
    pub kind: String,
    pub severity: String,
    pub title: String,
    pub body: String,
    pub link: Option<String>,
    pub property_id: Option<String>,
    pub dedup_key: Option<String>,
    pub auto_resolve: bool,
}

/// Inserts a notification, ignoring duplicates that share a `dedup_key`. This is
/// the entry point for any event-driven notification the application raises.
pub async fn create(pool: &SqlitePool, n: NewNotification) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT OR IGNORE INTO notifications \
         (id, kind, severity, title, body, link, property_id, dedup_key, auto_resolve, created_at, dismissed_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(&id)
    .bind(&n.kind)
    .bind(&n.severity)
    .bind(&n.title)
    .bind(&n.body)
    .bind(&n.link)
    .bind(&n.property_id)
    .bind(&n.dedup_key)
    .bind(n.auto_resolve as i64)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Active (non-dismissed) notifications, most severe and most recent first.
pub async fn list_active(pool: &SqlitePool) -> Result<Vec<Notification>, sqlx::Error> {
    sqlx::query_as::<_, Notification>(
        "SELECT id, kind, severity, title, body, link, property_id, created_at \
         FROM notifications WHERE dismissed_at IS NULL \
         ORDER BY CASE severity WHEN 'error' THEN 0 WHEN 'warning' THEN 1 ELSE 2 END, created_at DESC",
    )
    .fetch_all(pool)
    .await
}

/// Marks a notification dismissed. Idempotent: dismissing twice is a no-op.
pub async fn dismiss(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE notifications SET dismissed_at = ? WHERE id = ? AND dismissed_at IS NULL")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Rebuilds time-derived notifications so the store reflects current domain
/// state. New alerts are inserted; auto-resolving ones whose condition no longer
/// holds (e.g. a renewed policy) are removed. Dismissals survive because
/// existing rows are matched by `dedup_key`. Run by the daily scheduler and
/// lazily on read.
pub async fn reconcile(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let derived = collect_derived(pool).await?;
    let keys: Vec<String> = derived.iter().filter_map(|n| n.dedup_key.clone()).collect();

    for n in derived {
        create(pool, n).await?;
    }

    let existing =
        sqlx::query_as::<_, (String, Option<String>)>(
            "SELECT id, dedup_key FROM notifications WHERE auto_resolve = 1",
        )
        .fetch_all(pool)
        .await?;
    for (id, key) in existing {
        if key.map_or(true, |k| !keys.contains(&k)) {
            sqlx::query("DELETE FROM notifications WHERE id = ?")
                .bind(&id)
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}

/// Registry of notification sources. To add a new time-derived notification
/// (e.g. a tenant document expiring), write another `*_alerts` function and add
/// it here — the rest of the pipeline needs no changes.
async fn collect_derived(pool: &SqlitePool) -> Result<Vec<NewNotification>, sqlx::Error> {
    let mut out = Vec::new();
    out.extend(insurance_alerts(pool).await?);
    out.extend(lease_alerts(pool).await?);
    Ok(out)
}

#[derive(sqlx::FromRow)]
struct PolicyRow {
    id: String,
    property_id: String,
    property_name: String,
    provider: String,
    expiry_date: String,
    notify_days: i64,
}

/// Alerts for each property whose latest insurance policy is expired or nearing
/// expiry within its configured `notify_days` window.
async fn insurance_alerts(pool: &SqlitePool) -> Result<Vec<NewNotification>, sqlx::Error> {
    let rows = sqlx::query_as::<_, PolicyRow>(
        "SELECT i.id, i.property_id, p.name AS property_name, i.provider, i.expiry_date, i.notify_days \
         FROM insurance_policies i JOIN properties p ON p.id = i.property_id \
         ORDER BY i.expiry_date",
    )
    .fetch_all(pool)
    .await?;

    // Rows ascend by expiry, so the last seen per property is its latest policy.
    let mut latest: HashMap<String, PolicyRow> = HashMap::new();
    for row in rows {
        latest.insert(row.property_id.clone(), row);
    }

    Ok(latest
        .into_values()
        .filter_map(|row| {
            let days = dates::days_until(&row.expiry_date);
            let link = Some(format!("/properties/{}", row.property_id));
            if days < 0 {
                Some(NewNotification {
                    kind: "insurance_expired".into(),
                    severity: "error".into(),
                    title: format!("Insurance expired — {}", row.property_name),
                    body: format!("{} policy expired on {}.", row.provider, row.expiry_date),
                    link,
                    property_id: Some(row.property_id),
                    dedup_key: Some(format!("insurance_expired:{}", row.id)),
                    auto_resolve: true,
                })
            } else if days <= row.notify_days {
                Some(NewNotification {
                    kind: "insurance_expiring".into(),
                    severity: "warning".into(),
                    title: format!("Insurance expiring soon — {}", row.property_name),
                    body: format!("{} policy expires on {}.", row.provider, row.expiry_date),
                    link,
                    property_id: Some(row.property_id),
                    dedup_key: Some(format!("insurance_expiring:{}", row.id)),
                    auto_resolve: true,
                })
            } else {
                None
            }
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct LeaseRow {
    id: String,
    tenant_id: String,
    tenant_name: String,
    property_id: String,
    property_name: String,
    end_date: String,
    notify_days: i64,
}

/// Alerts for current tenants whose latest lease has ended or is ending within
/// its configured `notify_days` window.
async fn lease_alerts(pool: &SqlitePool) -> Result<Vec<NewNotification>, sqlx::Error> {
    let rows = sqlx::query_as::<_, LeaseRow>(
        "SELECT l.id, l.tenant_id, t.name AS tenant_name, t.property_id, p.name AS property_name, l.end_date, l.notify_days \
         FROM leases l \
         JOIN tenants t ON t.id = l.tenant_id \
         JOIN properties p ON p.id = t.property_id \
         WHERE t.is_current = 1 AND l.end_date IS NOT NULL AND l.end_date <> '' \
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
            let link = Some(format!("/properties/{}", row.property_id));
            if days < 0 {
                Some(NewNotification {
                    kind: "lease_expired".into(),
                    severity: "warning".into(),
                    title: format!("Lease ended — {}", row.tenant_name),
                    body: format!(
                        "{}'s lease at {} ended on {}.",
                        row.tenant_name, row.property_name, row.end_date
                    ),
                    link,
                    property_id: Some(row.property_id),
                    dedup_key: Some(format!("lease_expired:{}", row.id)),
                    auto_resolve: true,
                })
            } else if days <= row.notify_days {
                Some(NewNotification {
                    kind: "lease_ending".into(),
                    severity: "warning".into(),
                    title: format!("Lease ending soon — {}", row.tenant_name),
                    body: format!(
                        "{}'s lease at {} ends on {}.",
                        row.tenant_name, row.property_name, row.end_date
                    ),
                    link,
                    property_id: Some(row.property_id),
                    dedup_key: Some(format!("lease_ending:{}", row.id)),
                    auto_resolve: true,
                })
            } else {
                None
            }
        })
        .collect())
}
