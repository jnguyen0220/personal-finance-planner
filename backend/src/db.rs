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
    reminders_enabled INTEGER NOT NULL DEFAULT 1,
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
    first_name   TEXT NOT NULL DEFAULT '',
    last_name    TEXT NOT NULL DEFAULT '',
    email        TEXT NOT NULL DEFAULT '',
    phone        TEXT NOT NULL DEFAULT '',
    is_current   INTEGER NOT NULL DEFAULT 0,
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
    rent_due_day INTEGER,
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
    category_id TEXT NOT NULL REFERENCES categories(id),
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
    id               TEXT PRIMARY KEY,
    label            TEXT NOT NULL,
    parent_id        TEXT REFERENCES categories(id) ON DELETE CASCADE,
    kind             TEXT NOT NULL,
    fields           TEXT NOT NULL DEFAULT '',
    deductible       INTEGER NOT NULL DEFAULT 0,
    applies_rental   INTEGER NOT NULL DEFAULT 1,
    applies_personal INTEGER NOT NULL DEFAULT 1,
    selectable       INTEGER NOT NULL DEFAULT 1,
    counts_as_rent   INTEGER NOT NULL DEFAULT 0,
    position         INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS option_lists (
    list     TEXT NOT NULL,
    value    TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (list, value)
);

-- Idempotency ledger so each lease/insurance expiry texts the contact list once.
CREATE TABLE IF NOT EXISTS contact_reminders (
    dedup_key  TEXT PRIMARY KEY,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_transactions_property ON transactions(property_id);
CREATE INDEX IF NOT EXISTS idx_transactions_category ON transactions(category_id);
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

/// Populates the `states`, `categories`, and `option_lists` reference tables
/// from the canonical Rust definitions. States are kept complete on every
/// startup, while the editable tables are only seeded when empty so operator
/// edits (including deletions) are preserved.
async fn seed_reference_data(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for state in crate::states::STATES {
        sqlx::query("INSERT OR IGNORE INTO states (code, name) VALUES (?, ?)")
            .bind(state.code)
            .bind(state.name)
            .execute(pool)
            .await?;
    }

    let category_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM categories")
        .fetch_one(pool)
        .await?;
    if category_count == 0 {
        for (position, c) in crate::categories::CATEGORIES.iter().enumerate() {
            sqlx::query(
                "INSERT INTO categories (id, label, parent_id, kind, fields, deductible, \
                                         applies_rental, applies_personal, selectable, \
                                         counts_as_rent, position) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(c.id)
            .bind(c.label)
            .bind(c.parent_id)
            .bind(c.kind)
            .bind(c.fields.join(","))
            .bind(c.deductible as i64)
            .bind(c.applies_rental as i64)
            .bind(c.applies_personal as i64)
            .bind(c.selectable as i64)
            .bind(c.counts_as_rent as i64)
            .bind(position as i64)
            .execute(pool)
            .await?;
        }
    }

    for (list, defaults) in crate::options::LISTS {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM option_lists WHERE list = ?")
                .bind(list)
                .fetch_one(pool)
                .await?;
        if count == 0 {
            for (position, value) in defaults.iter().enumerate() {
                sqlx::query("INSERT INTO option_lists (list, value, position) VALUES (?, ?, ?)")
                    .bind(list)
                    .bind(value)
                    .bind(position as i64)
                    .execute(pool)
                    .await?;
            }
        }
    }

    // Default sign-off; kept only when the operator hasn't set their own.
    sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES (?, ?)")
        .bind(crate::settings::SIGNATURE)
        .bind(crate::settings::SIGNATURE_DEFAULT)
        .execute(pool)
        .await?;
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
    // Reminder opt-out moved from the (impermanent) tenant to the property.
    if !column_exists(pool, "properties", "reminders_enabled").await? {
        sqlx::query(
            "ALTER TABLE properties ADD COLUMN reminders_enabled INTEGER NOT NULL DEFAULT 1",
        )
        .execute(pool)
        .await?;
        // Preserve any opt-out previously set on a property's current tenant.
        if column_exists(pool, "tenants", "notifications_enabled").await? {
            sqlx::query(
                "UPDATE properties SET reminders_enabled = 0 \
                 WHERE EXISTS (SELECT 1 FROM tenants t \
                               WHERE t.property_id = properties.id AND t.is_current = 1 AND t.notifications_enabled = 0)",
            )
            .execute(pool)
            .await?;
        }
    }
    if column_exists(pool, "tenants", "notifications_enabled").await? {
        sqlx::query("ALTER TABLE tenants DROP COLUMN notifications_enabled")
            .execute(pool)
            .await?;
    }
    add_column_if_missing(
        pool,
        "tenants",
        "first_name",
        "ALTER TABLE tenants ADD COLUMN first_name TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    add_column_if_missing(
        pool,
        "tenants",
        "last_name",
        "ALTER TABLE tenants ADD COLUMN last_name TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    // Split the legacy single `name` column into first/last, then drop it.
    if column_exists(pool, "tenants", "name").await? {
        sqlx::query(
            "UPDATE tenants SET \
                first_name = CASE WHEN instr(name, ' ') > 0 THEN substr(name, 1, instr(name, ' ') - 1) ELSE name END, \
                last_name  = CASE WHEN instr(name, ' ') > 0 THEN substr(name, instr(name, ' ') + 1) ELSE '' END \
             WHERE first_name = '' AND last_name = ''",
        )
        .execute(pool)
        .await?;
        sqlx::query("ALTER TABLE tenants DROP COLUMN name")
            .execute(pool)
            .await?;
    }
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
        "rent_due_day",
        "ALTER TABLE leases ADD COLUMN rent_due_day INTEGER",
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
            "UPDATE transactions SET tenant_name = COALESCE((SELECT trim(first_name || ' ' || last_name) FROM tenants WHERE tenants.id = transactions.tenant_id), '') \
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
