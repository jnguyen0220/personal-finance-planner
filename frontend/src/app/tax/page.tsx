"use client";

import Link from "next/link";
import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  api,
  formatCurrency,
  formatPropertyAddress,
  type TaxCategoryTotal,
  type TaxPropertyReport,
  type TaxReport,
} from "@/lib/api";

/// Flattens the report into a spreadsheet and triggers a client-side download.
/// A UTF-8 BOM keeps Excel happy; amounts stay raw so they parse as numbers.
function downloadCsv(report: TaxReport) {
  const esc = (v: string | number) => {
    const s = String(v);
    return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  };
  const row = (cells: (string | number)[]) => cells.map(esc).join(",");
  const lines: string[] = [`Tax report ${report.year}`, "", row(["Property", "Section", "Category", "Amount"])];

  for (const p of report.properties) {
    const n = p.property.name;
    for (const c of p.income) lines.push(row([n, "Income", c.category, c.total]));
    for (const c of p.expense) lines.push(row([n, "Expense", c.category, c.total]));
    lines.push(row([n, "Total", "Income", p.total_income]));
    lines.push(row([n, "Total", "Expense", p.total_expense]));
    lines.push(row([n, "Total", "Net", p.net]));
  }

  lines.push("");
  for (const c of report.income) lines.push(row(["Portfolio", "Income", c.category, c.total]));
  for (const c of report.expense) lines.push(row(["Portfolio", "Expense", c.category, c.total]));
  lines.push(row(["Portfolio", "Total", "Income", report.total_income]));
  lines.push(row(["Portfolio", "Total", "Expense", report.total_expense]));
  lines.push(row(["Portfolio", "Total", "Net", report.net]));

  const blob = new Blob(["\uFEFF" + lines.join("\r\n")], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `tax-report-${report.year}.csv`;
  a.click();
  URL.revokeObjectURL(url);
}

export default function TaxReportPage() {
  const [year, setYear] = useState<number>(new Date().getFullYear() - 1);

  const { data, isLoading, error } = useQuery({
    queryKey: ["tax-report", year],
    queryFn: () => api.taxReport(year),
  });

  const yearOptions = useMemo(() => {
    const current = new Date().getFullYear();
    return Array.from({ length: 7 }, (_, i) => current - i);
  }, []);

  const properties = data?.properties ?? [];

  return (
    <main className="mx-auto max-w-5xl px-6 py-8 print:py-0">
      <div className="mb-6 flex flex-wrap items-start justify-between gap-4">
        <div>
          <div className="flex items-center gap-2 text-sm text-[var(--muted)] print:hidden">
            <Link href="/" className="transition hover:text-[var(--foreground)]">
              Portfolio
            </Link>
            <span>/</span>
            <span>Tax report</span>
          </div>
          <h1 className="mt-1 text-2xl font-bold tracking-tight">Year-end tax report</h1>
          <p className="mt-1 text-sm text-[var(--muted)]">
            Rental income and expenses by category for {year}, ready for filing.
          </p>
        </div>
        <div className="flex items-center gap-2 print:hidden">
          <button
            type="button"
            onClick={() => data && downloadCsv(data)}
            disabled={!data}
            className="btn-secondary"
          >
            Export CSV
          </button>
          <button
            type="button"
            onClick={() => window.print()}
            disabled={!data}
            className="btn-secondary"
          >
            Save as PDF
          </button>
          <select
            className="input"
            value={String(year)}
            onChange={(e) => setYear(Number(e.target.value))}
          >
            {yearOptions.map((y) => (
              <option key={y} value={y}>
                {y}
              </option>
            ))}
          </select>
        </div>
      </div>

      {error && (
        <p className="mb-4 rounded-lg border border-red-300 bg-red-50 px-4 py-2.5 text-sm text-red-700">
          {(error as Error).message}
        </p>
      )}

      {isLoading ? (
        <div className="card h-48 animate-pulse" />
      ) : !data || properties.length === 0 ? (
        <div className="card flex flex-col items-center justify-center px-6 py-16 text-center">
          <p className="text-sm font-medium">No rental activity in {year}</p>
          <p className="mt-1 text-sm text-[var(--muted)]">
            Pick another year or add transactions to your rentals.
          </p>
        </div>
      ) : (
        <>
          <section className="mb-6 grid grid-cols-3 gap-4">
            <TotalCard label="Total income" value={formatCurrency(data.total_income)} tone="up" />
            <TotalCard label="Total expenses" value={formatCurrency(data.total_expense)} tone="down" />
            <TotalCard
              label="Net (taxable)"
              value={formatCurrency(data.net)}
              tone={data.net >= 0 ? "up" : "down"}
            />
          </section>

          <section className="mb-8 grid gap-4 md:grid-cols-2">
            <CategoryCard title="Income by category" items={data.income} tone="up" />
            <CategoryCard title="Expenses by category" items={data.expense} tone="down" />
          </section>

          <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-[var(--muted)]">
            By property
          </h2>
          <div className="space-y-4">
            {properties.map((p) => (
              <PropertyReport key={p.property.id} report={p} />
            ))}
          </div>
        </>
      )}
    </main>
  );
}

function PropertyReport({ report }: { report: TaxPropertyReport }) {
  const { property, income, expense, total_income, total_expense, net } = report;
  const location = formatPropertyAddress(property);

  return (
    <section className="card p-5 print:break-inside-avoid">
      <div className="mb-4 flex items-start justify-between gap-4">
        <div>
          <Link
            href={`/properties/${property.id}`}
            className="font-semibold tracking-tight transition hover:text-indigo-600"
          >
            {property.name}
          </Link>
          {location && <p className="text-sm text-[var(--muted)]">{location}</p>}
        </div>
        <div className="text-right">
          <p className="text-xs font-medium uppercase tracking-wide text-[var(--muted)]">Net</p>
          <p
            className={`text-lg font-bold tabular-nums ${
              net >= 0 ? "text-emerald-600" : "text-red-600"
            }`}
          >
            {formatCurrency(net)}
          </p>
        </div>
      </div>
      <div className="grid gap-4 md:grid-cols-2">
        <CategoryList title="Income" items={income} total={total_income} tone="up" />
        <CategoryList title="Expenses" items={expense} total={total_expense} tone="down" />
      </div>
    </section>
  );
}

function CategoryCard({
  title,
  items,
  tone,
}: {
  title: string;
  items: TaxCategoryTotal[];
  tone: "up" | "down";
}) {
  const total = items.reduce((s, i) => s + i.total, 0);
  return (
    <div className="card p-5">
      <CategoryList title={title} items={items} total={total} tone={tone} />
    </div>
  );
}

function CategoryList({
  title,
  items,
  total,
  tone,
}: {
  title: string;
  items: TaxCategoryTotal[];
  total: number;
  tone: "up" | "down";
}) {
  const color = tone === "up" ? "text-emerald-600" : "text-red-600";
  return (
    <div>
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">{title}</h3>
        <span className={`text-sm font-bold tabular-nums ${color}`}>{formatCurrency(total)}</span>
      </div>
      {items.length === 0 ? (
        <p className="py-2 text-sm text-[var(--muted)]">None</p>
      ) : (
        <ul className="divide-y divide-[var(--border)]">
          {items.map((i) => (
            <li key={i.category} className="flex items-center justify-between py-1.5 text-sm">
              <span className="capitalize">{i.category}</span>
              <span className="tabular-nums text-[var(--muted)]">{formatCurrency(i.total)}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function TotalCard({
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
