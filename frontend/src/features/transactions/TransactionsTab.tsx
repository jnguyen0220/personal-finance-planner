"use client";

import { useMemo, useState } from "react";
import type { CategoryInfo, Tenant, Transaction } from "@/lib/api";
import { tenantName } from "@/lib/api";
import { subtreeIds, topLevel } from "./categories";
import { CategoryForm } from "./CategoryForm";
import { AllTransactionsTable, CategoryTable } from "./TransactionTables";
import { TransactionEditForm } from "./TransactionEditForm";

export function TransactionsTab({
  propertyId,
  categories,
  tenants,
  transactions,
  onChange,
}: {
  propertyId: string;
  categories: CategoryInfo[];
  tenants: Tenant[];
  transactions: Transaction[];
  onChange: () => Promise<void>;
}) {
  const [catTab, setCatTab] = useState("all");
  const [editing, setEditing] = useState<Transaction | null>(null);

  const currentTenant = tenants.find((t) => t.is_current);
  const currentTenantName = currentTenant ? tenantName(currentTenant) : "";
  const fullRent = currentTenant?.active_lease?.monthly_rent ?? null;

  // A tab per applicable top-level category (grouping parents included).
  const tabNodes = useMemo(() => topLevel(categories), [categories]);

  const visibleTransactions = useMemo(() => {
    if (catTab === "all") return transactions;
    const node = categories.find((c) => c.id === catTab);
    if (!node) return [];
    const ids = subtreeIds(categories, node);
    return transactions.filter((t) => ids.has(t.category_id));
  }, [transactions, catTab, categories]);

  return (
    <div>
      <div className="mb-4 flex flex-wrap gap-2">
        <button
          key="all"
          onClick={() => setCatTab("all")}
          className={`pill ${catTab === "all" ? "pill-active" : ""}`}
        >
          All
        </button>
        {tabNodes.map((n) => (
          <button
            key={n.id}
            onClick={() => setCatTab(n.id)}
            className={`pill capitalize ${catTab === n.id ? "pill-active" : ""}`}
          >
            {n.label}
          </button>
        ))}
      </div>

      {catTab !== "all" && (
        <CategoryForm
          key={catTab}
          propertyId={propertyId}
          categories={categories}
          category={catTab}
          defaultTenant={currentTenantName}
          fullRent={fullRent}
          onChange={onChange}
        />
      )}

      {visibleTransactions.length === 0 ? (
        <p className="card px-4 py-8 text-center text-sm text-[var(--muted)]">
          {catTab === "all" ? "No transactions yet." : "No transactions in this category."}
        </p>
      ) : catTab === "all" ? (
        <AllTransactionsTable rows={visibleTransactions} onEdit={setEditing} onChange={onChange} />
      ) : (
        <CategoryTable category={catTab} categories={categories} rows={visibleTransactions} onEdit={setEditing} onChange={onChange} />
      )}

      {editing && (
        <TransactionEditForm
          transaction={editing}
          categories={categories}
          onClose={() => setEditing(null)}
          onSaved={async () => {
            setEditing(null);
            await onChange();
          }}
        />
      )}
    </div>
  );
}
