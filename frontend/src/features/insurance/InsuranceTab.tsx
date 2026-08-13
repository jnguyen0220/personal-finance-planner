"use client";

import { useState } from "react";
import { api, formatCurrency, type InsurancePolicy } from "@/lib/api";
import { Field } from "@/components/ui/Field";
import { MoneyInput } from "@/components/ui/MoneyInput";
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
            <MoneyInput className="input w-28" value={premium} onChange={(e) => setPremium(e.target.value)} />
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
        <div className="table-card">
          <div className="table-scroll">
          <table className="data-table">
            <thead>
              <tr>
                <th className="th">Provider</th>
                <th className="th hidden sm:table-cell">Start</th>
                <th className="th">Expiry</th>
                <th className="th text-right">Premium</th>
                <th className="th">Status</th>
                <th className="th" />
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
                      className={
                        expired ? "bg-red-50/60" : p.status === "expiring" ? "bg-amber-50/60" : ""
                      }
                    >
                      <td className="td">
                        <span className="font-semibold">{p.provider}</span>
                        {p.policy_number && (
                          <span className="text-[var(--muted)]"> · {p.policy_number}</span>
                        )}
                      </td>
                      <td className="td hidden text-[var(--muted)] sm:table-cell">{p.start_date ?? "—"}</td>
                      <td className="td whitespace-nowrap">{p.expiry_date}</td>
                      <td className="td text-right tabular-nums">
                        {p.premium > 0 ? formatCurrency(p.premium) : "—"}
                      </td>
                      <td className="td whitespace-nowrap">
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
                      <td className="td text-right">
                        <DeleteButton confirmMessage="Delete this policy?" onDelete={async () => { await api.deleteInsurance(p.id); await onChange(); }} />
                      </td>
                    </tr>
                  );
                })}
            </tbody>
          </table>
          </div>
        </div>
      )}
    </div>
  );
}
