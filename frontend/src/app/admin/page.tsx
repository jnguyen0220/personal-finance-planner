"use client";

import Link from "next/link";
import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, formatPhoneInput, type Settings } from "@/lib/api";
import { Switch } from "@/components/ui/Switch";
import { MessageTemplates } from "@/features/messages/MessageTemplates";
import { Broadcast } from "@/features/messages/Broadcast";
import { CategoriesAdmin } from "@/features/admin/CategoriesAdmin";
import { OptionListsAdmin } from "@/features/admin/OptionListsAdmin";
import { StatesList } from "@/features/admin/StatesList";

const TABS = [
  { id: "general", label: "General" },
  { id: "templates", label: "Message templates" },
  { id: "categories", label: "Categories" },
  { id: "dropdowns", label: "Dropdowns" },
  { id: "states", label: "States" },
] as const;

type TabId = (typeof TABS)[number]["id"];

function SignatureSettings() {
  const queryClient = useQueryClient();
  const { data: settings } = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });

  // Local override while editing; falls back to the saved value once loaded.
  const [draft, setDraft] = useState<string | null>(null);
  const signature = draft ?? settings?.signature ?? "";
  const dirty = draft !== null && draft !== (settings?.signature ?? "");

  const save = useMutation({
    mutationFn: (input: Partial<Settings>) => api.updateSettings(input),
    onSuccess: (s) => {
      queryClient.setQueryData(["settings"], s);
      setDraft(null);
    },
  });

  return (
    <section className="card p-5">
      <label className="label" htmlFor="signature">
        Signature
      </label>
      <p className="mb-2 text-sm text-[var(--muted)]">
        Added to the end of every automated message via the{" "}
        <span className="font-mono text-xs">{"{signature}"}</span> placeholder, so tenants know
        who it&apos;s from.
      </p>
      <div className="flex items-start gap-2">
        <input
          id="signature"
          className="input flex-1"
          placeholder="e.g. Oakwood Property Management"
          value={signature}
          onChange={(e) => setDraft(e.target.value)}
        />
        <button
          type="button"
          onClick={() => save.mutate({ signature })}
          disabled={save.isPending || !dirty}
          className="btn-primary"
        >
          {save.isPending ? "Saving…" : "Save"}
        </button>
      </div>
    </section>
  );
}

function GeneralSettings() {
  const queryClient = useQueryClient();
  const { data: settings } = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const tenantEnabled = settings?.messaging_enabled ?? true;
  const propertyEnabled = settings?.property_messaging_enabled ?? true;

  const save = useMutation({
    mutationFn: (input: Partial<Settings>) => api.updateSettings(input),
    onSuccess: (s) => {
      queryClient.setQueryData(["settings"], s);
    },
  });

  return (
    <section className="card p-5 space-y-4">
      <div className="flex items-center justify-between gap-4">
        <div>
          <h2 className="font-semibold tracking-tight">Tenant reminders</h2>
          <p className="mt-0.5 text-sm text-[var(--muted)]">
            When off, no reminder texts are sent to any tenant.
          </p>
        </div>
        <label className="flex cursor-pointer items-center gap-2">
          <span className="text-sm font-medium">{tenantEnabled ? "On" : "Off"}</span>
          <Switch checked={tenantEnabled} onChange={(v) => save.mutate({ messaging_enabled: v })} />
        </label>
      </div>

      <div className="flex items-center justify-between gap-4 border-t border-[var(--border)] pt-4">
        <div>
          <h2 className="font-semibold tracking-tight">Property reminders</h2>
          <p className="mt-0.5 text-sm text-[var(--muted)]">
            When off, no lease or insurance expiry texts are sent to your contact phones.
          </p>
        </div>
        <label className="flex cursor-pointer items-center gap-2">
          <span className="text-sm font-medium">{propertyEnabled ? "On" : "Off"}</span>
          <Switch
            checked={propertyEnabled}
            onChange={(v) => save.mutate({ property_messaging_enabled: v })}
          />
        </label>
      </div>
    </section>
  );
}

function ReminderSettings() {
  const queryClient = useQueryClient();
  const { data: settings } = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });

  // Local overrides while editing; fall back to the saved values once loaded.
  const [lease, setLease] = useState<string | null>(null);
  const [insurance, setInsurance] = useState<string | null>(null);
  const leaseValue = lease ?? String(settings?.lease_notify_days ?? 30);
  const insuranceValue = insurance ?? String(settings?.insurance_notify_days ?? 30);
  const dirty =
    (lease !== null && Number(leaseValue) !== settings?.lease_notify_days) ||
    (insurance !== null && Number(insuranceValue) !== settings?.insurance_notify_days);

  const save = useMutation({
    mutationFn: (input: Partial<Settings>) => api.updateSettings(input),
    onSuccess: (s) => {
      queryClient.setQueryData(["settings"], s);
      setLease(null);
      setInsurance(null);
    },
  });

  return (
    <section className="card p-5">
      <h2 className="font-semibold tracking-tight">Reminder windows</h2>
      <p className="mt-0.5 text-sm text-[var(--muted)]">
        How many days ahead of a lease end or policy expiry to start raising notifications and
        sending reminders. Applies to every lease and policy.
      </p>
      <div className="mt-4 flex flex-wrap items-end gap-4">
        <div>
          <label className="label" htmlFor="lease_notify_days">
            Lease expiry (days)
          </label>
          <input
            id="lease_notify_days"
            type="number"
            min="0"
            className="input w-32"
            value={leaseValue}
            onChange={(e) => setLease(e.target.value)}
          />
        </div>
        <div>
          <label className="label" htmlFor="insurance_notify_days">
            Insurance expiry (days)
          </label>
          <input
            id="insurance_notify_days"
            type="number"
            min="0"
            className="input w-32"
            value={insuranceValue}
            onChange={(e) => setInsurance(e.target.value)}
          />
        </div>
        <button
          type="button"
          onClick={() =>
            save.mutate({
              lease_notify_days: Number(leaseValue) || 0,
              insurance_notify_days: Number(insuranceValue) || 0,
            })
          }
          disabled={save.isPending || !dirty}
          className="btn-primary"
        >
          {save.isPending ? "Saving…" : "Save"}
        </button>
      </div>
    </section>
  );
}

function ContactPhones() {
  const queryClient = useQueryClient();
  const { data: settings } = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });

  // Local draft while editing; falls back to the saved list once loaded.
  const [draft, setDraft] = useState<string[] | null>(null);
  const saved = settings?.contact_phones ?? [];
  const phones = draft ?? saved;
  const cleaned = phones.map((p) => p.trim()).filter(Boolean);
  const dirty = draft !== null && JSON.stringify(cleaned) !== JSON.stringify(saved);

  const save = useMutation({
    mutationFn: (input: Partial<Settings>) => api.updateSettings(input),
    onSuccess: (s) => {
      queryClient.setQueryData(["settings"], s);
      setDraft(null);
    },
  });

  return (
    <section className="card p-5">
      <h2 className="font-semibold tracking-tight">Contact phones</h2>
      <p className="mt-0.5 text-sm text-[var(--muted)]">
        These numbers receive an SMS reminder when a lease or insurance policy is nearing expiry.
      </p>
      <div className="mt-4 space-y-2">
        {phones.length === 0 && (
          <p className="text-sm text-[var(--muted)]">No contact numbers yet.</p>
        )}
        {phones.map((phone, i) => (
          <div key={i} className="flex items-center gap-2">
            <input
              className="input flex-1"
              placeholder="(555) 123-4567"
              value={phone}
              onChange={(e) =>
                setDraft(phones.map((p, idx) => (idx === i ? formatPhoneInput(e.target.value) : p)))
              }
            />
            <button
              type="button"
              onClick={() => setDraft(phones.filter((_, idx) => idx !== i))}
              className="link-muted"
            >
              Remove
            </button>
          </div>
        ))}
      </div>
      <div className="mt-3 flex items-center justify-between gap-3">
        <button type="button" onClick={() => setDraft([...phones, ""])} className="link-action">
          Add number
        </button>
        <button
          type="button"
          onClick={() => save.mutate({ contact_phones: cleaned })}
          disabled={save.isPending || !dirty}
          className="btn-primary"
        >
          {save.isPending ? "Saving…" : "Save"}
        </button>
      </div>
    </section>
  );
}

export default function AdminPage() {
  const [tab, setTab] = useState<TabId>("general");

  return (
    <main className="mx-auto max-w-3xl px-6 py-8">
      <div className="mb-6">
        <div className="flex items-center gap-2 text-sm text-[var(--muted)]">
          <Link href="/" className="transition hover:text-[var(--foreground)]">
            Portfolio
          </Link>
          <span>/</span>
          <span>Admin</span>
        </div>
        <h1 className="mt-1 text-2xl font-bold tracking-tight">Admin</h1>
        <p className="mt-1 text-sm text-[var(--muted)]">
          Manage messaging, reference data, and the dropdowns and categories used throughout the app.
        </p>
      </div>

      <div className="mb-6">
        <SignatureSettings />
      </div>

      <nav className="mb-6 flex flex-wrap gap-1 border-b border-[var(--border)]">
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTab(t.id)}
            className={`-mb-px border-b-2 px-3 py-2 text-sm font-medium transition ${
              tab === t.id
                ? "border-indigo-600 text-[var(--foreground)]"
                : "border-transparent text-[var(--muted)] hover:text-[var(--foreground)]"
            }`}
          >
            {t.label}
          </button>
        ))}
      </nav>

      {tab === "general" && (
        <div className="space-y-6">
          <GeneralSettings />
          <ReminderSettings />
          <ContactPhones />
          <Broadcast />
        </div>
      )}

      {tab === "templates" && (
        <div>
          <p className="mb-3 text-sm text-[var(--muted)]">
            Customize the wording of each automated reminder. Placeholders in{" "}
            <span className="font-mono text-xs">{"{braces}"}</span> are filled in with each
            tenant&apos;s details when a message is sent.
          </p>
          <MessageTemplates />
        </div>
      )}

      {tab === "categories" && <CategoriesAdmin />}

      {tab === "dropdowns" && <OptionListsAdmin />}

      {tab === "states" && <StatesList />}
    </main>
  );
}
