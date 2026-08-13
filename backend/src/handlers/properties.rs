use axum::extract::{Path, Query, State};
use axum::Json;
use std::collections::HashMap;
use uuid::Uuid;

use crate::dates::{current_day_of_month, current_month_index, current_year, month_index};
use crate::error::{AppError, AppResult};
use crate::handlers::{delete_attachment, delete_by_id};
use crate::models::{
    CategoryTotal, OutstandingBalance, OverviewResponse, OverviewRow, PortfolioTotals, Property,
    PropertyInput, PropertySummary, SummaryQuery, TaxCategoryTotal, TaxPropertyReport, TaxReport,
};
use crate::state::AppState;

const COLUMNS: &str = "id, name, address, city, state, zip, kind, reminders_enabled, purchase_date, notes, hoa_name, hoa_phone, hoa_email, hoa_webpage, created_at";

/// Rent credited in a year: income categories flagged `counts_as_rent`, plus any
/// tenant-borne expense (which the tenant deducts from rent). Single source of
/// truth for the "paid" figure. Assumes `transactions t` JOIN `categories c`.
pub(crate) const RENT_PAID_PREDICATE: &str = "(c.counts_as_rent = 1 OR t.borne_by = 'tenant')";

pub async fn get(State(st): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Property>> {
    let row =
        sqlx::query_as::<_, Property>(&format!("SELECT {COLUMNS} FROM properties WHERE id = ?"))
            .bind(&id)
            .fetch_optional(&st.pool)
            .await?
            .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

pub async fn create(
    State(st): State<AppState>,
    Json(input): Json<PropertyInput>,
) -> AppResult<Json<Property>> {
    if input.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    validate_zip(&input.zip)?;
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let row = sqlx::query_as::<_, Property>(&format!(
        "INSERT INTO properties (id, name, address, city, state, zip, kind, reminders_enabled, purchase_date, notes, hoa_name, hoa_phone, hoa_email, hoa_webpage, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING {COLUMNS}"
    ))
    .bind(&id)
    .bind(input.name.trim())
    .bind(&input.address)
    .bind(&input.city)
    .bind(&input.state)
    .bind(&input.zip)
    .bind(&input.kind)
    .bind(input.reminders_enabled)
    .bind(&input.purchase_date)
    .bind(&input.notes)
    .bind(&input.hoa_name)
    .bind(&input.hoa_phone)
    .bind(&input.hoa_email)
    .bind(&input.hoa_webpage)
    .bind(&now)
    .fetch_one(&st.pool)
    .await?;
    Ok(Json(row))
}

pub async fn update(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<PropertyInput>,
) -> AppResult<Json<Property>> {
    validate_zip(&input.zip)?;
    let row = sqlx::query_as::<_, Property>(&format!(
        "UPDATE properties SET name = ?, address = ?, city = ?, state = ?, zip = ?, kind = ?, reminders_enabled = ?, purchase_date = ?, notes = ?, hoa_name = ?, hoa_phone = ?, hoa_email = ?, hoa_webpage = ? \
         WHERE id = ? RETURNING {COLUMNS}"
    ))
    .bind(input.name.trim())
    .bind(&input.address)
    .bind(&input.city)
    .bind(&input.state)
    .bind(&input.zip)
    .bind(&input.kind)
    .bind(input.reminders_enabled)
    .bind(&input.purchase_date)
    .bind(&input.notes)
    .bind(&input.hoa_name)
    .bind(&input.hoa_phone)
    .bind(&input.hoa_email)
    .bind(&input.hoa_webpage)
    .bind(&id)
    .fetch_optional(&st.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

/// Zip is optional; when present it must be exactly 5 digits.
fn validate_zip(zip: &str) -> AppResult<()> {
    if zip.is_empty() {
        return Ok(());
    }
    if zip.len() == 5 && zip.bytes().all(|b| b.is_ascii_digit()) {
        Ok(())
    } else {
        Err(AppError::BadRequest("zip must be 5 digits".into()))
    }
}

pub async fn delete(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<axum::http::StatusCode> {
    // Cascade removes the property's transactions and tenants, so collect their
    // uploads first and delete them once those rows are gone.
    let attachment_ids = sqlx::query_scalar::<_, String>(
        "SELECT receipt_id FROM transactions WHERE property_id = ?1 AND receipt_id IS NOT NULL \
         UNION \
         SELECT driver_license_id FROM tenants WHERE property_id = ?1 AND driver_license_id IS NOT NULL",
    )
    .bind(&id)
    .fetch_all(&st.pool)
    .await?;
    let status = delete_by_id(&st, "properties", &id).await?;
    for aid in attachment_ids {
        delete_attachment(&st, &aid).await?;
    }
    Ok(status)
}

pub async fn summary(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<SummaryQuery>,
) -> AppResult<Json<PropertySummary>> {
    let year = validate_year(&q.year)?;
    let row = sqlx::query_as::<_, PropertySummary>(
        "SELECT \
            CAST(COALESCE(SUM(CASE WHEN kind = 'income'  THEN amount ELSE 0 END), 0) AS REAL) AS total_income, \
            CAST(COALESCE(SUM(CASE WHEN kind = 'expense' THEN amount ELSE 0 END), 0) AS REAL) AS total_expense \
         FROM transactions \
         WHERE property_id = ?1 AND (?2 IS NULL OR substr(date, 1, 4) = ?2)",
    )
    .bind(&id)
    .bind(year)
    .fetch_one(&st.pool)
    .await?;
    Ok(Json(row))
}

pub async fn breakdown(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<SummaryQuery>,
) -> AppResult<Json<Vec<CategoryTotal>>> {
    let year = validate_year(&q.year)?;
    let rows = sqlx::query_as::<_, CategoryTotal>(
        "SELECT t.kind, c.label AS category, CAST(SUM(t.amount) AS REAL) AS total \
         FROM transactions t \
         JOIN categories c ON c.id = t.category_id \
         WHERE t.property_id = ?1 AND (?2 IS NULL OR substr(t.date, 1, 4) = ?2) \
         GROUP BY t.kind, c.id \
         ORDER BY t.kind, total DESC",
    )
    .bind(&id)
    .bind(year)
    .fetch_all(&st.pool)
    .await?;
    Ok(Json(rows))
}

/// Optional year filter (e.g. tax year); when present it must be 4 digits.
fn validate_year(year: &Option<String>) -> AppResult<Option<String>> {
    match year {
        Some(y) if y.len() == 4 && y.bytes().all(|b| b.is_ascii_digit()) => Ok(Some(y.clone())),
        Some(_) => Err(AppError::BadRequest("year must be 4 digits".into())),
        None => Ok(None),
    }
}

/// Rent owed by the current tenant across their leases: accumulated
/// (annual rent − rent paid) per year, carried over into the selected year.
pub async fn outstanding(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<SummaryQuery>,
) -> AppResult<Json<OutstandingBalance>> {
    let selected_year: i32 = match validate_year(&q.year)? {
        Some(y) => y.parse().unwrap_or_else(|_| current_year()),
        None => current_year(),
    };
    Ok(Json(outstanding_for(&st, &id, selected_year).await?))
}

/// Rent owed by a property's current tenant for `selected_year`. Shared by the
/// `outstanding` endpoint and the automated messaging job.
pub(crate) async fn outstanding_for(
    st: &AppState,
    property_id: &str,
    selected_year: i32,
) -> AppResult<OutstandingBalance> {
    let tenant_id = sqlx::query_scalar::<_, String>(
        "SELECT id FROM tenants WHERE property_id = ? AND is_current = 1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(property_id)
    .fetch_optional(&st.pool)
    .await?;
    let tenant_id = match tenant_id {
        Some(t) => t,
        None => return Ok(compute_outstanding(&[], &HashMap::new(), selected_year)),
    };

    let lease_rows = sqlx::query_as::<_, LeaseRow>(
        "SELECT monthly_rent, start_date, end_date, rent_due_day FROM leases WHERE tenant_id = ?",
    )
    .bind(&tenant_id)
    .fetch_all(&st.pool)
    .await?;
    let spans = lease_spans_from(lease_rows);
    let paid_by_year = rent_paid_by_year(st, property_id).await?;
    Ok(compute_outstanding(&spans, &paid_by_year, selected_year))
}

/// Calendar years with recorded transaction activity, newest first. Drives the
/// year filters so only years backed by real data are offered. The current year
/// is always included so a fresh dataset still has a sensible default.
pub async fn years(State(st): State<AppState>) -> AppResult<Json<Vec<i32>>> {
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT substr(date, 1, 4) AS y FROM transactions WHERE length(date) >= 4",
    )
    .fetch_all(&st.pool)
    .await?;

    let mut years: Vec<i32> = rows.into_iter().filter_map(|y| y.parse().ok()).collect();
    if !years.contains(&current_year()) {
        years.push(current_year());
    }
    years.sort_unstable_by(|a, b| b.cmp(a));
    Ok(Json(years))
}

/// Portfolio rollup in one round-trip: every property with its year summary and
/// rent owed, plus per-kind totals for the header cards.
pub async fn overview(
    State(st): State<AppState>,
    Query(q): Query<SummaryQuery>,
) -> AppResult<Json<OverviewResponse>> {
    let year = validate_year(&q.year)?;
    let selected_year: i32 = year
        .as_deref()
        .and_then(|y| y.parse().ok())
        .unwrap_or_else(current_year);

    let props =
        sqlx::query_as::<_, Property>(&format!("SELECT {COLUMNS} FROM properties ORDER BY name"))
            .fetch_all(&st.pool)
            .await?;

    let sums = sqlx::query_as::<_, (String, f64, f64)>(
        "SELECT property_id, \
            CAST(COALESCE(SUM(CASE WHEN kind = 'income'  THEN amount ELSE 0 END), 0) AS REAL), \
            CAST(COALESCE(SUM(CASE WHEN kind = 'expense' THEN amount ELSE 0 END), 0) AS REAL) \
         FROM transactions \
         WHERE (?1 IS NULL OR substr(date, 1, 4) = ?1) \
         GROUP BY property_id",
    )
    .bind(&year)
    .fetch_all(&st.pool)
    .await?;
    let sum_map: HashMap<String, (f64, f64)> = sums
        .into_iter()
        .map(|(id, inc, exp)| (id, (inc, exp)))
        .collect();

    // Leases of the current tenant, grouped by property.
    let lease_rows =
        sqlx::query_as::<_, (String, f64, Option<String>, Option<String>, Option<i64>)>(
            "SELECT t.property_id, l.monthly_rent, l.start_date, l.end_date, l.rent_due_day \
         FROM leases l JOIN tenants t ON t.id = l.tenant_id WHERE t.is_current = 1",
        )
        .fetch_all(&st.pool)
        .await?;
    let mut spans_by_prop: HashMap<String, Vec<LeaseSpan>> = HashMap::new();
    for (pid, rent, sd, ed, due) in lease_rows {
        if let Some(span) = lease_span(rent, sd.as_deref(), ed.as_deref(), due) {
            spans_by_prop.entry(pid).or_default().push(span);
        }
    }

    let paid_rows = sqlx::query_as::<_, (String, String, f64)>(&format!(
        "SELECT t.property_id, substr(t.date, 1, 4) AS y, CAST(SUM(t.amount) AS REAL) \
         FROM transactions t \
         JOIN categories c ON c.id = t.category_id \
         WHERE {RENT_PAID_PREDICATE} \
         GROUP BY t.property_id, y"
    ))
    .fetch_all(&st.pool)
    .await?;
    let mut paid_map: HashMap<String, HashMap<i32, f64>> = HashMap::new();
    for (pid, y, amt) in paid_rows {
        if let Ok(yr) = y.parse::<i32>() {
            paid_map.entry(pid).or_default().insert(yr, amt);
        }
    }

    // Most recently added current tenant per property.
    let tenant_rows = sqlx::query_as::<_, (String, String)>(
        "SELECT property_id, trim(first_name || ' ' || last_name) AS name \
         FROM tenants WHERE is_current = 1 ORDER BY created_at ASC",
    )
    .fetch_all(&st.pool)
    .await?;
    let mut tenant_map: HashMap<String, String> = HashMap::new();
    for (pid, name) in tenant_rows {
        let name = name.trim().to_string();
        if !name.is_empty() {
            tenant_map.insert(pid, name);
        }
    }

    let empty_paid = HashMap::new();
    let empty_spans: Vec<LeaseSpan> = Vec::new();
    let rows: Vec<OverviewRow> = props
        .into_iter()
        .map(|p| {
            let (total_income, total_expense) = sum_map.get(&p.id).copied().unwrap_or((0.0, 0.0));
            let spans = spans_by_prop.get(&p.id).unwrap_or(&empty_spans);
            let pm = paid_map.get(&p.id).unwrap_or(&empty_paid);
            let balance = compute_outstanding(spans, pm, selected_year);
            let tenant_name = tenant_map.get(&p.id).cloned();
            OverviewRow {
                total_income,
                total_expense,
                net: total_income - total_expense,
                outstanding: balance.outstanding,
                monthly_rent: balance.monthly_rent,
                tenant_name,
                property: p,
            }
        })
        .collect();

    let totals = portfolio_totals(&rows);
    Ok(Json(OverviewResponse { rows, totals }))
}

/// Year-end tax report: per-rental income/expense broken down by category, plus
/// portfolio totals. Mirrors the app convention of treating utilities as rental
/// income (so utilities expense on rentals is excluded).
pub async fn tax_report(
    State(st): State<AppState>,
    Query(q): Query<SummaryQuery>,
) -> AppResult<Json<TaxReport>> {
    let year = validate_year(&q.year)?;

    let props = sqlx::query_as::<_, Property>(&format!(
        "SELECT {COLUMNS} FROM properties WHERE kind = 'rental' ORDER BY name"
    ))
    .fetch_all(&st.pool)
    .await?;

    let rows = sqlx::query_as::<_, (String, String, String, f64)>(
        "SELECT t.property_id, t.kind, c.label AS category, CAST(SUM(t.amount) AS REAL) \
         FROM transactions t \
         JOIN properties p ON p.id = t.property_id \
         JOIN categories c ON c.id = t.category_id \
         WHERE p.kind = 'rental' AND (?1 IS NULL OR substr(t.date, 1, 4) = ?1) \
         GROUP BY t.property_id, t.kind, c.id",
    )
    .bind(&year)
    .fetch_all(&st.pool)
    .await?;

    // Per-property, then portfolio-wide, category buckets keyed by income/expense.
    let mut income_by_prop: HashMap<String, HashMap<String, f64>> = HashMap::new();
    let mut expense_by_prop: HashMap<String, HashMap<String, f64>> = HashMap::new();
    let mut income_totals: HashMap<String, f64> = HashMap::new();
    let mut expense_totals: HashMap<String, f64> = HashMap::new();
    for (pid, kind, category, total) in rows {
        let (per_prop, portfolio) = if kind == "income" {
            (&mut income_by_prop, &mut income_totals)
        } else {
            (&mut expense_by_prop, &mut expense_totals)
        };
        *per_prop
            .entry(pid)
            .or_default()
            .entry(category.clone())
            .or_insert(0.0) += total;
        *portfolio.entry(category).or_insert(0.0) += total;
    }

    let properties: Vec<TaxPropertyReport> = props
        .into_iter()
        .map(|p| {
            let income = sorted_totals(income_by_prop.remove(&p.id).unwrap_or_default());
            let expense = sorted_totals(expense_by_prop.remove(&p.id).unwrap_or_default());
            let total_income: f64 = income.iter().map(|c| c.total).sum();
            let total_expense: f64 = expense.iter().map(|c| c.total).sum();
            TaxPropertyReport {
                property: p,
                income,
                expense,
                total_income,
                total_expense,
                net: total_income - total_expense,
            }
        })
        .collect();

    let income = sorted_totals(income_totals);
    let expense = sorted_totals(expense_totals);
    let total_income: f64 = income.iter().map(|c| c.total).sum();
    let total_expense: f64 = expense.iter().map(|c| c.total).sum();

    Ok(Json(TaxReport {
        year: year.unwrap_or_else(|| "all".to_string()),
        properties,
        income,
        expense,
        total_income,
        total_expense,
        net: total_income - total_expense,
    }))
}

/// Category buckets as a list sorted by amount, largest first.
fn sorted_totals(map: HashMap<String, f64>) -> Vec<TaxCategoryTotal> {
    let mut out: Vec<TaxCategoryTotal> = map
        .into_iter()
        .map(|(category, total)| TaxCategoryTotal { category, total })
        .collect();
    out.sort_by(|a, b| {
        b.total
            .partial_cmp(&a.total)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

/// Sums each property kind into the header-card figures, mirroring what the UI
/// shows: credits never lower another property's owed rent (negatives clamped).
fn portfolio_totals(rows: &[OverviewRow]) -> Vec<PortfolioTotals> {
    let mut by_kind: HashMap<String, PortfolioTotals> = HashMap::new();
    for r in rows {
        let t = by_kind
            .entry(r.property.kind.clone())
            .or_insert_with(|| PortfolioTotals {
                kind: r.property.kind.clone(),
                income: 0.0,
                expense: 0.0,
                net: 0.0,
                outstanding: 0.0,
                gain_pct: None,
            });
        t.income += r.total_income;
        t.expense += r.total_expense;
        t.outstanding += r.outstanding.max(0.0);
    }
    let mut totals: Vec<PortfolioTotals> = by_kind.into_values().collect();
    for t in &mut totals {
        t.net = t.income - t.expense;
        t.gain_pct = if t.expense > 0.0 {
            Some(t.net / t.expense * 100.0)
        } else {
            None
        };
    }
    totals
}

async fn rent_paid_by_year(st: &AppState, property_id: &str) -> AppResult<HashMap<i32, f64>> {
    let rows = sqlx::query_as::<_, (String, f64)>(&format!(
        "SELECT substr(t.date, 1, 4) AS y, CAST(SUM(t.amount) AS REAL) \
         FROM transactions t \
         JOIN categories c ON c.id = t.category_id \
         WHERE t.property_id = ? AND ({RENT_PAID_PREDICATE}) \
         GROUP BY y"
    ))
    .bind(property_id)
    .fetch_all(&st.pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(y, amt)| y.parse::<i32>().ok().map(|y| (y, amt)))
        .collect())
}

/// A lease row as fetched for outstanding-rent math: rent, start/end dates, due day.
type LeaseRow = (f64, Option<String>, Option<String>, Option<i64>);

/// A lease reduced to absolute month indices (year*12 + month-1) and its rent.
struct LeaseSpan {
    start: i32,
    end: Option<i32>,
    rent: f64,
    rent_due_day: Option<i64>,
}

fn lease_span(
    rent: f64,
    start_date: Option<&str>,
    end_date: Option<&str>,
    rent_due_day: Option<i64>,
) -> Option<LeaseSpan> {
    // A lease only accrues once it has a start date.
    let start = start_date.and_then(month_index)?;
    Some(LeaseSpan {
        start,
        end: end_date.and_then(month_index),
        rent,
        rent_due_day,
    })
}

fn lease_spans_from(rows: Vec<LeaseRow>) -> Vec<LeaseSpan> {
    rows.into_iter()
        .filter_map(|(rent, sd, ed, due)| lease_span(rent, sd.as_deref(), ed.as_deref(), due))
        .collect()
}

/// Expected rent is prorated by the months each lease actually covers, and never
/// counts months that have not yet come due.
fn compute_outstanding(
    spans: &[LeaseSpan],
    paid_by_year: &HashMap<i32, f64>,
    selected_year: i32,
) -> OutstandingBalance {
    let year = selected_year.to_string();

    let start = match spans.iter().map(|s| s.start).min() {
        Some(m) => m,
        None => {
            return OutstandingBalance {
                monthly_rent: 0.0,
                expected: 0.0,
                paid: 0.0,
                carry_over: 0.0,
                outstanding: 0.0,
                year,
            }
        }
    };

    // Rent of the lease covering a given month (most recently started one wins).
    let rent_for_month = |m: i32| -> f64 {
        spans
            .iter()
            .filter(|s| s.start <= m && s.end.is_none_or(|e| e >= m))
            .max_by_key(|s| s.start)
            .map(|s| s.rent)
            .unwrap_or(0.0)
    };

    // Only accrue through months that have already come due. The current month
    // isn't owed until its rent-due day passes, so it's excluded before then.
    let current_m = current_month_index();
    let due_day_now = spans
        .iter()
        .filter(|s| s.start <= current_m && s.end.is_none_or(|e| e >= current_m))
        .max_by_key(|s| s.start)
        .and_then(|s| s.rent_due_day);
    let cap = match due_day_now {
        Some(day) if current_day_of_month() < day => current_m - 1,
        _ => current_m,
    };
    let year_expected = |y: i32| -> f64 {
        let lo = std::cmp::max(y * 12, start);
        let hi = std::cmp::min(y * 12 + 11, cap);
        if hi < lo {
            return 0.0;
        }
        (lo..=hi).map(rent_for_month).sum()
    };

    let start_year = start / 12;
    let mut carry_over = 0.0;
    for y in start_year..selected_year {
        carry_over += year_expected(y) - paid_by_year.get(&y).copied().unwrap_or(0.0);
    }

    let expected = year_expected(selected_year);
    let paid = paid_by_year.get(&selected_year).copied().unwrap_or(0.0);
    let outstanding = carry_over + (expected - paid);

    // Show the rent active at the latest accrued month, else the most recent lease.
    let display_month = std::cmp::min(selected_year * 12 + 11, cap);
    let active_rent = rent_for_month(display_month);
    let monthly_rent = if active_rent > 0.0 {
        active_rent
    } else {
        spans
            .iter()
            .max_by_key(|s| s.start)
            .map(|s| s.rent)
            .unwrap_or(0.0)
    };

    OutstandingBalance {
        monthly_rent,
        expected,
        paid,
        carry_over,
        outstanding,
        year,
    }
}
