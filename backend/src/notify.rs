use std::collections::HashMap;

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::dates;
use crate::handlers::properties::RENT_PAID_PREDICATE;
use crate::models::Notification;
use crate::settings;

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

    let existing = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT id, dedup_key FROM notifications WHERE auto_resolve = 1",
    )
    .fetch_all(pool)
    .await?;
    for (id, key) in existing {
        if key.is_none_or(|k| !keys.contains(&k)) {
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
    out.extend(rent_alerts(pool).await?);
    Ok(out)
}

#[derive(sqlx::FromRow)]
struct PolicyRow {
    id: String,
    property_id: String,
    property_name: String,
    provider: String,
    start_date: Option<String>,
    expiry_date: String,
}

/// Alerts driven by each property's *current* insurance policy (the one in
/// effect today): a warning when it is within its `notify_days` window, or an
/// expired alert once coverage has lapsed and no policy is in effect.
async fn insurance_alerts(pool: &SqlitePool) -> Result<Vec<NewNotification>, sqlx::Error> {
    let notify_days = settings::get_i64(
        pool,
        settings::INSURANCE_NOTIFY_DAYS,
        settings::NOTIFY_DAYS_DEFAULT,
    )
    .await?;
    let rows = sqlx::query_as::<_, PolicyRow>(
        "SELECT i.id, i.property_id, p.name AS property_name, i.provider, i.start_date, i.expiry_date \
         FROM insurance_policies i JOIN properties p ON p.id = i.property_id \
         ORDER BY i.expiry_date",
    )
    .fetch_all(pool)
    .await?;

    let today = dates::today().format("%Y-%m-%d").to_string();
    let mut by_property: HashMap<String, Vec<PolicyRow>> = HashMap::new();
    for row in rows {
        by_property
            .entry(row.property_id.clone())
            .or_default()
            .push(row);
    }

    let mut out = Vec::new();
    for (property_id, policies) in by_property {
        let link = Some(format!("/properties/{property_id}"));
        // The policy in effect today: started, not yet expired, latest start wins.
        let current = policies
            .iter()
            .filter(|p| {
                p.start_date
                    .as_deref()
                    .is_none_or(|s| s <= today.as_str())
                    && p.expiry_date.as_str() >= today.as_str()
            })
            .max_by(|a, b| {
                a.start_date
                    .as_deref()
                    .unwrap_or("")
                    .cmp(b.start_date.as_deref().unwrap_or(""))
            });

        if let Some(p) = current {
            let days = dates::days_until(&p.expiry_date);
            if days <= notify_days {
                out.push(NewNotification {
                    kind: "insurance_expiring".into(),
                    severity: "warning".into(),
                    title: format!("Insurance expiring soon — {}", p.property_name),
                    body: format!("{} policy expires on {}.", p.provider, p.expiry_date),
                    link,
                    property_id: Some(property_id),
                    dedup_key: Some(format!("insurance_expiring:{}", p.id)),
                    auto_resolve: true,
                });
            }
        } else if let Some(p) = policies
            .iter()
            .filter(|p| p.expiry_date.as_str() < today.as_str())
            .max_by(|a, b| a.expiry_date.cmp(&b.expiry_date))
        {
            // No policy is in effect today: coverage has lapsed.
            out.push(NewNotification {
                kind: "insurance_expired".into(),
                severity: "error".into(),
                title: format!("Insurance expired — {}", p.property_name),
                body: format!("{} policy expired on {}.", p.provider, p.expiry_date),
                link,
                property_id: Some(property_id),
                dedup_key: Some(format!("insurance_expired:{}", p.id)),
                auto_resolve: true,
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
    property_id: String,
    property_name: String,
    start_date: Option<String>,
    end_date: Option<String>,
}

/// Alerts driven by each current tenant's *current* lease (the one in effect
/// today): a warning when it is ending within its configured `notify_days`
/// window, or an ended alert once the lease has lapsed and no lease is in effect.
async fn lease_alerts(pool: &SqlitePool) -> Result<Vec<NewNotification>, sqlx::Error> {
    let notify_days = settings::get_i64(
        pool,
        settings::LEASE_NOTIFY_DAYS,
        settings::NOTIFY_DAYS_DEFAULT,
    )
    .await?;
    let rows = sqlx::query_as::<_, LeaseRow>(
        "SELECT l.id, l.tenant_id, trim(t.first_name || ' ' || t.last_name) AS tenant_name, t.property_id, p.name AS property_name, l.start_date, l.end_date \
         FROM leases l \
         JOIN tenants t ON t.id = l.tenant_id \
         JOIN properties p ON p.id = t.property_id \
         WHERE t.is_current = 1 \
         ORDER BY l.start_date",
    )
    .fetch_all(pool)
    .await?;

    let today = dates::today().format("%Y-%m-%d").to_string();
    let mut by_tenant: HashMap<String, Vec<LeaseRow>> = HashMap::new();
    for row in rows {
        by_tenant
            .entry(row.tenant_id.clone())
            .or_default()
            .push(row);
    }

    let mut out = Vec::new();
    for leases in by_tenant.into_values() {
        // The lease in effect today: started, not yet ended, latest start wins.
        let current = leases
            .iter()
            .filter(|l| {
                l.start_date
                    .as_deref()
                    .is_none_or(|s| s <= today.as_str())
                    && l.end_date.as_deref().is_none_or(|e| e >= today.as_str())
            })
            .max_by(|a, b| {
                a.start_date
                    .as_deref()
                    .unwrap_or("")
                    .cmp(b.start_date.as_deref().unwrap_or(""))
            });

        if let Some(l) = current {
            let Some(end_date) = l.end_date.as_deref().filter(|e| !e.is_empty()) else {
                continue; // an open-ended lease never triggers an expiry reminder
            };
            let days = dates::days_until(end_date);
            if (0..=notify_days).contains(&days) {
                out.push(NewNotification {
                    kind: "lease_ending".into(),
                    severity: "warning".into(),
                    title: format!("Lease ending soon — {}", l.tenant_name),
                    body: format!(
                        "{}'s lease at {} ends on {}.",
                        l.tenant_name, l.property_name, end_date
                    ),
                    link: Some(format!("/properties/{}", l.property_id)),
                    property_id: Some(l.property_id.clone()),
                    dedup_key: Some(format!("lease_ending:{}", l.id)),
                    auto_resolve: true,
                });
            }
        } else if let Some(l) = leases
            .iter()
            .filter(|l| l.end_date.as_deref().is_some_and(|e| e < today.as_str()))
            .max_by(|a, b| {
                a.end_date
                    .as_deref()
                    .unwrap_or("")
                    .cmp(b.end_date.as_deref().unwrap_or(""))
            })
        {
            // No lease is in effect today: the tenant's latest lease has ended.
            let end_date = l.end_date.as_deref().unwrap_or_default();
            out.push(NewNotification {
                kind: "lease_expired".into(),
                severity: "warning".into(),
                title: format!("Lease ended — {}", l.tenant_name),
                body: format!(
                    "{}'s lease at {} ended on {}.",
                    l.tenant_name, l.property_name, end_date
                ),
                link: Some(format!("/properties/{}", l.property_id)),
                property_id: Some(l.property_id.clone()),
                dedup_key: Some(format!("lease_expired:{}", l.id)),
                auto_resolve: true,
            });
        }
    }
    Ok(out)
}

#[derive(sqlx::FromRow)]
struct RentDueRow {
    id: String,
    tenant_id: String,
    tenant_name: String,
    property_id: String,
    property_name: String,
    monthly_rent: f64,
    rent_due_day: i64,
    start_date: Option<String>,
    end_date: Option<String>,
}

/// Alerts for current tenants whose rent for the current month is past its
/// due day and still unpaid. Keyed per month so a fresh alert is raised each
/// month and cleared once payment lands or the month rolls over.
async fn rent_alerts(pool: &SqlitePool) -> Result<Vec<NewNotification>, sqlx::Error> {
    let rows = sqlx::query_as::<_, RentDueRow>(
        "SELECT l.id, l.tenant_id, trim(t.first_name || ' ' || t.last_name) AS tenant_name, \
                t.property_id, p.name AS property_name, l.monthly_rent, l.rent_due_day, \
                l.start_date, l.end_date \
         FROM leases l \
         JOIN tenants t ON t.id = l.tenant_id \
         JOIN properties p ON p.id = t.property_id \
         WHERE t.is_current = 1 AND l.rent_due_day IS NOT NULL AND l.monthly_rent > 0 \
         ORDER BY l.start_date",
    )
    .fetch_all(pool)
    .await?;

    // Keep only the lease covering the current month, latest start wins per tenant.
    let current_m = dates::current_month_index();
    let mut active: HashMap<String, RentDueRow> = HashMap::new();
    for row in rows {
        let start_m = row.start_date.as_deref().and_then(dates::month_index);
        let end_m = row.end_date.as_deref().and_then(dates::month_index);
        let covers =
            start_m.is_some_and(|s| s <= current_m) && end_m.is_none_or(|e| e >= current_m);
        if !covers {
            continue;
        }
        let this_start = start_m.unwrap_or(i32::MIN);
        let keep = active
            .get(&row.tenant_id)
            .and_then(|ex| ex.start_date.as_deref().and_then(dates::month_index))
            .is_none_or(|ex_start| this_start >= ex_start);
        if keep {
            active.insert(row.tenant_id.clone(), row);
        }
    }
    if active.is_empty() {
        return Ok(Vec::new());
    }

    // Rent credited toward the current month, per property.
    let month_prefix = dates::today().format("%Y-%m").to_string();
    let paid_rows = sqlx::query_as::<_, (String, f64)>(&format!(
        "SELECT t.property_id, CAST(COALESCE(SUM(t.amount), 0) AS REAL) \
         FROM transactions t \
         JOIN categories c ON c.id = t.category_id \
         WHERE substr(t.date, 1, 7) = ? AND ({RENT_PAID_PREDICATE}) \
         GROUP BY t.property_id"
    ))
    .bind(&month_prefix)
    .fetch_all(pool)
    .await?;
    let paid: HashMap<String, f64> = paid_rows.into_iter().collect();

    let today_day = dates::current_day_of_month();
    Ok(active
        .into_values()
        .filter_map(|row| {
            if today_day <= row.rent_due_day {
                return None;
            }
            let paid_amt = paid.get(&row.property_id).copied().unwrap_or(0.0);
            let remaining = row.monthly_rent - paid_amt;
            if remaining <= 0.005 {
                return None;
            }
            Some(NewNotification {
                kind: "rent_overdue".into(),
                severity: "warning".into(),
                title: format!("Rent overdue — {}", row.tenant_name),
                body: format!(
                    "{}'s rent at {} was due on day {} and ${:.2} is still unpaid this month.",
                    row.tenant_name, row.property_name, row.rent_due_day, remaining
                ),
                link: Some(format!("/properties/{}", row.property_id)),
                property_id: Some(row.property_id),
                dedup_key: Some(format!("rent_overdue:{}:{}", row.id, month_prefix)),
                auto_resolve: true,
            })
        })
        .collect())
}
