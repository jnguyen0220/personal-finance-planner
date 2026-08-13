"use client";

import { useState } from "react";
import { api, formatCurrency, formatDayOfMonth, formatPhoneInput, tenantName, type Lease, type Tenant } from "@/lib/api";
import { Field } from "@/components/ui/Field";
import { MoneyInput } from "@/components/ui/MoneyInput";
import { Switch } from "@/components/ui/Switch";
import { DeleteButton } from "@/components/ui/DeleteButton";
import { EditButton } from "@/components/ui/EditButton";

export function TenantEditForm({
  tenant,
  onClose,
  onChange,
}: {
  tenant: Tenant;
  onClose: () => void;
  onChange: () => Promise<void>;
}) {
  const [firstName, setFirstName] = useState(tenant.first_name);
  const [lastName, setLastName] = useState(tenant.last_name);
  const [email, setEmail] = useState(tenant.email);
  const [phone, setPhone] = useState(formatPhoneInput(tenant.phone));
  const [isCurrent, setIsCurrent] = useState(tenant.is_current);
  const [notes, setNotes] = useState(tenant.notes);
  const [licenseId, setLicenseId] = useState(tenant.driver_license_id);
  const [licenseFile, setLicenseFile] = useState<File | null>(null);
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [tab, setTab] = useState<"contact" | "leases">("contact");

  async function saveTenant(e: React.FormEvent) {
    e.preventDefault();
    setErr(null);
    setSaving(true);
    try {
      let driverLicenseId = licenseId;
      if (licenseFile) {
        driverLicenseId = (await api.uploadAttachment(licenseFile)).id;
      }
      await api.updateTenant(tenant.id, {
        first_name: firstName,
        last_name: lastName,
        email,
        phone,
        is_current: isCurrent,
        notes,
        driver_license_id: driverLicenseId,
      });
      setLicenseId(driverLicenseId);
      setLicenseFile(null);
      await onChange();
    } catch (e) {
      setErr((e as Error).message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="modal-panel flex max-h-[90vh] w-full max-w-3xl flex-col overflow-hidden"
      >
        <header className="flex items-center justify-between border-b border-[var(--border)] px-6 py-4">
          <div>
            <h2 className="text-lg font-bold tracking-tight">Edit tenant</h2>
            <p className="text-sm text-[var(--muted)]">{tenantName(tenant)}</p>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="-mr-2 rounded-lg p-2 text-[var(--muted)] transition hover:bg-[var(--background)] hover:text-[var(--foreground)]"
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          </button>
        </header>

        <div className="px-6 pt-4">
          <div className="flex gap-1 border-b border-[var(--border)]">
            <button
              type="button"
              onClick={() => setTab("contact")}
              className={`tab ${tab === "contact" ? "tab-active" : ""}`}
            >
              Contact info
            </button>
            <button
              type="button"
              onClick={() => setTab("leases")}
              className={`tab ${tab === "leases" ? "tab-active" : ""}`}
            >
              Leases
            </button>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto px-6 pb-6 pt-5">
          <form
            onSubmit={saveTenant}
            className={`space-y-4 ${tab === "contact" ? "" : "hidden"}`}
          >
            {err && (
              <p className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{err}</p>
            )}
            <div className="grid grid-cols-2 gap-4">
              <Field label="First name">
                <input className="input" value={firstName} onChange={(e) => setFirstName(e.target.value)} required />
              </Field>
              <Field label="Last name">
                <input className="input" value={lastName} onChange={(e) => setLastName(e.target.value)} />
              </Field>
              <Field label="Email">
                <input className="input" value={email} onChange={(e) => setEmail(e.target.value)} />
              </Field>
              <Field label="Phone">
                <input className="input" value={phone} onChange={(e) => setPhone(formatPhoneInput(e.target.value))} />
              </Field>
              <Field label="Current tenant">
                <label className="flex h-[38px] cursor-pointer items-center gap-3">
                  <Switch checked={isCurrent} onChange={setIsCurrent} />
                  <span className="text-sm text-[var(--muted)]">
                    {isCurrent ? "Currently occupying" : "Not current"}
                  </span>
                </label>
              </Field>
            </div>
            <Field label="Notes">
              <textarea className="input" rows={2} value={notes} onChange={(e) => setNotes(e.target.value)} />
            </Field>
            <Field label="Driver's license">
              <div className="flex items-center gap-3">
                <input
                  type="file"
                  accept="image/*,application/pdf"
                  className="text-sm text-[var(--muted)] file:mr-3 file:rounded-md file:border-0 file:bg-indigo-50 file:px-3 file:py-1.5 file:text-sm file:font-medium file:text-indigo-700 hover:file:bg-indigo-100"
                  onChange={(e) => setLicenseFile(e.target.files?.[0] ?? null)}
                />
                {licenseId && !licenseFile && (
                  <a
                    href={api.attachmentUrl(licenseId)}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-sm font-medium text-indigo-600 hover:underline"
                  >
                    View current
                  </a>
                )}
              </div>
            </Field>
            <div className="flex justify-end pt-1">
              <button type="submit" disabled={saving} className="btn-primary">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M20 6 9 17l-5-5" />
                </svg>
                {saving ? "Saving…" : "Save changes"}
              </button>
            </div>
          </form>

          <div className={tab === "leases" ? "" : "hidden"}>
            <LeaseManager tenantId={tenant.id} initialLeases={tenant.leases} onChange={onChange} />
          </div>
        </div>
      </div>
    </div>
  );
}

function LeaseManager({
  tenantId,
  initialLeases,
  onChange,
}: {
  tenantId: string;
  initialLeases: Lease[];
  onChange: () => Promise<void>;
}) {
  const [leases, setLeases] = useState<Lease[]>(initialLeases);
  const [rent, setRent] = useState("");
  const [start, setStart] = useState("");
  const [end, setEnd] = useState("");
  const [payment, setPayment] = useState("");
  const [lateFee, setLateFee] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function add() {
    if (!rent && !start) return;
    setErr(null);
    setBusy(true);
    try {
      const lease = await api.createLease(tenantId, {
        monthly_rent: parseFloat(rent || "0"),
        start_date: start || null,
        end_date: end || null,
        rent_due_day: payment ? parseInt(payment, 10) : null,
        late_fee: parseFloat(lateFee || "0"),
      });
      setLeases((ls) => [lease, ...ls]);
      setRent("");
      setStart("");
      setEnd("");
      setPayment("");
      setLateFee("");
      await onChange();
    } catch (e) {
      setErr((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div>
      {err && <p className="mb-2 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{err}</p>}

      <div className="mb-4">
        <div className="grid grid-cols-3 gap-3">
          <Field label="Monthly rent">
            <MoneyInput className="input" value={rent} onChange={(e) => setRent(e.target.value)} />
          </Field>
          <Field label="Start">
            <input type="date" className="input" value={start} onChange={(e) => setStart(e.target.value)} />
          </Field>
          <Field label="End">
            <input type="date" className="input" value={end} onChange={(e) => setEnd(e.target.value)} />
          </Field>
          <Field label="Rent due day">
            <input
              type="number"
              min="1"
              max="31"
              className="input"
              value={payment}
              onChange={(e) => setPayment(e.target.value)}
            />
          </Field>
          <Field label="Late fee">
            <MoneyInput className="input" value={lateFee} onChange={(e) => setLateFee(e.target.value)} />
          </Field>
        </div>
        <div className="mt-3 flex justify-end">
          <button type="button" onClick={add} disabled={busy} className="btn-primary">
            {busy ? "Adding…" : "Add lease"}
          </button>
        </div>
      </div>

      {leases.length === 0 ? (
        <p className="rounded-lg border border-dashed border-[var(--border)] px-3 py-4 text-center text-sm text-[var(--muted)]">
          No leases yet.
        </p>
      ) : (
        <div className="overflow-x-auto rounded-lg border border-[var(--border)]">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-[var(--border)] bg-[var(--background)] text-left text-xs uppercase tracking-wide text-[var(--muted)]">
                <th className="px-3 py-2 font-medium">Monthly rent</th>
                <th className="px-3 py-2 font-medium">Start</th>
                <th className="px-3 py-2 font-medium">End</th>
                <th className="px-3 py-2 font-medium">Rent due day</th>
                <th className="px-3 py-2 font-medium">Late fee</th>
                <th className="px-3 py-2"></th>
              </tr>
            </thead>
            <tbody>
              {leases
                .slice()
                .sort((a, b) => (b.start_date ?? "").localeCompare(a.start_date ?? ""))
                .map((l) => (
                  <LeaseRow
                    key={l.id}
                    lease={l}
                    onChange={onChange}
                    onUpdated={(updated) =>
                      setLeases((ls) => ls.map((x) => (x.id === updated.id ? updated : x)))
                    }
                    onDeleted={(delId) => setLeases((ls) => ls.filter((x) => x.id !== delId))}
                  />
                ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function LeaseRow({
  lease,
  onChange,
  onUpdated,
  onDeleted,
}: {
  lease: Lease;
  onChange: () => Promise<void>;
  onUpdated: (lease: Lease) => void;
  onDeleted: (id: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [rent, setRent] = useState(String(lease.monthly_rent));
  const [start, setStart] = useState(lease.start_date ?? "");
  const [end, setEnd] = useState(lease.end_date ?? "");
  const [payment, setPayment] = useState(lease.rent_due_day?.toString() ?? "");
  const [lateFee, setLateFee] = useState(String(lease.late_fee));
  const [busy, setBusy] = useState(false);

  async function save() {
    setBusy(true);
    try {
      const updated = await api.updateLease(lease.id, {
        monthly_rent: parseFloat(rent || "0"),
        start_date: start || null,
        end_date: end || null,
        rent_due_day: payment ? parseInt(payment, 10) : null,
        late_fee: parseFloat(lateFee || "0"),
      });
      onUpdated(updated);
      setEditing(false);
      await onChange();
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    setBusy(true);
    try {
      await api.deleteLease(lease.id);
      onDeleted(lease.id);
      await onChange();
    } finally {
      setBusy(false);
    }
  }

  if (editing) {
    return (
      <tr className="border-b border-[var(--border)] last:border-0">
        <td className="px-3 py-2">
          <MoneyInput className="input w-full" value={rent} onChange={(e) => setRent(e.target.value)} />
        </td>
        <td className="px-3 py-2">
          <input type="date" className="input w-full" value={start} onChange={(e) => setStart(e.target.value)} />
        </td>
        <td className="px-3 py-2">
          <input type="date" className="input w-full" value={end} onChange={(e) => setEnd(e.target.value)} />
        </td>
        <td className="px-3 py-2">
          <input type="number" min="1" max="31" className="input w-full" value={payment} onChange={(e) => setPayment(e.target.value)} />
        </td>
        <td className="px-3 py-2">
          <MoneyInput className="input w-full" value={lateFee} onChange={(e) => setLateFee(e.target.value)} />
        </td>
        <td className="px-3 py-2">
          <div className="flex items-center justify-end gap-3">
            <button type="button" onClick={() => setEditing(false)} className="link-muted">
              Cancel
            </button>
            <button type="button" onClick={save} disabled={busy} className="link-action">
              {busy ? "Saving…" : "Save"}
            </button>
          </div>
        </td>
      </tr>
    );
  }

  return (
    <tr className="border-b border-[var(--border)] last:border-0 transition hover:bg-[var(--background)]">
      <td className="px-3 py-2 font-medium tabular-nums">
        {lease.monthly_rent > 0 ? formatCurrency(lease.monthly_rent) : "—"}
      </td>
      <td className="px-3 py-2 text-[var(--muted)]">{lease.start_date ?? "—"}</td>
      <td className="px-3 py-2 text-[var(--muted)]">
        {lease.start_date ? lease.end_date ?? "ongoing" : "—"}
      </td>
      <td className="px-3 py-2 text-[var(--muted)]">{formatDayOfMonth(lease.rent_due_day) ?? "—"}</td>
      <td className="px-3 py-2 tabular-nums text-[var(--muted)]">
        {lease.late_fee > 0 ? formatCurrency(lease.late_fee) : "—"}
      </td>
      <td className="px-3 py-2">
        <div className="flex items-center justify-end gap-2">
          <EditButton onEdit={() => setEditing(true)} />
          <DeleteButton confirmMessage="Delete this lease?" onDelete={remove} />
        </div>
      </td>
    </tr>
  );
}
