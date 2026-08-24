//! Editable SMS templates for automated tenant reminders. Bodies live in the
//! `settings` table under `template:{kind}`; when unset the built-in default is
//! used. `{token}` placeholders are substituted with live values at send time.

use sqlx::SqlitePool;

use crate::settings;

pub struct Placeholder {
    pub token: &'static str,
    pub description: &'static str,
}

pub struct TemplateDef {
    pub kind: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// Audience the message is sent to: "tenant" or "property" (landlord).
    pub group: &'static str,
    pub placeholders: &'static [Placeholder],
    pub default_body: &'static str,
}

/// Every automated message type whose wording the operator can customise.
pub const TEMPLATES: &[TemplateDef] = &[
    TemplateDef {
        kind: "outstanding_balance",
        label: "Outstanding balance reminder",
        description: "Sent once a month to a current tenant who still owes rent.",
        group: "tenant",
        placeholders: &[
            Placeholder { token: "tenant_name", description: "Tenant's full name" },
            Placeholder { token: "address", description: "Property street address" },
            Placeholder { token: "city", description: "Property city" },
            Placeholder { token: "state", description: "Property state" },
            Placeholder { token: "zip", description: "Property ZIP code" },
            Placeholder { token: "balance", description: "Amount owed, e.g. $1,200.00" },
            Placeholder { token: "month", description: "Current month, e.g. August 2026" },
            Placeholder { token: "late_fee", description: "Late-fee sentence (blank when none)" },
            Placeholder { token: "signature", description: "Your sign-off (set on the Admin page)" },
        ],
        default_body: "Hi {tenant_name}, our records show an outstanding balance of {balance} as of {month}.{late_fee} Please arrange payment at your earliest convenience. Thank you.\n\n{signature}",
    },
    TemplateDef {
        kind: "lease_expiring",
        label: "Lease expiry reminder",
        description: "Sent when a current tenant's lease is within its reminder window.",
        group: "tenant",
        placeholders: &[
            Placeholder { token: "tenant_name", description: "Tenant's full name" },
            Placeholder { token: "address", description: "Property street address" },
            Placeholder { token: "city", description: "Property city" },
            Placeholder { token: "state", description: "Property state" },
            Placeholder { token: "zip", description: "Property ZIP code" },
            Placeholder { token: "end_date", description: "Lease end date" },
            Placeholder { token: "signature", description: "Your sign-off (set on the Admin page)" },
        ],
        default_body: "Hi {tenant_name}, your lease is set to expire on {end_date}. Please contact us to discuss renewal.\n\n{signature}",
    },
    TemplateDef {
        kind: "providers",
        label: "Utility & HOA info",
        description: "Sent to a tenant with the property's utility providers and HOA contacts.",
        group: "tenant",
        placeholders: &[
            Placeholder { token: "address", description: "Property street address" },
            Placeholder { token: "city", description: "Property city" },
            Placeholder { token: "state", description: "Property state" },
            Placeholder { token: "zip", description: "Property ZIP code" },
            Placeholder { token: "providers", description: "Bulleted list of utility providers" },
            Placeholder { token: "hoa", description: "HOA contact line (blank when none)" },
            Placeholder { token: "signature", description: "Your sign-off (set on the Admin page)" },
        ],
        default_body: "Utility providers for {address}, {city}, {state} {zip}:\n{providers}{hoa}\n\n{signature}",
    },
    TemplateDef {
        kind: "landlord_lease",
        label: "Lease expiry alert (landlord)",
        description: "Texted to your contact phones when a tenant's lease is ending or has ended.",
        group: "property",
        placeholders: &[
            Placeholder { token: "alert", description: "The lease alert detail (tenant, property, date)" },
        ],
        default_body: "{alert}",
    },
    TemplateDef {
        kind: "landlord_insurance",
        label: "Insurance expiry alert (landlord)",
        description: "Texted to your contact phones when a property's insurance is expiring or has expired.",
        group: "property",
        placeholders: &[
            Placeholder { token: "alert", description: "The insurance alert detail (property, provider, date)" },
        ],
        default_body: "{alert}",
    },
];

pub fn find(kind: &str) -> Option<&'static TemplateDef> {
    TEMPLATES.iter().find(|t| t.kind == kind)
}

/// The configured sign-off as plain text (blank when unset). Formatting around
/// it lives in the template body, so the operator controls spacing and styling.
pub async fn signature_value(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    Ok(settings::get_string(pool, settings::SIGNATURE)
        .await?
        .unwrap_or_default()
        .trim()
        .to_string())
}

fn settings_key(kind: &str) -> String {
    format!("template:{kind}")
}

/// The effective body for a template: the stored override if present and
/// non-blank, otherwise the built-in default.
pub async fn body(pool: &SqlitePool, kind: &str) -> Result<String, sqlx::Error> {
    let def = find(kind).expect("unknown template kind");
    let stored = settings::get_string(pool, &settings_key(kind)).await?;
    Ok(stored
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| def.default_body.to_string()))
}

/// Whether a non-default override is currently stored for `kind`.
pub async fn is_custom(pool: &SqlitePool, kind: &str) -> Result<bool, sqlx::Error> {
    Ok(settings::get_string(pool, &settings_key(kind))
        .await?
        .is_some_and(|s| !s.trim().is_empty()))
}

/// Persists an override, or clears it (reverting to the default) when `body`
/// is blank or identical to the built-in default.
pub async fn set_body(pool: &SqlitePool, kind: &str, body: &str) -> Result<(), sqlx::Error> {
    let def = find(kind).expect("unknown template kind");
    let trimmed = body.trim();
    if trimmed.is_empty() || trimmed == def.default_body {
        settings::delete(pool, &settings_key(kind)).await
    } else {
        settings::set_string(pool, &settings_key(kind), body).await
    }
}

/// Substitutes `{token}` occurrences with their values; unknown tokens are
/// left intact so a typo degrades gracefully rather than dropping text.
pub fn render(template: &str, vars: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    for (token, value) in vars {
        out = out.replace(&format!("{{{token}}}"), value);
    }
    out
}
