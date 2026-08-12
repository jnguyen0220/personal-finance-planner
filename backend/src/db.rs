use std::time::Duration;

use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS properties (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    address       TEXT NOT NULL DEFAULT '',
    city          TEXT NOT NULL DEFAULT '',
    state         TEXT NOT NULL DEFAULT '',
    zip           TEXT NOT NULL DEFAULT '',
    kind          TEXT NOT NULL DEFAULT 'rental',
    purchase_date TEXT,
    notes         TEXT NOT NULL DEFAULT '',
    hoa_name      TEXT NOT NULL DEFAULT '',
    hoa_phone     TEXT NOT NULL DEFAULT '',
    hoa_email     TEXT NOT NULL DEFAULT '',
    hoa_webpage   TEXT NOT NULL DEFAULT '',
    created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tenants (
    id           TEXT PRIMARY KEY,
    property_id  TEXT NOT NULL REFERENCES properties(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    email        TEXT NOT NULL DEFAULT '',
    phone        TEXT NOT NULL DEFAULT '',
    is_current   INTEGER NOT NULL DEFAULT 0,
    notifications_enabled INTEGER NOT NULL DEFAULT 1,
    notes        TEXT NOT NULL DEFAULT '',
    driver_license_id TEXT REFERENCES attachments(id) ON DELETE SET NULL,
    created_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS leases (
    id           TEXT PRIMARY KEY,
    tenant_id    TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    monthly_rent REAL NOT NULL DEFAULT 0,
    start_date   TEXT,
    end_date     TEXT,
    payment_date TEXT,
    late_fee     REAL NOT NULL DEFAULT 0,
    notify_days  INTEGER NOT NULL DEFAULT 30,
    notes        TEXT NOT NULL DEFAULT '',
    created_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS attachments (
    id            TEXT PRIMARY KEY,
    stored_name   TEXT NOT NULL,
    original_name TEXT NOT NULL,
    content_type  TEXT NOT NULL,
    size          INTEGER NOT NULL,
    uploaded_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS transactions (
    id          TEXT PRIMARY KEY,
    property_id TEXT NOT NULL REFERENCES properties(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL,
    category    TEXT NOT NULL DEFAULT 'other',
    amount      REAL NOT NULL,
    date        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    tenant_name TEXT NOT NULL DEFAULT '',
    borne_by    TEXT NOT NULL DEFAULT 'landlord',
    receipt_id  TEXT REFERENCES attachments(id) ON DELETE SET NULL,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS insurance_policies (
    id            TEXT PRIMARY KEY,
    property_id   TEXT NOT NULL REFERENCES properties(id) ON DELETE CASCADE,
    provider      TEXT NOT NULL,
    policy_number TEXT NOT NULL DEFAULT '',
    premium       REAL NOT NULL DEFAULT 0,
    start_date    TEXT,
    expiry_date   TEXT NOT NULL,
    notify_days   INTEGER NOT NULL DEFAULT 30,
    notes         TEXT NOT NULL DEFAULT '',
    created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS notifications (
    id           TEXT PRIMARY KEY,
    kind         TEXT NOT NULL,
    severity     TEXT NOT NULL DEFAULT 'warning',
    title        TEXT NOT NULL,
    body         TEXT NOT NULL DEFAULT '',
    link         TEXT,
    property_id  TEXT REFERENCES properties(id) ON DELETE CASCADE,
    dedup_key    TEXT UNIQUE,
    auto_resolve INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL,
    dismissed_at TEXT
);

CREATE TABLE IF NOT EXISTS messages (
    id          TEXT PRIMARY KEY,
    tenant_id   TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    property_id TEXT NOT NULL REFERENCES properties(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL DEFAULT 'custom',
    to_phone    TEXT NOT NULL DEFAULT '',
    body        TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'queued',
    error       TEXT,
    dedup_key   TEXT,
    created_at  TEXT NOT NULL,
    sent_at     TEXT
);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS providers (
    id          TEXT PRIMARY KEY,
    property_id TEXT NOT NULL REFERENCES properties(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL DEFAULT 'other',
    name        TEXT NOT NULL,
    phone       TEXT NOT NULL DEFAULT '',
    homepage    TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS states (
    code TEXT PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS categories (
    name       TEXT PRIMARY KEY,
    kind       TEXT NOT NULL,
    fields     TEXT NOT NULL,
    rentals    INTEGER NOT NULL,
    personal   INTEGER NOT NULL,
    deductible INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_transactions_property ON transactions(property_id);
CREATE INDEX IF NOT EXISTS idx_tenants_property ON tenants(property_id);
CREATE INDEX IF NOT EXISTS idx_leases_tenant ON leases(tenant_id);
CREATE INDEX IF NOT EXISTS idx_insurance_property ON insurance_policies(property_id);
CREATE INDEX IF NOT EXISTS idx_notifications_active ON notifications(dismissed_at);
CREATE INDEX IF NOT EXISTS idx_messages_property ON messages(property_id);
CREATE INDEX IF NOT EXISTS idx_messages_tenant ON messages(tenant_id);
CREATE INDEX IF NOT EXISTS idx_providers_property ON providers(property_id);
"#;

pub async fn init_pool(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::raw_sql(SCHEMA).execute(&pool).await?;
    migrate(&pool).await?;
    seed_reference_data(&pool).await?;
    Ok(pool)
}

/// Populates the `states` and `categories` reference tables from the canonical
/// Rust definitions. Runs on every startup but only inserts rows that are
/// missing, so a fresh database is fully populated and existing data is kept.
async fn seed_reference_data(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for state in crate::states::STATES {
        sqlx::query("INSERT OR IGNORE INTO states (code, name) VALUES (?, ?)")
            .bind(state.code)
            .bind(state.name)
            .execute(pool)
            .await?;
    }
    for category in crate::categories::CATEGORIES {
        sqlx::query(
            "INSERT OR IGNORE INTO categories (name, kind, fields, rentals, personal, deductible) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(category.name)
        .bind(category.kind)
        .bind(category.fields.join(","))
        .bind(category.rentals as i64)
        .bind(category.personal as i64)
        .bind(category.deductible as i64)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Applies additive migrations for databases created before a column existed.
async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    add_column_if_missing(
        pool,
        "tenants",
        "is_current",
        "ALTER TABLE tenants ADD COLUMN is_current INTEGER NOT NULL DEFAULT 0",
    )
    .await?;
    add_column_if_missing(
        pool,
        "tenants",
        "driver_license_id",
        "ALTER TABLE tenants ADD COLUMN driver_license_id TEXT REFERENCES attachments(id) ON DELETE SET NULL",
    )
    .await?;
    add_column_if_missing(
        pool,
        "tenants",
        "notifications_enabled",
        "ALTER TABLE tenants ADD COLUMN notifications_enabled INTEGER NOT NULL DEFAULT 1",
    )
    .await?;
    add_column_if_missing(
        pool,
        "properties",
        "city",
        "ALTER TABLE properties ADD COLUMN city TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    add_column_if_missing(
        pool,
        "properties",
        "state",
        "ALTER TABLE properties ADD COLUMN state TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    add_column_if_missing(
        pool,
        "properties",
        "zip",
        "ALTER TABLE properties ADD COLUMN zip TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    for col in ["hoa_name", "hoa_phone", "hoa_email", "hoa_webpage"] {
        add_column_if_missing(
            pool,
            "properties",
            col,
            &format!("ALTER TABLE properties ADD COLUMN {col} TEXT NOT NULL DEFAULT ''"),
        )
        .await?;
    }
    add_column_if_missing(
        pool,
        "transactions",
        "tenant_name",
        "ALTER TABLE transactions ADD COLUMN tenant_name TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    add_column_if_missing(
        pool,
        "transactions",
        "borne_by",
        "ALTER TABLE transactions ADD COLUMN borne_by TEXT NOT NULL DEFAULT 'landlord'",
    )
    .await?;
    add_column_if_missing(
        pool,
        "leases",
        "payment_date",
        "ALTER TABLE leases ADD COLUMN payment_date TEXT",
    )
    .await?;
    add_column_if_missing(
        pool,
        "leases",
        "late_fee",
        "ALTER TABLE leases ADD COLUMN late_fee REAL NOT NULL DEFAULT 0",
    )
    .await?;
    add_column_if_missing(
        pool,
        "leases",
        "notify_days",
        "ALTER TABLE leases ADD COLUMN notify_days INTEGER NOT NULL DEFAULT 30",
    )
    .await?;
    add_column_if_missing(
        pool,
        "insurance_policies",
        "notify_days",
        "ALTER TABLE insurance_policies ADD COLUMN notify_days INTEGER NOT NULL DEFAULT 30",
    )
    .await?;
    // Idempotency key for automated messages; unique so a condition sends once.
    add_column_if_missing(
        pool,
        "messages",
        "dedup_key",
        "ALTER TABLE messages ADD COLUMN dedup_key TEXT",
    )
    .await?;
    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_messages_dedup ON messages(dedup_key)")
        .execute(pool)
        .await?;

    // Backfill the tenant name from the legacy tenant_id link on older databases.
    if column_exists(pool, "transactions", "tenant_id").await? {
        sqlx::query(
            "UPDATE transactions SET tenant_name = COALESCE((SELECT name FROM tenants WHERE tenants.id = transactions.tenant_id), '') \
             WHERE tenant_name = '' AND tenant_id IS NOT NULL",
        )
        .execute(pool)
        .await?;
    }

    // Move legacy per-tenant lease data into the leases table (older databases).
    if column_exists(pool, "tenants", "monthly_rent").await? {
        sqlx::query(
            "INSERT INTO leases (id, tenant_id, monthly_rent, start_date, end_date, notes, created_at) \
             SELECT lower(hex(randomblob(16))), t.id, t.monthly_rent, t.lease_start, t.lease_end, '', t.created_at \
             FROM tenants t \
             WHERE (t.monthly_rent > 0 OR t.lease_start IS NOT NULL) \
               AND NOT EXISTS (SELECT 1 FROM leases l WHERE l.tenant_id = t.id)",
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn column_exists(pool: &SqlitePool, table: &str, column: &str) -> Result<bool, sqlx::Error> {
    let row = sqlx::query(&format!(
        "SELECT 1 FROM pragma_table_info('{table}') WHERE name = ?"
    ))
    .bind(column)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

async fn add_column_if_missing(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    ddl: &str,
) -> Result<(), sqlx::Error> {
    if !column_exists(pool, table, column).await? {
        sqlx::query(ddl).execute(pool).await?;
    }
    Ok(())
}
