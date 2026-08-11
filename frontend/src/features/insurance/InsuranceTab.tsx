"use client";

import { useState } from "react";
import { api, formatCurrency, type InsurancePolicy } from "@/lib/api";
import { Field } from "@/components/ui/Field";
import { DeleteButton } from "@/components/ui/DeleteButton";

export function InsuranceTab({
  propertyId,
  policies,
  onChange,
}: {
  propertyId: string;
  policies: InsurancePolicy[];
  onChange: () => Promise<void>;
}) {
  const [provider, setProvider] = useState("");
  const [policyNumber, setPolicyNumber] = useState("");
  const [premium, setPremium] = useState("");
  const [startDate, setStartDate] = useState("");
  const [expiryDate, setExpiryDate] = useState("");
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setErr(null);
    setSaving(true);
    try {
      await api.createInsurance(propertyId, {
        provider,
        policy_number: policyNumber,
        premium: parseFloat(premium || "0"),
        start_date: startDate || null,
        expiry_date: expiryDate,
      });
      setProvider("");
      setPolicyNumber("");
      setPremium("");
      setStartDate("");
      setExpiryDate("");
      await onChange();
    } catch (e) {
      setErr((e as Error).message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div>
      <form onSubmit={submit} className="mb-6 card p-5">
        {err && <p className="mb-3 text-sm text-red-700">{err}</p>}
        <div className="flex flex-wrap items-end gap-3">
          <Field label="Provider">
            <input className="input" value={provider} onChange={(e) => setProvider(e.target.value)} required />
          </Field>
          <Field label="Policy #">
            <input className="input" value={policyNumber} onChange={(e) => setPolicyNumber(e.target.value)} />
          </Field>
          <Field label="Premium">
            <input type="number" step="0.01" min="0" className="input w-28" value={premium} onChange={(e) => setPremium(e.target.value)} />
          </Field>
          <Field label="Start">
            <input type="date" className="input" value={startDate} onChange={(e) => setStartDate(e.target.value)} />
          </Field>
          <Field label="Expiry">
            <input type="date" className="input" value={expiryDate} onChange={(e) => setExpiryDate(e.target.value)} required />
          </Field>
          <button type="submit" disabled={saving} className="btn-primary">
            {saving ? "Saving…" : "Add policy"}
          </button>
        </div>
      </form>

      {policies.length === 0 ? (
        <p className="card px-4 py-8 text-center text-sm text-[var(--muted)]">No insurance policies yet.</p>
      ) : (
        <div className="card overflow-x-auto">
          <table className="w-full min-w-[720px] text-sm">
            <thead>
              <tr className="border-b border-[var(--border)] bg-[var(--background)] text-left text-xs uppercase tracking-wide text-[var(--muted)]">
                <th className="px-4 py-3 font-medium">Provider</th>
                <th className="px-4 py-3 font-medium">Start</th>
                <th className="px-4 py-3 font-medium">Expiry</th>
                <th className="px-4 py-3 text-right font-medium">Premium</th>
                <th className="px-4 py-3 font-medium">Status</th>
                <th className="px-4 py-3" />
              </tr>
            </thead>
            <tbody>
              {[...policies]
                .sort((a, b) => b.expiry_date.localeCompare(a.expiry_date))
                .map((p) => {
                  const days = p.days_until_expiry;
                  const expired = p.status === "expired";
                  return (
                    <tr
                      key={p.id}
                      className={`border-b border-[var(--border)] last:border-0 ${
                        expired ? "bg-red-50/60" : p.status === "expiring" ? "bg-amber-50/60" : ""
                      }`}
                    >
                      <td className="px-4 py-3">
                        <span className="font-semibold">{p.provider}</span>
                        {p.policy_number && (
                          <span className="text-[var(--muted)]"> · {p.policy_number}</span>
                        )}
                      </td>
                      <td className="px-4 py-3 text-[var(--muted)]">{p.start_date ?? "—"}</td>
                      <td className="px-4 py-3">{p.expiry_date}</td>
                      <td className="px-4 py-3 text-right">
                        {p.premium > 0 ? formatCurrency(p.premium) : "—"}
                      </td>
                      <td className="px-4 py-3">
                        {expired ? (
                          <span className="font-medium text-red-700">Expired</span>
                        ) : days === 0 ? (
                          <span className="font-medium text-amber-700">Expires today</span>
                        ) : p.status === "expiring" ? (
                          <span className="font-medium text-amber-700">in {days}d</span>
                        ) : (
                          <span className="font-medium text-emerald-600">Active · in {days}d</span>
                        )}
                      </td>
                      <td className="px-4 py-3 text-right">
                        <DeleteButton confirmMessage="Delete this policy?" onDelete={async () => { await api.deleteInsurance(p.id); await onChange(); }} />
                      </td>
                    </tr>
                  );
                })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
