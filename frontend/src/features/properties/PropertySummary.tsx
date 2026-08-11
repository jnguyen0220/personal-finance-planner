import Link from "next/link";
import { formatCurrency, type CategoryTotal, type OutstandingBalance, type PropertyKind } from "@/lib/api";

/// Outlined home (personal) or building (rental) glyph for a property kind.
export function PropertyIcon({ kind }: { kind: PropertyKind }) {
  return (
    <svg
      className="h-full w-full"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {kind === "personal" ? (
        <>
          <path d="M3 10.5 12 3l9 7.5" />
          <path d="M5 9.5V21h14V9.5" />
          <path d="M9 21v-6h6v6" />
        </>
      ) : (
        <>
          <path d="M3 21h18" />
          <path d="M5 21V5a2 2 0 0 1 2-2h6a2 2 0 0 1 2 2v16" />
          <path d="M15 9h2a2 2 0 0 1 2 2v10" />
          <path d="M9 7h2" />
          <path d="M9 11h2" />
          <path d="M9 15h2" />
        </>
      )}
    </svg>
  );
}

export function BackLink({ kind }: { kind?: PropertyKind }) {
  const label =
    kind === "personal" ? "Personal" : kind === "rental" ? "Rentals" : "All properties";
  return (
    <Link href={kind ? `/?type=${kind}` : "/"} className="link-action inline-flex items-center gap-1">
      ← {label}
    </Link>
  );
}

export function Stat({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone: "green" | "red";
}) {
  const positive = tone === "green";
  return (
    <div className="card group p-5 transition-shadow hover:shadow-md">
      <p className="text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">{label}</p>
      <p
        className={`mt-3 text-[1.75rem] font-bold leading-tight tracking-tight tabular-nums ${
          positive ? "text-emerald-600" : "text-red-600"
        }`}
      >
        {value}
      </p>
    </div>
  );
}

export function OutstandingBanner({ data }: { data: OutstandingBalance }) {
  return (
    <div className="card border-amber-300/70 bg-amber-50/70 p-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <span className="badge bg-amber-200 text-amber-900">Outstanding</span>
          <span className="text-sm font-semibold text-amber-900">
            Rent owed: {formatCurrency(data.outstanding)}
          </span>
        </div>
        <span className="text-xs text-amber-800">
          {formatCurrency(data.carry_over)} carried over ·{" "}
          {formatCurrency(Math.max(data.expected - data.paid, 0))} unpaid in {data.year}
        </span>
      </div>
    </div>
  );
}

export function CategoryBreakdown({
  items,
  showIncome,
}: {
  items: CategoryTotal[];
  showIncome: boolean;
}) {
  if (items.length === 0) return null;
  const income = items.filter((i) => i.kind === "income");
  const expense = items.filter((i) => i.kind === "expense");
  return (
    <div className={`grid gap-4 ${showIncome ? "sm:grid-cols-2" : ""}`}>
      {showIncome && <BreakdownList title="Income by category" items={income} tone="green" />}
      <BreakdownList title="Expenses by category" items={expense} tone="red" />
    </div>
  );
}

function BreakdownList({
  title,
  items,
  tone,
}: {
  title: string;
  items: CategoryTotal[];
  tone: "green" | "red";
}) {
  const total = items.reduce((sum, i) => sum + i.total, 0);
  return (
    <div className="card p-4">
      <div className="mb-2 flex items-baseline justify-between">
        <h3 className="text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">{title}</h3>
        <span className={`text-sm font-bold tabular-nums ${tone === "green" ? "text-emerald-600" : "text-red-600"}`}>
          {formatCurrency(total)}
        </span>
      </div>
      {items.length === 0 ? (
        <p className="py-2 text-sm text-[var(--muted)]">None</p>
      ) : (
        <ul className="divide-y divide-[var(--border)]">
          {items.map((i) => (
            <li key={i.category} className="flex items-center justify-between py-1.5 text-sm">
              <span className="capitalize text-[var(--foreground)]">{i.category}</span>
              <span className="font-semibold tabular-nums">{formatCurrency(i.total)}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
