export type PropertyKind = "rental" | "personal";
export type TxKind = "income" | "expense";
export type TxBorneBy = "landlord" | "tenant";
export type TxField = "date" | "amount" | "tenant" | "description" | "receipt";

export interface Property {
  id: string;
  name: string;
  address: string;
  city: string;
  state: string;
  zip: string;
  kind: PropertyKind;
  purchase_date: string | null;
  notes: string;
  hoa_name: string;
  hoa_phone: string;
  hoa_email: string;
  hoa_webpage: string;
  created_at: string;
}

export interface PropertyInput {
  name: string;
  address?: string;
  city?: string;
  state?: string;
  zip?: string;
  kind?: PropertyKind;
  purchase_date?: string | null;
  notes?: string;
  hoa_name?: string;
  hoa_phone?: string;
  hoa_email?: string;
  hoa_webpage?: string;
}

export interface Lease {
  id: string;
  tenant_id: string;
  monthly_rent: number;
  start_date: string | null;
  end_date: string | null;
  payment_date: string | null;
  late_fee: number;
  notes: string;
  created_at: string;
}

export interface LeaseInput {
  monthly_rent?: number;
  start_date?: string | null;
  end_date?: string | null;
  payment_date?: string | null;
  late_fee?: number;
  notify_days?: number;
  notes?: string;
}

export interface Tenant {
  id: string;
  property_id: string;
  first_name: string;
  last_name: string;
  email: string;
  phone: string;
  is_current: boolean;
  notifications_enabled: boolean;
  notes: string;
  driver_license_id: string | null;
  created_at: string;
  leases: Lease[];
  active_lease: Lease | null;
}

/// Full display name for a tenant, tolerant of a missing last name.
export function tenantName(t: Pick<Tenant, "first_name" | "last_name">): string {
  return `${t.first_name} ${t.last_name}`.trim();
}

export interface TenantInput {
  first_name: string;
  last_name?: string;
  email?: string;
  phone?: string;
  is_current?: boolean;
  notifications_enabled?: boolean;
  notes?: string;
  driver_license_id?: string | null;
}

export interface Transaction {
  id: string;
  property_id: string;
  kind: TxKind;
  category: string;
  amount: number;
  date: string;
  description: string;
  tenant_name: string;
  borne_by: TxBorneBy;
  receipt_id: string | null;
  created_at: string;
}

export interface TransactionInput {
  kind: TxKind;
  category?: string;
  amount: number;
  date: string;
  description?: string;
  tenant_name?: string;
  borne_by?: TxBorneBy;
  receipt_id?: string | null;
}

export type InsuranceStatus = "expired" | "expiring" | "active";

export interface InsurancePolicy {
  id: string;
  property_id: string;
  provider: string;
  policy_number: string;
  premium: number;
  start_date: string | null;
  expiry_date: string;
  notes: string;
  created_at: string;
  days_until_expiry: number;
  status: InsuranceStatus;
}

export interface InsuranceInput {
  provider: string;
  policy_number?: string;
  premium?: number;
  start_date?: string | null;
  expiry_date: string;
  notify_days?: number;
  notes?: string;
}

export interface Attachment {
  id: string;
  stored_name: string;
  original_name: string;
  content_type: string;
  size: number;
  uploaded_at: string;
}

export interface PropertySummary {
  total_income: number;
  total_expense: number;
}

export interface CategoryTotal {
  kind: TxKind;
  category: string;
  total: number;
}

export interface OutstandingBalance {
  monthly_rent: number;
  expected: number;
  paid: number;
  carry_over: number;
  outstanding: number;
  year: string;
}

export interface OverviewRow {
  property: Property;
  total_income: number;
  total_expense: number;
  net: number;
  outstanding: number;
}

export interface PortfolioTotals {
  kind: PropertyKind;
  income: number;
  expense: number;
  net: number;
  outstanding: number;
  gain_pct: number | null;
}

export interface OverviewResponse {
  rows: OverviewRow[];
  totals: PortfolioTotals[];
}

export interface TaxCategoryTotal {
  category: string;
  total: number;
}

export interface TaxPropertyReport {
  property: Property;
  income: TaxCategoryTotal[];
  expense: TaxCategoryTotal[];
  total_income: number;
  total_expense: number;
  net: number;
}

export interface TaxReport {
  year: string;
  properties: TaxPropertyReport[];
  income: TaxCategoryTotal[];
  expense: TaxCategoryTotal[];
  total_income: number;
  total_expense: number;
  net: number;
}

/// A category resolved by the backend for a given property kind.
export interface CategoryInfo {
  name: string;
  kind: TxKind;
  fields: TxField[];
  applies: boolean;
  deductible: boolean;
}

/// A US state served by the backend for address selection.
export interface UsState {
  code: string;
  name: string;
}

/// An actionable alert resolved entirely by the backend.
export type NotificationSeverity = "info" | "warning" | "error";

export interface Notification {
  id: string;
  kind: string;
  severity: NotificationSeverity;
  title: string;
  body: string;
  link: string | null;
  property_id: string | null;
  created_at: string;
}

/// A text message sent (or attempted) to a tenant.
export type MessageStatus = "queued" | "sent" | "failed";

export interface Message {
  id: string;
  tenant_id: string;
  property_id: string;
  kind: string;
  to_phone: string;
  body: string;
  status: MessageStatus;
  error: string | null;
  created_at: string;
  sent_at: string | null;
}

export interface MessageInput {
  kind?: string;
  body: string;
}

export interface Settings {
  messaging_enabled: boolean;
}

/// A utility provider a tenant contacts to set up service.
export interface Provider {
  id: string;
  property_id: string;
  kind: string;
  name: string;
  phone: string;
  homepage: string;
  created_at: string;
}

export interface ProviderInput {
  kind: string;
  name: string;
  phone?: string;
  homepage?: string;
}

/// Turns a non-OK response into an Error carrying the backend's message.
async function toError(res: Response, fallback = "request failed"): Promise<Error> {
  const body = await res.json().catch(() => ({ error: res.statusText }));
  return new Error(body.error ?? fallback);
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`/api${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
    cache: "no-store",
  });
  if (!res.ok) throw await toError(res);
  if (res.status === 204) return undefined as T;
  return res.json();
}

type CachedEnvelope<T> = { etag: string | null; data: T };

function cacheKey(path: string): string {
  return `pfp:cache:${path}`;
}

function readCache<T>(path: string): CachedEnvelope<T> | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(cacheKey(path));
    return raw ? (JSON.parse(raw) as CachedEnvelope<T>) : null;
  } catch {
    return null;
  }
}

function writeCache<T>(path: string, env: CachedEnvelope<T>): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(cacheKey(path), JSON.stringify(env));
  } catch {
    // Best-effort: ignore quota or serialization failures.
  }
}

// GET slow-changing data with ETag revalidation. Sends the stored hash via
// If-None-Match; on 304 the cached body is reused, and new data is only
// downloaded (and persisted) when the server's hash has changed.
async function cachedRequest<T>(path: string): Promise<T> {
  const cached = readCache<T>(path);
  const res = await fetch(`/api${path}`, {
    headers: cached?.etag ? { "If-None-Match": cached.etag } : undefined,
    cache: "no-store",
  });
  if (res.status === 304 && cached) return cached.data;
  if (!res.ok) throw await toError(res);
  const data = (await res.json()) as T;
  writeCache<T>(path, { etag: res.headers.get("ETag"), data });
  return data;
}

/// Optional `?year=` query for the endpoints that accept a year filter; empty
/// (all years) when no specific year is selected.
function yearQuery(year?: number | "all"): string {
  return year && year !== "all" ? `?year=${year}` : "";
}

export const api = {
  overview: (year?: number | "all") =>
    request<OverviewResponse>(`/overview${yearQuery(year)}`),
  taxReport: (year?: number | "all") =>
    request<TaxReport>(`/tax-report${yearQuery(year)}`),
  getProperty: (id: string) => request<Property>(`/properties/${id}`),
  createProperty: (input: PropertyInput) =>
    request<Property>("/properties", { method: "POST", body: JSON.stringify(input) }),
  updateProperty: (id: string, input: PropertyInput) =>
    request<Property>(`/properties/${id}`, { method: "PUT", body: JSON.stringify(input) }),
  propertySummary: (id: string, year?: number | "all") =>
    request<PropertySummary>(`/properties/${id}/summary${yearQuery(year)}`),
  propertyBreakdown: (id: string, year?: number | "all") =>
    request<CategoryTotal[]>(`/properties/${id}/breakdown${yearQuery(year)}`),
  propertyOutstanding: (id: string, year?: number | "all") =>
    request<OutstandingBalance>(`/properties/${id}/outstanding${yearQuery(year)}`),

  categories: (kind: PropertyKind) =>
    cachedRequest<CategoryInfo[]>(`/categories?kind=${kind}`),

  states: () => cachedRequest<UsState[]>("/states"),

  notifications: () => request<Notification[]>("/notifications"),
  dismissNotification: (id: string) =>
    request<void>(`/notifications/${id}/dismiss`, { method: "POST" }),

  getSettings: () => request<Settings>("/settings"),
  updateSettings: (input: Settings) =>
    request<Settings>("/settings", { method: "PUT", body: JSON.stringify(input) }),

  listTenants: (propertyId: string) =>
    request<Tenant[]>(`/properties/${propertyId}/tenants`),
  createTenant: (propertyId: string, input: TenantInput) =>
    request<Tenant>(`/properties/${propertyId}/tenants`, {
      method: "POST",
      body: JSON.stringify(input),
    }),
  updateTenant: (id: string, input: TenantInput) =>
    request<Tenant>(`/tenants/${id}`, {
      method: "PUT",
      body: JSON.stringify(input),
    }),
  deleteTenant: (id: string) =>
    request<void>(`/tenants/${id}`, { method: "DELETE" }),

  createLease: (tenantId: string, input: LeaseInput) =>
    request<Lease>(`/tenants/${tenantId}/leases`, {
      method: "POST",
      body: JSON.stringify(input),
    }),
  updateLease: (id: string, input: LeaseInput) =>
    request<Lease>(`/leases/${id}`, { method: "PUT", body: JSON.stringify(input) }),
  deleteLease: (id: string) =>
    request<void>(`/leases/${id}`, { method: "DELETE" }),

  listTransactions: (propertyId: string) =>
    request<Transaction[]>(`/properties/${propertyId}/transactions`),
  createTransaction: (propertyId: string, input: TransactionInput) =>
    request<Transaction>(`/properties/${propertyId}/transactions`, {
      method: "POST",
      body: JSON.stringify(input),
    }),
  updateTransaction: (id: string, input: TransactionInput) =>
    request<Transaction>(`/transactions/${id}`, {
      method: "PUT",
      body: JSON.stringify(input),
    }),
  deleteTransaction: (id: string) =>
    request<void>(`/transactions/${id}`, { method: "DELETE" }),

  listInsurance: (propertyId: string) =>
    request<InsurancePolicy[]>(`/properties/${propertyId}/insurance`),
  createInsurance: (propertyId: string, input: InsuranceInput) =>
    request<InsurancePolicy>(`/properties/${propertyId}/insurance`, {
      method: "POST",
      body: JSON.stringify(input),
    }),
  deleteInsurance: (id: string) =>
    request<void>(`/insurance/${id}`, { method: "DELETE" }),

  listMessages: (propertyId: string) =>
    request<Message[]>(`/properties/${propertyId}/messages`),
  sendMessage: (tenantId: string, input: MessageInput) =>
    request<Message>(`/tenants/${tenantId}/messages`, {
      method: "POST",
      body: JSON.stringify(input),
    }),
  sendProviders: (tenantId: string) =>
    request<Message>(`/tenants/${tenantId}/messages/providers`, { method: "POST" }),
  previewProviders: (propertyId: string) =>
    request<{ body: string }>(`/properties/${propertyId}/providers/message`),

  listProviders: (propertyId: string) =>
    request<Provider[]>(`/properties/${propertyId}/providers`),
  createProvider: (propertyId: string, input: ProviderInput) =>
    request<Provider>(`/properties/${propertyId}/providers`, {
      method: "POST",
      body: JSON.stringify(input),
    }),
  deleteProvider: (id: string) =>
    request<void>(`/providers/${id}`, { method: "DELETE" }),

  uploadAttachment: async (file: File): Promise<Attachment> => {
    const form = new FormData();
    form.append("file", file);
    const res = await fetch("/api/attachments", { method: "POST", body: form });
    if (!res.ok) throw await toError(res, "upload failed");
    return res.json();
  },
  attachmentUrl: (id: string) => `/api/attachments/${id}`,
};

export function formatCurrency(value: number): string {
  return new Intl.NumberFormat(undefined, {
    style: "currency",
    currency: "USD",
  }).format(value);
}

/// Single source of truth for a property's display address: "street, city, ST zip".
/// The street is dropped when it merely repeats the property name (case-insensitive),
/// since names are often the street address itself.
export function formatPropertyAddress(
  property: Pick<Property, "name" | "address" | "city" | "state" | "zip">,
): string {
  const street =
    property.address.trim().toLowerCase() === property.name.trim().toLowerCase()
      ? ""
      : property.address;
  const cityZip = [property.city, [property.state, property.zip].filter(Boolean).join(" ")]
    .filter(Boolean)
    .join(", ");
  return [street, cityZip].filter(Boolean).join(", ");
}

// Shared React Query options for slow-changing reference data. Kept fresh for
// the whole session (staleTime: Infinity); cross-reload refetches are gated by
// the ETag hash in cachedRequest, so the network is hit only when data changes.
export const statesQueryOptions = {
  queryKey: ["states"] as const,
  queryFn: api.states,
  staleTime: Infinity,
  gcTime: Infinity,
};

export function categoriesQueryOptions(kind: PropertyKind) {
  return {
    queryKey: ["categories", kind] as const,
    queryFn: () => api.categories(kind),
    staleTime: Infinity,
    gcTime: Infinity,
  };
}
