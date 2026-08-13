//! The transaction category taxonomy: a single self-referential tree. A
//! category may be a selectable leaf (e.g. `rent`, `service.repair`) or a
//! grouping parent (e.g. `service`) that only exists to organize its children.
//! Every transaction points at exactly one leaf category, which is the single
//! source of truth for its income/expense kind and rent-deductibility.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Seed definition for one category; used only to populate a fresh database.
pub struct CategoryDef {
    pub id: &'static str,
    pub label: &'static str,
    pub parent_id: Option<&'static str>,
    pub kind: &'static str,
    pub fields: &'static [&'static str],
    pub deductible: bool,
    pub applies_rental: bool,
    pub applies_personal: bool,
    /// False for grouping-only parents that can't be recorded against directly.
    pub selectable: bool,
    /// Income that counts toward rent owed (used by the "rent paid" rollups).
    pub counts_as_rent: bool,
}

/// The default taxonomy. Order here becomes each category's `position`. Two
/// top-level groups mirror how a landlord thinks: money the tenant pays in
/// (Tenant) and what the property costs (Property). Whether a property expense
/// was actually paid by the tenant — and so deducts from rent — is a per-
/// transaction fact (the "paid by tenant" toggle), not a separate category.
pub const CATEGORIES: &[CategoryDef] = &[
    CategoryDef {
        id: "tenant",
        label: "Tenant",
        parent_id: None,
        kind: "income",
        fields: &[],
        deductible: false,
        applies_rental: true,
        applies_personal: false,
        selectable: false,
        counts_as_rent: false,
    },
    CategoryDef {
        id: "rent",
        label: "Rent",
        parent_id: Some("tenant"),
        kind: "income",
        fields: &["date", "amount", "tenant"],
        deductible: false,
        applies_rental: true,
        applies_personal: false,
        selectable: true,
        counts_as_rent: true,
    },
    CategoryDef {
        id: "late_fee",
        label: "Late fee",
        parent_id: Some("tenant"),
        kind: "income",
        fields: &["date", "amount", "tenant"],
        deductible: false,
        applies_rental: true,
        applies_personal: false,
        selectable: true,
        counts_as_rent: false,
    },
    CategoryDef {
        id: "pet_fee",
        label: "Pet fee",
        parent_id: Some("tenant"),
        kind: "income",
        fields: &["date", "amount", "tenant"],
        deductible: false,
        applies_rental: true,
        applies_personal: false,
        selectable: true,
        counts_as_rent: false,
    },
    CategoryDef {
        id: "property",
        label: "Property",
        parent_id: None,
        kind: "expense",
        fields: &[],
        deductible: false,
        applies_rental: true,
        applies_personal: true,
        selectable: false,
        counts_as_rent: false,
    },
    CategoryDef {
        id: "mortgage",
        label: "Mortgage",
        parent_id: Some("property"),
        kind: "expense",
        fields: &["date", "amount", "description", "receipt"],
        deductible: false,
        applies_rental: true,
        applies_personal: true,
        selectable: true,
        counts_as_rent: false,
    },
    CategoryDef {
        id: "tax",
        label: "Tax",
        parent_id: Some("property"),
        kind: "expense",
        fields: &["date", "amount", "description", "receipt"],
        deductible: false,
        applies_rental: true,
        applies_personal: true,
        selectable: true,
        counts_as_rent: false,
    },
    CategoryDef {
        id: "insurance",
        label: "Insurance",
        parent_id: Some("property"),
        kind: "expense",
        fields: &["date", "amount", "description", "receipt"],
        deductible: false,
        applies_rental: true,
        applies_personal: true,
        selectable: true,
        counts_as_rent: false,
    },
    CategoryDef {
        id: "utilities",
        label: "Utilities",
        parent_id: Some("property"),
        kind: "expense",
        fields: &["date", "amount", "description", "receipt"],
        deductible: true,
        applies_rental: true,
        applies_personal: true,
        selectable: true,
        counts_as_rent: false,
    },
    CategoryDef {
        id: "repair",
        label: "Repair",
        parent_id: Some("property"),
        kind: "expense",
        fields: &["date", "amount", "description", "receipt"],
        deductible: true,
        applies_rental: true,
        applies_personal: true,
        selectable: true,
        counts_as_rent: false,
    },
    CategoryDef {
        id: "other",
        label: "Other",
        parent_id: Some("property"),
        kind: "expense",
        fields: &["date", "amount", "description", "receipt"],
        deductible: true,
        applies_rental: true,
        applies_personal: true,
        selectable: true,
        counts_as_rent: false,
    },
];

/// Raw category row as stored in the `categories` table (fields as CSV).
#[derive(sqlx::FromRow)]
struct CategoryRecord {
    id: String,
    label: String,
    parent_id: Option<String>,
    kind: String,
    fields: String,
    deductible: i64,
    applies_rental: i64,
    applies_personal: i64,
    selectable: i64,
    counts_as_rent: i64,
    position: i64,
}

impl CategoryRecord {
    fn into_category(self) -> Category {
        Category {
            id: self.id,
            label: self.label,
            parent_id: self.parent_id,
            kind: self.kind,
            fields: split_fields(&self.fields),
            deductible: self.deductible != 0,
            applies_rental: self.applies_rental != 0,
            applies_personal: self.applies_personal != 0,
            selectable: self.selectable != 0,
            counts_as_rent: self.counts_as_rent != 0,
            position: self.position,
        }
    }
}

fn split_fields(csv: &str) -> Vec<String> {
    csv.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// A category as stored and edited on the Admin page.
#[derive(Serialize, Deserialize, Clone)]
pub struct Category {
    pub id: String,
    pub label: String,
    pub parent_id: Option<String>,
    pub kind: String,
    pub fields: Vec<String>,
    pub deductible: bool,
    pub applies_rental: bool,
    pub applies_personal: bool,
    pub selectable: bool,
    pub counts_as_rent: bool,
    #[serde(default)]
    pub position: i64,
}

impl Category {
    /// Whether this category can be recorded against a property of `property_kind`.
    pub fn applies_to(&self, property_kind: &str) -> bool {
        if property_kind == "personal" {
            self.applies_personal
        } else {
            self.applies_rental
        }
    }
}

/// A category with `applies` resolved for a specific property kind, sent to the
/// client so it can render the picker without re-deriving any rules.
#[derive(Serialize)]
pub struct ResolvedCategory {
    pub id: String,
    pub label: String,
    pub parent_id: Option<String>,
    pub kind: String,
    pub fields: Vec<String>,
    pub deductible: bool,
    pub counts_as_rent: bool,
    pub selectable: bool,
    pub applies: bool,
    pub position: i64,
}

/// Every category, ordered by position.
pub async fn all(pool: &SqlitePool) -> Result<Vec<Category>, sqlx::Error> {
    let rows = sqlx::query_as::<_, CategoryRecord>(
        "SELECT id, label, parent_id, kind, fields, deductible, applies_rental, \
                applies_personal, selectable, counts_as_rent, position \
         FROM categories ORDER BY position",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(CategoryRecord::into_category)
        .collect())
}

/// A single category by id.
pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<Category>, sqlx::Error> {
    let row = sqlx::query_as::<_, CategoryRecord>(
        "SELECT id, label, parent_id, kind, fields, deductible, applies_rental, \
                applies_personal, selectable, counts_as_rent, position \
         FROM categories WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(CategoryRecord::into_category))
}

/// Every category resolved for `property_kind`, ordered by position.
pub async fn resolved_for(
    pool: &SqlitePool,
    property_kind: &str,
) -> Result<Vec<ResolvedCategory>, sqlx::Error> {
    Ok(all(pool)
        .await?
        .into_iter()
        .map(|c| ResolvedCategory {
            applies: c.applies_to(property_kind),
            // Deducting from rent only makes sense on a rental that collects it.
            deductible: c.deductible && property_kind == "rental",
            id: c.id,
            label: c.label,
            parent_id: c.parent_id,
            kind: c.kind,
            fields: c.fields,
            counts_as_rent: c.counts_as_rent,
            selectable: c.selectable,
            position: c.position,
        })
        .collect())
}

pub async fn exists(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query_scalar::<_, i64>("SELECT 1 FROM categories WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .is_some(),
    )
}

/// Inserts a new category at the end of the ordering.
pub async fn insert(pool: &SqlitePool, c: &Category) -> Result<Category, sqlx::Error> {
    let position =
        sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(position) + 1, 0) FROM categories")
            .fetch_one(pool)
            .await?;
    sqlx::query(
        "INSERT INTO categories (id, label, parent_id, kind, fields, deductible, applies_rental, \
                                 applies_personal, selectable, counts_as_rent, position) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&c.id)
    .bind(&c.label)
    .bind(&c.parent_id)
    .bind(&c.kind)
    .bind(c.fields.join(","))
    .bind(c.deductible as i64)
    .bind(c.applies_rental as i64)
    .bind(c.applies_personal as i64)
    .bind(c.selectable as i64)
    .bind(c.counts_as_rent as i64)
    .bind(position)
    .execute(pool)
    .await?;
    get(pool, &c.id).await.map(|c| c.expect("just inserted"))
}

/// Updates the mutable attributes of an existing category (the id is the key).
pub async fn update(pool: &SqlitePool, id: &str, c: &Category) -> Result<u64, sqlx::Error> {
    Ok(sqlx::query(
        "UPDATE categories SET label = ?, parent_id = ?, kind = ?, fields = ?, deductible = ?, \
                               applies_rental = ?, applies_personal = ?, selectable = ?, \
                               counts_as_rent = ? \
         WHERE id = ?",
    )
    .bind(&c.label)
    .bind(&c.parent_id)
    .bind(&c.kind)
    .bind(c.fields.join(","))
    .bind(c.deductible as i64)
    .bind(c.applies_rental as i64)
    .bind(c.applies_personal as i64)
    .bind(c.selectable as i64)
    .bind(c.counts_as_rent as i64)
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected())
}

/// Removes a category. Fails at the database layer if transactions still
/// reference it (the FK is RESTRICT), preserving data integrity.
pub async fn remove(pool: &SqlitePool, id: &str) -> Result<u64, sqlx::Error> {
    Ok(sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected())
}
