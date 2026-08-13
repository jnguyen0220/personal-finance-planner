"use client";

import { useMemo, useState } from "react";
import type { CategoryInfo, Tenant, Transaction } from "@/lib/api";
import { tenantName } from "@/lib/api";
import { categoriesFor } from "./categories";
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

  // A tab per applicable category, plus any legacy categories no longer defined.
  const categoryTabs = useMemo(() => {
    const allowed = categoriesFor(categories);
    const present = new Set(transactions.map((t) => t.category));
    const extra = Array.from(present)
      .filter((c) => !categories.some((x) => x.name === c))
      .sort();
    return [...allowed, ...extra];
  }, [transactions, categories]);

  const visibleTransactions = useMemo(
    () => (catTab === "all" ? transactions : transactions.filter((t) => t.category === catTab)),
    [transactions, catTab],
  );

  return (
    <div>
      <div className="mb-4 flex flex-wrap gap-2">
        {(["all", ...categoryTabs] as string[]).map((c) => (
          <button
            key={c}
            onClick={() => setCatTab(c)}
            className={`pill capitalize ${catTab === c ? "pill-active" : ""}`}
          >
            {c === "all" ? "All" : c}
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
          fullRent={catTab === "rent" ? fullRent : null}
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
