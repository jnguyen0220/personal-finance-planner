"use client";

import { useEffect, useMemo, useState } from "react";
import { api, formatCurrency, type CategoryInfo } from "@/lib/api";
import { Field } from "@/components/ui/Field";
import { MoneyInput } from "@/components/ui/MoneyInput";
import { configFor, leavesFor } from "./categories";

export function CategoryForm({
  propertyId,
  categories,
  category,
  defaultTenant,
  fullRent,
  onChange,
}: {
  propertyId: string;
  categories: CategoryInfo[];
  category: string;
  defaultTenant: string;
  fullRent?: number | null;
  onChange: () => Promise<void>;
}) {
  // A tab is a top-level node; its selectable leaves populate the Type picker
  // (a single leaf when the tab is itself a leaf).
  const leaves = useMemo(() => {
    const node = categories.find((c) => c.id === category);
    return node ? leavesFor(categories, node) : [];
  }, [categories, category]);
  const isGroup = leaves.length > 1 || (leaves[0]?.id !== category);

  const [leafId, setLeafId] = useState<string>(leaves[0]?.id ?? category);
  useEffect(() => {
    setLeafId(leaves[0]?.id ?? category);
  }, [category, leaves]);

  const cfg = configFor(categories, leafId);
  const leaf = categories.find((c) => c.id === leafId);
  const today = new Date().toISOString().slice(0, 10);
  const [amount, setAmount] = useState("");
  const [date, setDate] = useState(today);
  const [description, setDescription] = useState("");
  const [tenant, setTenant] = useState(defaultTenant);
  const [file, setFile] = useState<File | null>(null);
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [isFullRent, setIsFullRent] = useState(false);
  const [tenantPaid, setTenantPaid] = useState(false);

  const showFullRent = fullRent != null && fullRent > 0 && !!leaf?.counts_as_rent;
  // The backend flags which categories a tenant can pay and deduct from rent.
  const showTenantPaid = cfg.deductible;

  function toggleFullRent(checked: boolean) {
    setIsFullRent(checked);
    setAmount(checked && fullRent != null ? String(fullRent) : "");
  }

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setErr(null);
    setSaving(true);
    try {
      let receiptId: string | null = null;
      if (cfg.fields.includes("receipt") && file) {
        receiptId = (await api.uploadAttachment(file)).id;
      }
      await api.createTransaction(propertyId, {
        category_id: leafId,
        amount: parseFloat(amount || "0"),
        date,
        description: cfg.fields.includes("description") ? description : "",
        tenant_name: cfg.fields.includes("tenant") ? tenant : "",
        borne_by: showTenantPaid && tenantPaid ? "tenant" : "landlord",
        receipt_id: receiptId,
      });
      setAmount("");
      setDescription("");
      setTenant(defaultTenant);
      setFile(null);
      setIsFullRent(false);
      setTenantPaid(false);
      const input = document.getElementById("receipt-input") as HTMLInputElement | null;
      if (input) input.value = "";
      await onChange();
    } catch (e) {
      setErr((e as Error).message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <form onSubmit={submit} className="mb-6 card p-5">
      {err && <p className="mb-3 text-sm text-red-700">{err}</p>}
      <div className="flex flex-wrap items-end gap-3">
        {isGroup && (
          <Field label="Type">
            <select className="input" value={leafId} onChange={(e) => setLeafId(e.target.value)}>
              {leaves.map((l) => (
                <option key={l.id} value={l.id}>
                  {l.label}
                </option>
              ))}
            </select>
          </Field>
        )}
        {cfg.fields.includes("date") && (
          <Field label="Date">
            <input type="date" className="input" value={date} onChange={(e) => setDate(e.target.value)} required />
          </Field>
        )}
        {cfg.fields.includes("amount") && (
          <Field label="Amount">
            <MoneyInput
              className="input w-28"
              value={amount}
              onChange={(e) => {
                setAmount(e.target.value);
                if (isFullRent) setIsFullRent(false);
              }}
              required
            />
          </Field>
        )}
        {showFullRent && (
          <Field label="Full rent">
            <label className="flex h-[38px] items-center gap-2 text-sm text-[var(--muted)]">
              <input
                type="checkbox"
                className="h-4 w-4"
                checked={isFullRent}
                onChange={(e) => toggleFullRent(e.target.checked)}
              />
              {formatCurrency(fullRent!)}
            </label>
          </Field>
        )}
        {cfg.fields.includes("tenant") && (
          <Field label="Tenant">
            <input className="input" value={tenant} onChange={(e) => setTenant(e.target.value)} />
          </Field>
        )}
        {cfg.fields.includes("description") && (
          <Field label="Description">
            <input className="input" value={description} onChange={(e) => setDescription(e.target.value)} />
          </Field>
        )}
        {cfg.fields.includes("receipt") && (
          <Field label="Receipt">
            <input
              id="receipt-input"
              type="file"
              accept="image/*,application/pdf"
              className="text-sm text-[var(--muted)] file:mr-3 file:rounded-md file:border-0 file:bg-indigo-50 file:px-3 file:py-1.5 file:text-sm file:font-medium file:text-indigo-700 hover:file:bg-indigo-100"
              onChange={(e) => setFile(e.target.files?.[0] ?? null)}
            />
          </Field>
        )}
        {showTenantPaid && (
          <Field label="Paid by tenant">
            <label className="flex h-[38px] items-center gap-2 text-sm text-[var(--muted)]">
              <input
                type="checkbox"
                className="h-4 w-4"
                checked={tenantPaid}
                onChange={(e) => setTenantPaid(e.target.checked)}
              />
              Deduct from rent
            </label>
          </Field>
        )}
        <button type="submit" disabled={saving} className="btn-primary">
          {saving ? "Saving…" : "Add"}
        </button>
      </div>
    </form>
  );
}
