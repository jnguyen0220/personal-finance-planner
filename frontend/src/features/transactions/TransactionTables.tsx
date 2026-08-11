"use client";

import { api, formatCurrency, type CategoryInfo, type Transaction } from "@/lib/api";
import { DeleteButton } from "@/components/ui/DeleteButton";
import { EditButton } from "@/components/ui/EditButton";
import { configFor, FIELD_LABELS, type TxField } from "./categories";

export function txCell(field: TxField, t: Transaction) {
  switch (field) {
    case "date":
      return (
        <td key="date" className="px-4 py-3 whitespace-nowrap">
          {t.date}
        </td>
      );
    case "amount":
      return (
        <td
          key="amount"
          className={`px-4 py-3 text-right font-semibold ${t.kind === "income" ? "text-emerald-600" : "text-red-600"}`}
        >
          {t.kind === "income" ? "+" : "-"}
          {formatCurrency(t.amount)}
          {t.borne_by === "tenant" && (
            <span className="ml-2 badge bg-amber-100 font-normal text-amber-800">tenant-paid</span>
          )}
        </td>
      );
    case "tenant":
      return (
        <td key="tenant" className="px-4 py-3 text-[var(--muted)]">
          {t.tenant_name || "—"}
        </td>
      );
    case "description":
      return (
        <td key="description" className="px-4 py-3 text-[var(--muted)]">
          {t.description || "—"}
        </td>
      );
    case "receipt":
      return (
        <td key="receipt" className="px-4 py-3">
          {t.receipt_id ? (
            <a
              href={api.attachmentUrl(t.receipt_id)}
              target="_blank"
              rel="noreferrer"
              className="font-medium text-indigo-600 hover:text-indigo-500"
            >
              view
            </a>
          ) : (
            <span className="text-[var(--muted)]">—</span>
          )}
        </td>
      );
  }
}

function RowActions({
  transaction,
  onEdit,
  onChange,
}: {
  transaction: Transaction;
  onEdit: (t: Transaction) => void;
  onChange: () => Promise<void>;
}) {
  return (
    <td className="px-4 py-3 text-right">
      <div className="flex items-center justify-end gap-2">
        <EditButton onEdit={() => onEdit(transaction)} />
        <DeleteButton
          confirmMessage="Delete this transaction?"
          onDelete={async () => { await api.deleteTransaction(transaction.id); await onChange(); }}
        />
      </div>
    </td>
  );
}

export function CategoryTable({
  category,
  categories,
  rows,
  onEdit,
  onChange,
}: {
  category: string;
  categories: CategoryInfo[];
  rows: Transaction[];
  onEdit: (t: Transaction) => void;
  onChange: () => Promise<void>;
}) {
  const cfg = configFor(categories, category);
  return (
    <div className="card overflow-x-auto">
      <table className="w-full min-w-[640px] text-sm">
        <thead>
          <tr className="border-b border-[var(--border)] bg-[var(--background)] text-left text-xs uppercase tracking-wide text-[var(--muted)]">
            {cfg.fields.map((f) => (
              <th key={f} className={`px-4 py-3 font-medium ${f === "amount" ? "text-right" : ""}`}>
                {FIELD_LABELS[f]}
              </th>
            ))}
            <th className="px-4 py-3"></th>
          </tr>
        </thead>
        <tbody>
          {rows.map((t) => (
            <tr key={t.id} className="border-b border-[var(--border)] last:border-0 transition hover:bg-[var(--background)]">
              {cfg.fields.map((f) => txCell(f, t))}
              <RowActions transaction={t} onEdit={onEdit} onChange={onChange} />
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function AllTransactionsTable({
  rows,
  onEdit,
  onChange,
}: {
  rows: Transaction[];
  onEdit: (t: Transaction) => void;
  onChange: () => Promise<void>;
}) {
  return (
    <div className="card overflow-x-auto">
      <table className="w-full min-w-[640px] text-sm">
        <thead>
          <tr className="border-b border-[var(--border)] bg-[var(--background)] text-left text-xs uppercase tracking-wide text-[var(--muted)]">
            <th className="px-4 py-3 font-medium">Date</th>
            <th className="px-4 py-3 font-medium">Category</th>
            <th className="px-4 py-3 font-medium">Details</th>
            <th className="px-4 py-3 text-right font-medium">Amount</th>
            <th className="px-4 py-3 font-medium">Receipt</th>
            <th className="px-4 py-3"></th>
          </tr>
        </thead>
        <tbody>
          {rows.map((t) => (
            <tr key={t.id} className="border-b border-[var(--border)] last:border-0 transition hover:bg-[var(--background)]">
              <td className="px-4 py-3 whitespace-nowrap">{t.date}</td>
              <td className="px-4 py-3">
                <span className="badge bg-slate-100 capitalize text-slate-700">{t.category}</span>
              </td>
              <td className="px-4 py-3 text-[var(--muted)]">{t.tenant_name || t.description || "—"}</td>
              {txCell("amount", t)}
              {txCell("receipt", t)}
              <RowActions transaction={t} onEdit={onEdit} onChange={onChange} />
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
