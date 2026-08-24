use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::dates;

#[derive(Serialize, FromRow)]
pub struct Property {
    pub id: String,
    pub name: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub kind: String,
    pub reminders_enabled: bool,
    pub purchase_date: Option<String>,
    pub notes: String,
    pub hoa_name: String,
    pub hoa_phone: String,
    pub hoa_email: String,
    pub hoa_webpage: String,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct PropertyInput {
    pub name: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub zip: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    /// Whether automated reminders may be sent to this property's tenants.
    #[serde(default = "default_true")]
    pub reminders_enabled: bool,
    pub purchase_date: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub hoa_name: String,
    #[serde(default)]
    pub hoa_phone: String,
    #[serde(default)]
    pub hoa_email: String,
    #[serde(default)]
    pub hoa_webpage: String,
}

fn default_kind() -> String {
    "rental".to_string()
}

#[derive(Serialize, FromRow)]
pub struct Tenant {
    pub id: String,
    pub property_id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub is_current: bool,
    pub notes: String,
    pub driver_license_id: Option<String>,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct TenantInput {
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub is_current: bool,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub driver_license_id: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, FromRow, Clone)]
pub struct Lease {
    pub id: String,
    pub tenant_id: String,
    pub monthly_rent: f64,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub rent_due_day: Option<i64>,
    pub late_fee: f64,
    pub notes: String,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct LeaseInput {
    #[serde(default)]
    pub monthly_rent: f64,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub rent_due_day: Option<i64>,
    #[serde(default)]
    pub late_fee: f64,
    #[serde(default)]
    pub notes: String,
}

#[derive(Serialize)]
pub struct TenantWithLeases {
    #[serde(flatten)]
    pub tenant: Tenant,
    pub leases: Vec<Lease>,
    /// The lease in effect today, resolved server-side for display.
    pub active_lease: Option<Lease>,
}

impl TenantWithLeases {
    /// Builds the view, selecting the lease that is active today.
    pub fn new(tenant: Tenant, leases: Vec<Lease>) -> Self {
        let active_lease = active_lease(&leases);
        TenantWithLeases {
            tenant,
            leases,
            active_lease,
        }
    }
}

/// The lease in effect today: started, not yet ended, most recently started wins.
fn active_lease(leases: &[Lease]) -> Option<Lease> {
    let today = dates::today().format("%Y-%m-%d").to_string();
    leases
        .iter()
        .filter(|l| {
            l.start_date.as_deref().is_none_or(|s| s <= today.as_str())
                && l.end_date.as_deref().is_none_or(|e| e >= today.as_str())
        })
        .max_by(|a, b| {
            a.start_date
                .as_deref()
                .unwrap_or("")
                .cmp(b.start_date.as_deref().unwrap_or(""))
        })
        .cloned()
}

#[derive(Serialize, FromRow)]
pub struct Transaction {
    pub id: String,
    pub property_id: String,
    pub kind: String,
    pub category_id: String,
    pub category_label: String,
    pub amount: f64,
    pub date: String,
    pub description: String,
    pub tenant_name: String,
    pub borne_by: String,
    pub receipt_id: Option<String>,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct TransactionInput {
    pub category_id: String,
    pub amount: f64,
    pub date: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tenant_name: String,
    #[serde(default = "default_borne_by")]
    pub borne_by: String,
    pub receipt_id: Option<String>,
}

fn default_borne_by() -> String {
    "landlord".to_string()
}

#[derive(Serialize, FromRow)]
pub struct InsurancePolicy {
    pub id: String,
    pub property_id: String,
    pub provider: String,
    pub policy_number: String,
    pub premium: f64,
    pub start_date: Option<String>,
    pub expiry_date: String,
    pub notes: String,
    pub created_at: String,
}

/// A policy with its expiry evaluated against today, so the client renders
/// status without any date math of its own.
#[derive(Serialize)]
pub struct InsurancePolicyView {
    #[serde(flatten)]
    pub policy: InsurancePolicy,
    pub days_until_expiry: i64,
    pub status: String,
}

impl InsurancePolicyView {
    pub fn from_policy(policy: InsurancePolicy) -> Self {
        let days_until_expiry = dates::days_until(&policy.expiry_date);
        // A policy is "expiring" within the 30-day renewal window.
        let status = if days_until_expiry < 0 {
            "expired"
        } else if days_until_expiry <= 30 {
            "expiring"
        } else {
            "active"
        }
        .to_string();
        InsurancePolicyView {
            policy,
            days_until_expiry,
            status,
        }
    }
}

/// A persisted, presentation-ready notification. Any backend event can create
/// one; the client only renders and dismisses them.
#[derive(Serialize, FromRow)]
pub struct Notification {
    pub id: String,
    pub kind: String,
    pub severity: String,
    pub title: String,
    pub body: String,
    pub link: Option<String>,
    pub property_id: Option<String>,
    pub created_at: String,
}

/// A text message sent (or attempted) to a tenant, with its delivery status.
#[derive(Serialize, FromRow)]
pub struct Message {
    pub id: String,
    pub tenant_id: String,
    pub property_id: String,
    pub kind: String,
    pub to_phone: String,
    pub body: String,
    pub status: String,
    pub error: Option<String>,
    pub created_at: String,
    pub sent_at: Option<String>,
}

#[derive(Deserialize)]
pub struct MessageInput {
    /// Origin of the message, e.g. 'outstanding_balance', 'lease_expiring', 'custom'.
    #[serde(default = "default_message_kind")]
    pub kind: String,
    pub body: String,
}

/// Message `kind` values for messages composed here (rather than from a named
/// template). The single source for these origin strings.
pub mod message_kind {
    pub const CUSTOM: &str = "custom";
    pub const BROADCAST: &str = "broadcast";
    pub const PROVIDERS: &str = "providers";
}

fn default_message_kind() -> String {
    message_kind::CUSTOM.to_string()
}

/// Outcome of a broadcast: how many recipients were attempted and delivered.
#[derive(Serialize)]
pub struct BroadcastResult {
    pub total: usize,
    pub sent: usize,
    pub failed: usize,
}

/// A current tenant a broadcast would reach.
#[derive(Serialize, FromRow)]
pub struct BroadcastRecipient {
    pub id: String,
    pub name: String,
    pub phone: String,
    pub property_name: String,
}

/// Global application settings the operator can toggle at runtime.
#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub messaging_enabled: bool,
    pub property_messaging_enabled: bool,
    pub signature: String,
    pub lease_notify_days: i64,
    pub insurance_notify_days: i64,
    pub contact_phones: Vec<String>,
    pub daily_email_enabled: bool,
    pub daily_reminders_enabled: bool,
    /// Next scheduled daily-job run (RFC 3339); read-only, ignored on update.
    pub next_daily_run: String,
}

/// Partial settings update: only the provided fields are written.
#[derive(Deserialize)]
pub struct SettingsUpdate {
    pub messaging_enabled: Option<bool>,
    pub property_messaging_enabled: Option<bool>,
    pub signature: Option<String>,
    pub lease_notify_days: Option<i64>,
    pub insurance_notify_days: Option<i64>,
    pub contact_phones: Option<Vec<String>>,
    pub daily_email_enabled: Option<bool>,
    pub daily_reminders_enabled: Option<bool>,
}

/// A utility provider (electricity, water, gas, trash, …) for a property, with
/// the contact details a tenant needs to set up service.
#[derive(Serialize, FromRow)]
pub struct Provider {
    pub id: String,
    pub property_id: String,
    pub kind: String,
    pub name: String,
    pub phone: String,
    pub homepage: String,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct ProviderInput {
    #[serde(default = "default_provider_kind")]
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub homepage: String,
}

fn default_provider_kind() -> String {
    "other".to_string()
}

#[derive(Deserialize)]
pub struct InsuranceInput {
    pub provider: String,
    #[serde(default)]
    pub policy_number: String,
    #[serde(default)]
    pub premium: f64,
    pub start_date: Option<String>,
    pub expiry_date: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Serialize, FromRow)]
pub struct Attachment {
    pub id: String,
    pub stored_name: String,
    pub original_name: String,
    pub content_type: String,
    pub size: i64,
    pub uploaded_at: String,
}

/// An inbound invoice attachment awaiting review. Joins its attachment so the
/// UI can preview the file before assignment.
#[derive(Serialize, FromRow)]
pub struct InboxItem {
    pub id: String,
    pub gmail_id: String,
    pub thread_id: String,
    pub from_addr: String,
    pub subject: String,
    pub snippet: String,
    pub received_at: String,
    pub attachment_id: Option<String>,
    pub attachment_name: Option<String>,
    pub attachment_type: Option<String>,
    pub status: String,
    pub created_at: String,
}

/// The details a reviewer supplies to file an inbox invoice as a transaction.
#[derive(Deserialize)]
pub struct InboxAssignInput {
    pub property_id: String,
    pub category_id: String,
    pub amount: f64,
    pub date: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tenant_name: String,
    #[serde(default = "default_borne_by")]
    pub borne_by: String,
}

#[derive(Serialize, FromRow)]
pub struct PropertySummary {
    pub total_income: f64,
    pub total_expense: f64,
}

#[derive(Deserialize)]
pub struct SummaryQuery {
    pub year: Option<String>,
}

#[derive(Serialize, FromRow)]
pub struct CategoryTotal {
    pub kind: String,
    pub category: String,
    pub total: f64,
}

#[derive(Serialize)]
pub struct OutstandingBalance {
    pub monthly_rent: f64,
    pub expected: f64,
    pub paid: f64,
    pub carry_over: f64,
    pub outstanding: f64,
    pub year: String,
}

#[derive(Serialize)]
pub struct OverviewRow {
    pub property: Property,
    pub total_income: f64,
    pub total_expense: f64,
    pub net: f64,
    pub outstanding: f64,
    pub monthly_rent: f64,
    pub tenant_name: Option<String>,
}

/// Portfolio roll-up for one property kind, computed server-side for the header cards.
#[derive(Serialize)]
pub struct PortfolioTotals {
    pub kind: String,
    pub income: f64,
    pub expense: f64,
    pub net: f64,
    pub outstanding: f64,
    /// Net as a percentage of spend; None when nothing has been spent yet.
    pub gain_pct: Option<f64>,
}

#[derive(Serialize)]
pub struct OverviewResponse {
    pub rows: Vec<OverviewRow>,
    pub totals: Vec<PortfolioTotals>,
}

/// One category's total for a tax year (income or expense side).
#[derive(Serialize)]
pub struct TaxCategoryTotal {
    pub category: String,
    pub total: f64,
}

/// A single rental's year figures, broken down by category for filing.
#[derive(Serialize)]
pub struct TaxPropertyReport {
    pub property: Property,
    pub income: Vec<TaxCategoryTotal>,
    pub expense: Vec<TaxCategoryTotal>,
    pub total_income: f64,
    pub total_expense: f64,
    pub net: f64,
}

/// Year-end tax report: every rental broken down by category, plus portfolio
/// totals, so a full Schedule E-style picture comes back in one request.
#[derive(Serialize)]
pub struct TaxReport {
    pub year: String,
    pub properties: Vec<TaxPropertyReport>,
    pub income: Vec<TaxCategoryTotal>,
    pub expense: Vec<TaxCategoryTotal>,
    pub total_income: f64,
    pub total_expense: f64,
    pub net: f64,
}
