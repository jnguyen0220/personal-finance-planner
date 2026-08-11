use serde::Serialize;

/// Canonical category definitions — the single source of truth for which
/// categories exist, their income/expense kind, form fields, and where they apply.
#[derive(Serialize)]
pub struct CategoryDef {
    pub name: &'static str,
    pub kind: &'static str,
    pub fields: &'static [&'static str],
    pub rentals: bool,
    pub personal: bool,
    /// Whether a tenant who pays this expense can deduct it from rent owed.
    pub deductible: bool,
}

pub const CATEGORIES: &[CategoryDef] = &[
    CategoryDef {
        name: "rent",
        kind: "income",
        fields: &["date", "amount", "tenant"],
        rentals: true,
        personal: false,
        deductible: false,
    },
    CategoryDef {
        name: "late fee",
        kind: "income",
        fields: &["date", "amount", "tenant"],
        rentals: true,
        personal: false,
        deductible: false,
    },
    CategoryDef {
        name: "repair",
        kind: "expense",
        fields: &["date", "amount", "description", "receipt"],
        rentals: true,
        personal: true,
        deductible: true,
    },
    CategoryDef {
        name: "utilities",
        kind: "expense",
        fields: &["date", "amount", "description", "receipt"],
        rentals: false,
        personal: true,
        deductible: false,
    },
    CategoryDef {
        name: "tax",
        kind: "expense",
        fields: &["date", "amount", "description", "receipt"],
        rentals: true,
        personal: true,
        deductible: false,
    },
    CategoryDef {
        name: "mortgage",
        kind: "expense",
        fields: &["date", "amount", "description", "receipt"],
        rentals: true,
        personal: true,
        deductible: false,
    },
    CategoryDef {
        name: "other",
        kind: "expense",
        fields: &["date", "amount", "description", "receipt"],
        rentals: true,
        personal: true,
        deductible: true,
    },
];

/// A category resolved for a specific property kind: income/expense, the form
/// fields to render, and whether it can be recorded for that property kind.
#[derive(Serialize)]
pub struct ResolvedCategory {
    pub name: &'static str,
    pub kind: &'static str,
    pub fields: Vec<&'static str>,
    pub applies: bool,
    /// Whether a tenant-paid transaction in this category deducts from rent owed.
    pub deductible: bool,
}

/// Every category resolved for `property_kind`. Non-applicable categories are
/// still returned (with `applies = false`) so clients can render legacy rows.
pub fn resolved_for(property_kind: &str) -> Vec<ResolvedCategory> {
    CATEGORIES.iter().map(|c| resolve(c, property_kind)).collect()
}

fn resolve(c: &CategoryDef, property_kind: &str) -> ResolvedCategory {
    let applies = if property_kind == "personal" {
        c.personal
    } else {
        c.rentals
    };
    // Deducting from rent only makes sense on a rental that collects rent.
    let deductible = c.deductible && property_kind == "rental";
    // Rentals bill utilities to the tenant, so it is tenant-paid income.
    if c.name == "utilities" && property_kind == "rental" {
        return ResolvedCategory {
            name: c.name,
            kind: "income",
            fields: vec!["date", "amount", "tenant"],
            applies,
            deductible: false,
        };
    }
    ResolvedCategory {
        name: c.name,
        kind: c.kind,
        fields: c.fields.to_vec(),
        applies,
        deductible,
    }
}

/// Whether a tenant who pays this expense may deduct it from rent owed. This is
/// the authoritative rule; handlers and the API both defer to it.
pub fn is_deductible(category: &str, property_kind: &str) -> bool {
    resolved_for(property_kind)
        .iter()
        .any(|c| c.name == category && c.deductible)
}

/// The authoritative income/expense kind for a known category on a given
/// property type, derived from `CATEGORIES` (so the utilities-on-rental override
/// stays in sync). Unknown (legacy) categories return None so their stored kind
/// is preserved.
pub fn canonical_kind(category: &str, property_kind: &str) -> Option<&'static str> {
    CATEGORIES
        .iter()
        .find(|c| c.name == category)
        .map(|c| resolve(c, property_kind).kind)
}
