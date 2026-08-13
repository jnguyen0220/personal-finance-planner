"use client";

import { useParams } from "next/navigation";
import { useCallback, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api, categoriesQueryOptions, formatCurrency, formatPhone, formatPropertyAddress } from "@/lib/api";
import { TransactionsTab } from "@/features/transactions/TransactionsTab";
import { TenantsTab } from "@/features/tenants/TenantsTab";
import { InsuranceTab } from "@/features/insurance/InsuranceTab";
import { MessagesTab } from "@/features/messages/MessagesTab";
import { ProvidersTab } from "@/features/providers/ProvidersTab";
import { PropertyEditForm } from "@/features/properties/PropertyEditForm";
import { EditButton } from "@/components/ui/EditButton";
import { YearSelect } from "@/components/ui/YearSelect";
import {
  BackLink,
  CategoryBreakdown,
  OutstandingBanner,
  PropertyIcon,
  Stat,
} from "@/features/properties/PropertySummary";

type Tab = "transactions" | "tenants" | "insurance" | "providers" | "messages";

export default function PropertyDetail() {
  const params = useParams<{ id: string }>();
  const id = params.id;
  const queryClient = useQueryClient();

  const [tab, setTab] = useState<Tab>("transactions");
  const [editing, setEditing] = useState(false);
  const [year, setYear] = useState<number | "all">(new Date().getFullYear());

  const propertyQuery = useQuery({ queryKey: ["property", id], queryFn: () => api.getProperty(id) });
  const tenantsQuery = useQuery({ queryKey: ["tenants", id], queryFn: () => api.listTenants(id) });
  const transactionsQuery = useQuery({
    queryKey: ["transactions", id],
    queryFn: () => api.listTransactions(id),
  });
  const policiesQuery = useQuery({ queryKey: ["insurance", id], queryFn: () => api.listInsurance(id) });
  const summaryQuery = useQuery({
    queryKey: ["summary", id, year],
    queryFn: () => api.propertySummary(id, year),
  });
  const breakdownQuery = useQuery({
    queryKey: ["breakdown", id, year],
    queryFn: () => api.propertyBreakdown(id, year),
  });
  const outstandingQuery = useQuery({
    queryKey: ["outstanding", id, year],
    queryFn: () => api.propertyOutstanding(id, year),
  });

  const property = propertyQuery.data ?? null;
  const categoriesQuery = useQuery({
    ...categoriesQueryOptions(property?.kind ?? "rental"),
    enabled: !!property,
  });
  const summary = summaryQuery.data ?? null;
  const tenants = tenantsQuery.data ?? [];
  const transactions = useMemo(() => transactionsQuery.data ?? [], [transactionsQuery.data]);
  const policies = policiesQuery.data ?? [];
  const breakdown = breakdownQuery.data ?? [];
  const outstanding = outstandingQuery.data ?? null;
  const categories = categoriesQuery.data ?? [];
  const error = propertyQuery.error ?? summaryQuery.error ?? transactionsQuery.error ?? null;

  // Mutations refetch only this property's queries plus the portfolio overview,
  // leaving other cached data (and cross-session reference data) untouched.
  const refresh = useCallback(async () => {
    await queryClient.invalidateQueries({
      predicate: (query) =>
        query.queryKey[1] === id || query.queryKey[0] === "overview",
    });
  }, [queryClient, id]);

  const yearOptions = useMemo(() => {
    const set = new Set<number>([new Date().getFullYear()]);
    for (const t of transactions) {
      const y = Number(t.date.slice(0, 4));
      if (!Number.isNaN(y)) set.add(y);
    }
    return Array.from(set).sort((a, b) => b - a);
  }, [transactions]);

  const filteredTransactions = useMemo(
    () =>
      year === "all"
        ? transactions
        : transactions.filter((t) => t.date.startsWith(`${year}`)),
    [transactions, year],
  );

  if (error) {
    return (
      <main className="mx-auto max-w-5xl px-6 py-8">
        <BackLink />
        <p className="mt-4 rounded-lg border border-red-300 bg-red-50 px-4 py-2.5 text-sm text-red-700">
          {(error as Error).message}
        </p>
      </main>
    );
  }

  if (!property || !summary) {
    return (
      <main className="mx-auto max-w-5xl px-6 py-8">
        <BackLink />
        <div className="mt-4 grid grid-cols-3 gap-4">
          {[0, 1, 2].map((i) => (
            <div key={i} className="card h-20 animate-pulse" />
          ))}
        </div>
      </main>
    );
  }

  const tabs: Tab[] =
    property.kind === "personal"
      ? ["transactions", "insurance", "providers"]
      : ["transactions", "tenants", "insurance", "providers", "messages"];
  const address = formatPropertyAddress(property);

  return (
    <main className="mx-auto max-w-5xl px-6 py-8">
      <BackLink kind={property.kind} />
      <div className="mt-4 space-y-4">
        <div className="flex flex-wrap items-start justify-between gap-4 rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-4 shadow-sm">
        <div className="flex min-w-0 items-center gap-4">
          <span
            className={`flex h-12 w-12 shrink-0 items-center justify-center rounded-xl p-2.5 ${
              property.kind === "personal"
                ? "bg-violet-50 text-violet-600"
                : "bg-indigo-50 text-indigo-600"
            }`}
          >
            <PropertyIcon kind={property.kind} />
          </span>
          <div className="min-w-0">
            <h1 className="truncate text-xl font-bold tracking-tight">{property.name}</h1>
            {address && (
              <p className="mt-1.5 text-sm font-medium text-[var(--muted)]">{address}</p>
            )}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-3">
          <span
            className={`badge ${
              property.kind === "personal"
                ? "bg-violet-100 text-violet-700"
                : "bg-indigo-100 text-indigo-700"
            }`}
          >
            {property.kind === "personal" ? "Personal" : "Rental"}
          </span>
          <EditButton label="Edit property" onEdit={() => setEditing(true)} />
        </div>
      </div>

      <div className="flex items-center justify-between gap-3">
        <h2 className="text-sm font-semibold uppercase tracking-wide text-[var(--muted)]">
          {year === "all" ? "All years" : year} summary
        </h2>
        <YearSelect value={year} options={yearOptions} onChange={setYear} />
      </div>

      {property.kind !== "personal" && (
        <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
          <Stat label="Income" value={formatCurrency(summary.total_income)} tone="green" />
          <Stat label="Expense" value={formatCurrency(summary.total_expense)} tone="red" />
          <Stat
            label="Net"
            value={formatCurrency(summary.total_income - summary.total_expense)}
            tone={summary.total_income - summary.total_expense >= 0 ? "green" : "red"}
          />
          {(() => {
            const net = summary.total_income - summary.total_expense;
            const gainPct = summary.total_expense > 0 ? (net / summary.total_expense) * 100 : null;
            return (
              <Stat
                label="Percent gain"
                value={gainPct === null ? "—" : `${gainPct >= 0 ? "+" : ""}${gainPct.toFixed(1)}%`}
                tone={gainPct !== null && gainPct < 0 ? "red" : "green"}
              />
            );
          })()}
        </div>
      )}

      {property.kind === "rental" && outstanding && outstanding.outstanding > 0.005 && (
        <OutstandingBanner data={outstanding} />
      )}

      <CategoryBreakdown items={breakdown} showIncome={property.kind !== "personal"} />

      {(property.hoa_name || property.hoa_phone || property.hoa_email || property.hoa_webpage) && (
        <div className="card p-4">
          <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">
            Homeowners association
          </p>
          <div className="flex flex-wrap items-center gap-x-6 gap-y-1 text-sm">
            {property.hoa_name && <span className="font-medium">{property.hoa_name}</span>}
            {property.hoa_phone && (
              <a href={`tel:${property.hoa_phone}`} className="text-indigo-600 hover:underline">
                {formatPhone(property.hoa_phone)}
              </a>
            )}
            {property.hoa_email && (
              <a href={`mailto:${property.hoa_email}`} className="text-indigo-600 hover:underline">
                {property.hoa_email}
              </a>
            )}
            {property.hoa_webpage && (
              <a
                href={property.hoa_webpage}
                target="_blank"
                rel="noopener noreferrer"
                className="text-indigo-600 hover:underline"
              >
                {property.hoa_webpage}
              </a>
            )}
          </div>
        </div>
      )}

      <div className="flex gap-1 border-b border-[var(--border)]">
        {tabs.map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`tab ${tab === t ? "tab-active" : ""}`}
          >
            {t}
          </button>
        ))}
      </div>

      {tab === "transactions" && (
        <TransactionsTab
          propertyId={id}
          categories={categories}
          tenants={tenants}
          transactions={filteredTransactions}
          onChange={refresh}
        />
      )}
      {tab === "tenants" && (
        <TenantsTab propertyId={id} tenants={tenants} outstanding={outstanding} onChange={refresh} />
      )}
      {tab === "insurance" && (
        <InsuranceTab propertyId={id} policies={policies} onChange={refresh} />
      )}
      {tab === "messages" && (
        <MessagesTab propertyId={id} tenants={tenants} />
      )}
      {tab === "providers" && (
        <ProvidersTab propertyId={id} tenants={tenants} propertyKind={property.kind} />
      )}
      </div>

      {editing && (
        <PropertyEditForm
          property={property}
          onClose={() => setEditing(false)}
          onSaved={async () => {
            setEditing(false);
            await refresh();
          }}
        />
      )}
    </main>
  );
}
