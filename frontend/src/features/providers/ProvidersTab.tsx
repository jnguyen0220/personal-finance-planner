"use client";

import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  api,
  formatPhone,
  formatPhoneInput,
  optionListQueryOptions,
  type ProviderInput,
  type Tenant,
} from "@/lib/api";
import { Field } from "@/components/ui/Field";
import { DeleteButton } from "@/components/ui/DeleteButton";

export function ProvidersTab({
  propertyId,
  tenants,
}: {
  propertyId: string;
  tenants: Tenant[];
}) {
  const queryClient = useQueryClient();
  const currentTenants = useMemo(() => tenants.filter((t) => t.is_current), [tenants]);
  const { data: kinds = [] } = useQuery(optionListQueryOptions("provider_kinds"));

  // Default to the first backend-provided kind rather than a hardcoded value.
  const [kind, setKind] = useState("");
  const selectedKind = kind || kinds[0] || "";
  const [name, setName] = useState("");
  const [phone, setPhone] = useState("");
  const [homepage, setHomepage] = useState("");
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const [tenantId, setTenantId] = useState(currentTenants[0]?.id ?? "");
  const [sendMsg, setSendMsg] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);

  const { data: providers = [] } = useQuery({
    queryKey: ["providers", propertyId],
    queryFn: () => api.listProviders(propertyId),
  });

  async function addProvider(e: React.FormEvent) {
    e.preventDefault();
    setErr(null);
    setSaving(true);
    try {
      const input: ProviderInput = { kind: selectedKind, name, phone, homepage };
      await api.createProvider(propertyId, input);
      setName("");
      setPhone("");
      setHomepage("");
      await queryClient.invalidateQueries({ queryKey: ["providers", propertyId] });
    } catch (e) {
      setErr((e as Error).message);
    } finally {
      setSaving(false);
    }
  }

  async function removeProvider(id: string) {
    await api.deleteProvider(id);
    await queryClient.invalidateQueries({ queryKey: ["providers", propertyId] });
  }

  const send = useMutation({
    mutationFn: () => api.sendProviders(tenantId),
    onSuccess: async (m) => {
      setSendMsg(
        m.status === "sent"
          ? "Utility info sent to tenant."
          : `Could not send: ${m.error ?? "unknown error"}`,
      );
      await queryClient.invalidateQueries({ queryKey: ["messages", propertyId] });
    },
    onError: (e) => setSendMsg((e as Error).message),
  });

  const previewMsg = useMutation({
    mutationFn: () => api.previewProviders(propertyId),
    onSuccess: (r) => setPreview(r.body),
    onError: (e) => setPreview((e as Error).message),
  });

  return (
    <div>
      <form onSubmit={addProvider} className="mb-6 card p-5">
        {err && <p className="mb-3 text-sm text-red-700">{err}</p>}
        <div className="flex flex-wrap items-end gap-3">
          <Field label="Utility">
            <select className="input" value={selectedKind} onChange={(e) => setKind(e.target.value)}>
              {kinds.map((k) => (
                <option key={k} value={k}>
                  {k[0].toUpperCase() + k.slice(1)}
                </option>
              ))}
            </select>
          </Field>
          <Field label="Provider name">
            <input className="input" value={name} onChange={(e) => setName(e.target.value)} required />
          </Field>
          <Field label="Phone">
            <input className="input" value={phone} onChange={(e) => setPhone(formatPhoneInput(e.target.value))} />
          </Field>
          <Field label="Homepage">
            <input
              className="input w-56"
              placeholder="https://…"
              value={homepage}
              onChange={(e) => setHomepage(e.target.value)}
            />
          </Field>
          <button type="submit" disabled={saving} className="btn-primary">
            {saving ? "Saving…" : "Add provider"}
          </button>
        </div>
      </form>

      <div className="mb-6 card p-5">
        <div className="flex flex-wrap items-end gap-3">
          {currentTenants.length > 0 && (
            <Field label="Send utility info to">
              <select className="input" value={tenantId} onChange={(e) => setTenantId(e.target.value)}>
                {currentTenants.map((t) => (
                  <option key={t.id} value={t.id}>
                    {`${t.first_name} ${t.last_name}`.trim()}
                    {t.phone ? ` · ${formatPhone(t.phone)}` : " · no phone"}
                  </option>
                ))}
              </select>
            </Field>
          )}
          <button
            type="button"
            disabled={previewMsg.isPending}
            onClick={() => {
              setPreview(null);
              previewMsg.mutate();
            }}
            className="btn-secondary"
          >
            {previewMsg.isPending ? "Loading…" : "Preview message"}
          </button>
          {currentTenants.length > 0 && (
            <button
              type="button"
              disabled={send.isPending || !tenantId}
              onClick={() => {
                setSendMsg(null);
                send.mutate();
              }}
              className="btn-primary"
            >
              {send.isPending ? "Sending…" : "Send to tenant"}
            </button>
          )}
          {sendMsg && <span className="pb-2 text-xs text-[var(--muted)]">{sendMsg}</span>}
        </div>
        {preview !== null && (
          <div className="mt-4">
            <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">
              Message preview
            </p>
            <pre className="whitespace-pre-wrap rounded-lg border border-[var(--border)] bg-[var(--background)] p-3 text-sm">
              {preview}
            </pre>
          </div>
        )}
      </div>

      {providers.length === 0 ? (
        <p className="card px-4 py-8 text-center text-sm text-[var(--muted)]">No providers yet.</p>
      ) : (
        <div className="table-card">
          <div className="table-scroll">
          <table className="data-table">
            <thead>
              <tr>
                <th className="th">Utility</th>
                <th className="th">Provider</th>
                <th className="th hidden md:table-cell">Phone</th>
                <th className="th hidden lg:table-cell">Homepage</th>
                <th className="th" />
              </tr>
            </thead>
            <tbody>
              {providers.map((p) => (
                <tr key={p.id}>
                  <td className="td capitalize">{p.kind}</td>
                  <td className="td font-medium">{p.name}</td>
                  <td className="td hidden whitespace-nowrap text-[var(--muted)] md:table-cell">{p.phone ? formatPhone(p.phone) : "—"}</td>
                  <td className="td hidden max-w-[18rem] lg:table-cell">
                    {p.homepage ? (
                      <a
                        href={p.homepage}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="block truncate text-indigo-600 hover:underline"
                      >
                        {p.homepage}
                      </a>
                    ) : (
                      "—"
                    )}
                  </td>
                  <td className="td text-right">
                    <DeleteButton onDelete={() => removeProvider(p.id)} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          </div>
        </div>
      )}
    </div>
  );
}
