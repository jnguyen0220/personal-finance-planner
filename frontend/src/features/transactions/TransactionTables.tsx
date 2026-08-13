"use client";

import { api, formatCurrency, type CategoryInfo, type Transaction } from "@/lib/api";
import { DeleteButton } from "@/components/ui/DeleteButton";
import { EditButton } from "@/components/ui/EditButton";
import { childrenOf, configFor, FIELD_LABELS, type TxField } from "./categories";

// Column order for grouping-parent tabs, derived from the field vocabulary.
const FIELD_ORDER = Object.keys(FIELD_LABELS) as TxField[];

// Secondary columns collapse first so tables fit narrow screens without scrolling.
const FIELD_HIDE: Partial<Record<TxField, string>> = {
  description: "hidden md:table-cell",
  receipt: "hidden sm:table-cell",
};

export function txCell(field: TxField, t: Transaction) {
  switch (field) {
    case "date":
      return (
        <td key="date" className="td whitespace-nowrap">
          {t.date}
        </td>
      );
    case "amount":
      return (
        <td
          key="amount"
          className={`td text-right font-semibold tabular-nums ${t.kind === "income" ? "text-emerald-600" : "text-red-600"}`}
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
        <td key="tenant" className="td text-[var(--muted)]">
          {t.tenant_name || "—"}
        </td>
      );
    case "description":
      return (
        <td key="description" className="td hidden text-[var(--muted)] md:table-cell">
          {t.description || "—"}
        </td>
      );
    case "receipt":
      return (
        <td key="receipt" className="td hidden sm:table-cell">
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
    <td className="td text-right">
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
  const node = categories.find((c) => c.id === category);
  // A grouping parent mixes sub-categories, so add a Type column and use the
  // union of its children's fields; a leaf uses its own fields.
  const isGroup = node ? !node.selectable : false;
  const fields = isGroup
    ? FIELD_ORDER.filter((f) => childrenOf(categories, category).some((c) => c.fields.includes(f)))
    : configFor(categories, category).fields;
  return (
    <div className="table-card">
      <div className="table-scroll">
      <table className="data-table">
        <thead>
          <tr>
            {isGroup && <th className="th">Type</th>}
            {fields.map((f) => (
              <th key={f} className={`th ${f === "amount" ? "text-right" : ""} ${FIELD_HIDE[f] ?? ""}`}>
                {FIELD_LABELS[f]}
              </th>
            ))}
            <th className="th" />
          </tr>
        </thead>
        <tbody>
          {rows.map((t) => (
            <tr key={t.id}>
              {isGroup && <td className="td capitalize text-[var(--muted)]">{t.category_label}</td>}
              {fields.map((f) => txCell(f, t))}
              <RowActions transaction={t} onEdit={onEdit} onChange={onChange} />
            </tr>
          ))}
        </tbody>
      </table>
      </div>
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
    <div className="table-card">
      <div className="table-scroll">
      <table className="data-table">
        <thead>
          <tr>
            <th className="th">Date</th>
            <th className="th">Category</th>
            <th className="th">Details</th>
            <th className="th text-right">Amount</th>
            <th className="th hidden sm:table-cell">Receipt</th>
            <th className="th" />
          </tr>
        </thead>
        <tbody>
          {rows.map((t) => (
            <tr key={t.id}>
              <td className="td whitespace-nowrap">{t.date}</td>
              <td className="td">
                <span className="badge bg-slate-100 capitalize text-slate-700">{t.category_label}</span>
              </td>
              <td className="td max-w-[14rem] truncate text-[var(--muted)]">{t.tenant_name || t.description || "—"}</td>
              {txCell("amount", t)}
              {txCell("receipt", t)}
              <RowActions transaction={t} onEdit={onEdit} onChange={onChange} />
            </tr>
          ))}
        </tbody>
      </table>
      </div>
    </div>
  );
}
