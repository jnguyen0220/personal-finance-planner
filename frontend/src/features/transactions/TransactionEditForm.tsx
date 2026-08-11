"use client";

import { useState } from "react";
import { api, type CategoryInfo, type Transaction } from "@/lib/api";
import { Field } from "@/components/ui/Field";
import { Modal } from "@/components/ui/Modal";
import { configFor } from "./categories";

export function TransactionEditForm({
  transaction,
  categories,
  onClose,
  onSaved,
}: {
  transaction: Transaction;
  categories: CategoryInfo[];
  onClose: () => void;
  onSaved: () => Promise<void>;
}) {
  const cfg = configFor(categories, transaction.category);
  const [amount, setAmount] = useState(String(transaction.amount));
  const [date, setDate] = useState(transaction.date);
  const [description, setDescription] = useState(transaction.description);
  const [tenant, setTenant] = useState(transaction.tenant_name);
  const [file, setFile] = useState<File | null>(null);
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [tenantPaid, setTenantPaid] = useState(transaction.borne_by === "tenant");

  // The backend flags which categories a tenant can pay and deduct from rent.
  const showTenantPaid = cfg.deductible;

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setErr(null);
    setSaving(true);
    try {
      let receiptId = transaction.receipt_id;
      if (cfg.fields.includes("receipt") && file) {
        receiptId = (await api.uploadAttachment(file)).id;
      }
      await api.updateTransaction(transaction.id, {
        kind: cfg.kind,
        category: transaction.category,
        amount: parseFloat(amount || "0"),
        date,
        description: cfg.fields.includes("description") ? description : transaction.description,
        tenant_name: cfg.fields.includes("tenant") ? tenant : transaction.tenant_name,
        borne_by: showTenantPaid && tenantPaid ? "tenant" : "landlord",
        receipt_id: receiptId,
      });
      await onSaved();
    } catch (e) {
      setErr((e as Error).message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      title={
        <>
          Edit <span className="capitalize">{transaction.category}</span>
        </>
      }
      onClose={onClose}
      onSubmit={submit}
      error={err}
      saving={saving}
    >
      <div className="grid grid-cols-2 gap-3">
        {cfg.fields.includes("date") && (
          <Field label="Date">
            <input type="date" className="input" value={date} onChange={(e) => setDate(e.target.value)} required />
          </Field>
        )}
        {cfg.fields.includes("amount") && (
          <Field label="Amount">
            <input type="number" step="0.01" min="0" className="input" value={amount} onChange={(e) => setAmount(e.target.value)} required />
          </Field>
        )}
        {cfg.fields.includes("tenant") && (
          <Field label="Tenant">
            <input className="input" value={tenant} onChange={(e) => setTenant(e.target.value)} />
          </Field>
        )}
      </div>
      {cfg.fields.includes("description") && (
        <div className="mt-3">
          <Field label="Description">
            <input className="input" value={description} onChange={(e) => setDescription(e.target.value)} />
          </Field>
        </div>
      )}
      {cfg.fields.includes("receipt") && (
        <div className="mt-3">
          <Field label={transaction.receipt_id ? "Replace receipt" : "Receipt"}>
            <input
              type="file"
              accept="image/*,application/pdf"
              className="text-sm text-[var(--muted)] file:mr-3 file:rounded-md file:border-0 file:bg-indigo-50 file:px-3 file:py-1.5 file:text-sm file:font-medium file:text-indigo-700 hover:file:bg-indigo-100"
              onChange={(e) => setFile(e.target.files?.[0] ?? null)}
            />
          </Field>
        </div>
      )}
      {showTenantPaid && (
        <div className="mt-3">
          <label className="flex items-center gap-2 text-sm text-[var(--muted)]">
            <input
              type="checkbox"
              className="h-4 w-4"
              checked={tenantPaid}
              onChange={(e) => setTenantPaid(e.target.checked)}
            />
            Paid by tenant — deduct from rent owed
          </label>
        </div>
      )}
    </Modal>
  );
}
