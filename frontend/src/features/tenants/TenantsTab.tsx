"use client";

import { useState } from "react";
import { api, formatCurrency, formatPhone, latestLease, tenantName, type OutstandingBalance, type Tenant } from "@/lib/api";
import { Field } from "@/components/ui/Field";
import { Switch } from "@/components/ui/Switch";
import { DeleteButton } from "@/components/ui/DeleteButton";
import { EditButton } from "@/components/ui/EditButton";
import { TenantEditForm } from "./TenantEditForm";

export function TenantsTab({
  propertyId,
  tenants,
  outstanding,
  onChange,
}: {
  propertyId: string;
  tenants: Tenant[];
  outstanding: OutstandingBalance | null;
  onChange: () => Promise<void>;
}) {
  const [firstName, setFirstName] = useState("");
  const [lastName, setLastName] = useState("");
  const [email, setEmail] = useState("");
  const [phone, setPhone] = useState("");
  const [isCurrent, setIsCurrent] = useState(true);
  const [licenseFile, setLicenseFile] = useState<File | null>(null);
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [editing, setEditing] = useState<Tenant | null>(null);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setErr(null);
    setSaving(true);
    try {
      let driverLicenseId: string | null = null;
      if (licenseFile) {
        driverLicenseId = (await api.uploadAttachment(licenseFile)).id;
      }
      await api.createTenant(propertyId, {
        first_name: firstName,
        last_name: lastName,
        email,
        phone,
        is_current: isCurrent,
        driver_license_id: driverLicenseId,
      });
      setFirstName("");
      setLastName("");
      setEmail("");
      setPhone("");
      setIsCurrent(true);
      setLicenseFile(null);
      const input = document.getElementById("new-tenant-license") as HTMLInputElement | null;
      if (input) input.value = "";
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
            <input className="input" value={phone} onChange={(e) => setPhone(e.target.value)} />
          </Field>
          <label className="flex items-center gap-2 pb-2 text-sm">
            <Switch checked={isCurrent} onChange={setIsCurrent} />
            Current tenant
          </label>
          <Field label="Driver's license">
            <input
              id="new-tenant-license"
              type="file"
              accept="image/*,application/pdf"
              className="text-sm text-[var(--muted)] file:mr-3 file:rounded-md file:border-0 file:bg-indigo-50 file:px-3 file:py-1.5 file:text-sm file:font-medium file:text-indigo-700 hover:file:bg-indigo-100"
              onChange={(e) => setLicenseFile(e.target.files?.[0] ?? null)}
            />
          </Field>
          <button type="submit" disabled={saving} className="btn-primary">
            {saving ? "Saving…" : "Add tenant"}
          </button>
        </div>
        <p className="mt-3 text-xs text-[var(--muted)]">
          Add lease terms and rent from the tenant&apos;s Edit dialog.
        </p>
      </form>

      {tenants.length === 0 ? (
        <p className="card px-4 py-8 text-center text-sm text-[var(--muted)]">No tenants yet.</p>
      ) : (
        <div className="table-card">
          <div className="table-scroll">
          <table className="data-table">
            <thead>
              <tr>
                <th className="th">Name</th>
                <th className="th hidden lg:table-cell">Email</th>
                <th className="th hidden md:table-cell">Phone</th>
                <th className="th text-right">Monthly rent</th>
                <th className="th hidden sm:table-cell">Lease</th>
                <th className="th"></th>
              </tr>
            </thead>
            <tbody>
              {tenants.map((t) => {
                const lease = latestLease(t.leases);
                return (
                  <tr key={t.id}>
                    <td className="td">
                      <span className="flex items-center gap-2 whitespace-nowrap font-semibold">
                        {tenantName(t)}
                        {t.is_current && (
                          <span className="badge bg-indigo-100 text-indigo-800">Current</span>
                        )}
                        {t.is_current && outstanding && outstanding.outstanding > 0.005 && (
                          <span className="badge bg-amber-100 text-amber-800">
                            Owes {formatCurrency(outstanding.outstanding)}
                          </span>
                        )}
                      </span>
                    </td>
                    <td className="td hidden max-w-[16rem] truncate text-[var(--muted)] lg:table-cell">
                      {t.email || "—"}
                    </td>
                    <td className="td hidden whitespace-nowrap text-[var(--muted)] md:table-cell">
                      {t.phone ? formatPhone(t.phone) : "—"}
                    </td>
                    <td className="td text-right whitespace-nowrap tabular-nums">
                      {lease && lease.monthly_rent > 0 ? formatCurrency(lease.monthly_rent) : "—"}
                    </td>
                    <td className="td hidden whitespace-nowrap text-[var(--muted)] sm:table-cell">
                      {lease?.start_date ? `${lease.start_date} → ${lease.end_date ?? "?"}` : "—"}
                    </td>
                    <td className="td">
                      <div className="flex items-center justify-end gap-2">
                        <EditButton label="Edit tenant" onEdit={() => setEditing(t)} />
                        <DeleteButton
                          label="Delete tenant"
                          confirmMessage="Delete this tenant?"
                          onDelete={async () => {
                            await api.deleteTenant(t.id);
                            await onChange();
                          }}
                        />
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          </div>
        </div>
      )}

      {editing && (
        <TenantEditForm
          tenant={editing}
          onClose={() => setEditing(null)}
          onChange={onChange}
        />
      )}
    </div>
  );
}
