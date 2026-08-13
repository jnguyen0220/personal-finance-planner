"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  api,
  formatCurrency,
  formatPropertyAddress,
  statesQueryOptions,
  type OverviewRow,
  type PortfolioTotals,
  type PropertyKind,
} from "@/lib/api";
import { YearSelect } from "@/components/ui/YearSelect";

export default function Home() {
  const queryClient = useQueryClient();

  const [name, setName] = useState("");
  const [address, setAddress] = useState("");
  const [city, setCity] = useState("");
  const [state, setStateField] = useState("");
  const [zip, setZip] = useState("");
  const [kind, setKind] = useState<PropertyKind>("rental");
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [year, setYear] = useState<number | "all">(new Date().getFullYear());
  const [kindFilter, setKindFilter] = useState<PropertyKind>("rental");

  // Preselect the tab matching the property we navigated back from.
  useEffect(() => {
    const type = new URLSearchParams(window.location.search).get("type");
    if (type === "personal" || type === "rental") setKindFilter(type);
  }, []);

  // One aggregate request per year covers every property's summary and rent owed.
  const { data, isLoading, error } = useQuery({
    queryKey: ["overview", year],
    queryFn: () => api.overview(year),
  });
  const rows: OverviewRow[] = useMemo(() => data?.rows ?? [], [data]);

  const { data: states = [] } = useQuery(statesQueryOptions);

  const yearOptions = useMemo(() => {
    const current = new Date().getFullYear();
    return Array.from({ length: 6 }, (_, i) => current - i);
  }, []);

  async function addProperty(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    setSaving(true);
    setFormError(null);
    try {
      await api.createProperty({ name, address, city, state, zip, kind });
      setName("");
      setAddress("");
      setCity("");
      setStateField("");
      setZip("");
      setKind("rental");
      await queryClient.invalidateQueries({ queryKey: ["overview"] });
    } catch (e) {
      setFormError((e as Error).message);
    } finally {
      setSaving(false);
    }
  }

  const visibleRows = useMemo(
    () => rows.filter((r) => r.property.kind === kindFilter),
    [rows, kindFilter],
  );

  // Header-card figures are computed by the backend; pick the active kind.
  const totals: PortfolioTotals = useMemo(
    () =>
      data?.totals.find((t) => t.kind === kindFilter) ?? {
        kind: kindFilter,
        income: 0,
        expense: 0,
        net: 0,
        outstanding: 0,
        gain_pct: null,
      },
    [data, kindFilter],
  );

  return (
    <main className="mx-auto max-w-5xl px-6 py-8">
      <div className="mb-6 flex items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Portfolio overview</h1>
          <p className="mt-1 text-sm text-[var(--muted)]">
            {visibleRows.length} {visibleRows.length === 1 ? "property" : "properties"} tracked ·{" "}
            {year === "all" ? "all years" : year}
          </p>
        </div>
        <YearSelect value={year} options={yearOptions} onChange={setYear} />
      </div>

      {error && (
        <p className="mb-4 rounded-lg border border-red-300 bg-red-50 px-4 py-2.5 text-sm text-red-700">
          {(error as Error).message}
        </p>
      )}

      <section className="mb-8 card p-5">
        <h2 className="mb-4 text-sm font-semibold">Add a property</h2>
        {formError && <p className="mb-3 text-sm text-red-700">{formError}</p>}
        <form onSubmit={addProperty} className="flex flex-wrap items-end gap-3">
          <label className="flex flex-1 flex-col" style={{ minWidth: "12rem" }}>
            <span className="label">Name</span>
            <input
              className="input"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. 12 Oak Street"
              required
            />
          </label>
          <label className="flex flex-1 flex-col" style={{ minWidth: "12rem" }}>
            <span className="label">Address</span>
            <input
              className="input"
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              placeholder="Optional"
            />
          </label>
          <label className="flex flex-col">
            <span className="label">City</span>
            <input
              className="input"
              value={city}
              onChange={(e) => setCity(e.target.value)}
              placeholder="Optional"
            />
          </label>
          <label className="flex flex-col">
            <span className="label">State</span>
            <select
              className="input"
              value={state}
              onChange={(e) => setStateField(e.target.value)}
            >
              <option value="">—</option>
              {states.map((s) => (
                <option key={s.code} value={s.code}>
                  {s.name}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col">
            <span className="label">Zip</span>
            <input
              className="input w-28"
              value={zip}
              onChange={(e) => setZip(e.target.value)}
              inputMode="numeric"
              pattern="\d{5}"
              maxLength={5}
              placeholder="12345"
            />
          </label>
          <label className="flex flex-col">
            <span className="label">Type</span>
            <select
              className="input"
              value={kind}
              onChange={(e) => setKind(e.target.value as PropertyKind)}
            >
              <option value="rental">Rental</option>
              <option value="personal">Personal</option>
            </select>
          </label>
          <button type="submit" disabled={saving} className="btn-primary">
            {saving ? "Saving…" : "Add property"}
          </button>
        </form>
      </section>

      {!isLoading && rows.length > 0 && (
        <>
          <div className="mb-4 flex flex-wrap gap-2">
            {(["rental", "personal"] as const).map((k) => (
              <button
                key={k}
                onClick={() => setKindFilter(k)}
                className={`pill ${kindFilter === k ? "pill-active" : ""}`}
              >
                {k === "rental" ? "Rentals" : "Personal"}
              </button>
            ))}
          </div>
          {kindFilter === "rental" && (
            <section className="mb-6 grid gap-4 grid-cols-2 lg:grid-cols-4">
              <SummaryCard label="Total income" value={formatCurrency(totals.income)} tone="up" />
              <SummaryCard label="Total expense" value={formatCurrency(totals.expense)} tone="down" />
              <SummaryCard
                label="Net position"
                value={formatCurrency(totals.net)}
                tone={totals.net >= 0 ? "up" : "down"}
              />
              <SummaryCard
                label="Percent gain"
                value={totals.gain_pct === null ? "—" : `${totals.gain_pct >= 0 ? "+" : ""}${totals.gain_pct.toFixed(1)}%`}
                tone={totals.gain_pct !== null && totals.gain_pct < 0 ? "down" : "up"}
              />
            </section>
          )}
          {totals.outstanding > 0.005 && kindFilter !== "personal" && (
            <div className="mb-6 card border-amber-300/70 bg-amber-50/70 p-4">
              <div className="flex items-center gap-2">
                <span className="badge bg-amber-200 text-amber-900">Outstanding</span>
                <span className="text-sm font-semibold text-amber-900">
                  {formatCurrency(totals.outstanding)} in unpaid rent across your rentals
                </span>
              </div>
            </div>
          )}
        </>
      )}

      {isLoading ? (
        <div className="card h-48 animate-pulse" />
      ) : rows.length === 0 ? (
        <div className="card flex flex-col items-center justify-center px-6 py-16 text-center">
          <p className="text-sm font-medium">No properties yet</p>
          <p className="mt-1 text-sm text-[var(--muted)]">
            Add your first property using the form above to get started.
          </p>
        </div>
      ) : visibleRows.length === 0 ? (
        <div className="card flex flex-col items-center justify-center px-6 py-16 text-center">
          <p className="text-sm font-medium">No matching properties</p>
          <p className="mt-1 text-sm text-[var(--muted)]">
            No {kindFilter === "personal" ? "homes" : "rentals"} to show. Try a different filter.
          </p>
        </div>
      ) : kindFilter === "personal" ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {visibleRows.map(({ property, total_expense }) => {
            const location = formatPropertyAddress(property);
            return (
              <Link
                key={property.id}
                href={`/properties/${property.id}`}
                className="group card card-hover p-5"
              >
                <div className="flex items-start justify-between gap-2">
                  <span className="font-semibold group-hover:text-indigo-600">{property.name}</span>
                </div>
                {location && <p className="mt-0.5 text-xs text-[var(--muted)]">{location}</p>}
                <p className="mt-4 text-xs font-medium uppercase tracking-wide text-[var(--muted)]">
                  Total expense
                </p>
                <p className="mt-1 text-2xl font-bold tracking-tight text-red-600">
                  {formatCurrency(total_expense)}
                </p>
              </Link>
            );
          })}
        </div>
      ) : (
        <div className="table-card">
          <div className="table-scroll">
          <table className="data-table">
            <thead>
              <tr>
                <th className="th">Property</th>
                <th className="th hidden sm:table-cell">Type</th>
                <th className="th text-right">Income</th>
                <th className="th text-right">Expense</th>
                <th className="th text-right">Net</th>
                <th className="th hidden text-right sm:table-cell">Outstanding</th>
              </tr>
            </thead>
            <tbody>
              {visibleRows.map(({ property, total_income, total_expense, net, outstanding }) => {
                const location = formatPropertyAddress(property);
                return (
                  <tr key={property.id} className="group">
                    <td className="td">
                      <Link href={`/properties/${property.id}`} className="block">
                        <span className="font-semibold group-hover:text-indigo-600">
                          {property.name}
                        </span>
                        {location && (
                          <span className="mt-0.5 block truncate text-xs text-[var(--muted)]">
                            {location}
                          </span>
                        )}
                      </Link>
                    </td>
                    <td className="td hidden sm:table-cell">
                      <span
                        className={`badge ${
                          property.kind === "personal"
                            ? "bg-violet-100 text-violet-700"
                            : "bg-indigo-100 text-indigo-700"
                        }`}
                      >
                        {property.kind === "personal" ? "Personal" : "Rental"}
                      </span>
                    </td>
                    <td className="td text-right font-semibold tabular-nums text-emerald-600">
                      {formatCurrency(total_income)}
                    </td>
                    <td className="td text-right font-semibold tabular-nums text-red-600">
                      {formatCurrency(total_expense)}
                    </td>
                    <td
                      className={`td text-right font-semibold tabular-nums ${
                        net >= 0 ? "text-emerald-600" : "text-red-600"
                      }`}
                    >
                      {formatCurrency(net)}
                    </td>
                    <td className="td hidden text-right font-semibold tabular-nums sm:table-cell">
                      {property.kind === "rental" && outstanding > 0.005 ? (
                        <span className="text-amber-700">{formatCurrency(outstanding)}</span>
                      ) : (
                        <span className="text-[var(--muted)]">—</span>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          </div>
        </div>
      )}
    </main>
  );
}

function SummaryCard({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone: "up" | "down";
}) {
  return (
    <div className="card p-4">
      <p className="text-xs font-medium uppercase tracking-wide text-[var(--muted)]">{label}</p>
      <p
        className={`mt-1 text-2xl font-bold tracking-tight ${
          tone === "up" ? "text-emerald-600" : "text-red-600"
        }`}
      >
        {value}
      </p>
    </div>
  );
}
