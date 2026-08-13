"use client";

import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, formatPhone, type MessageStatus, type Tenant } from "@/lib/api";
import { Field } from "@/components/ui/Field";

const STATUS_BADGE: Record<MessageStatus, string> = {
  sent: "bg-green-100 text-green-700",
  queued: "bg-sky-100 text-sky-700",
  failed: "bg-red-100 text-red-700",
};

export function MessagesTab({
  propertyId,
  tenants,
}: {
  propertyId: string;
  tenants: Tenant[];
}) {
  const queryClient = useQueryClient();
  const currentTenants = useMemo(() => tenants.filter((t) => t.is_current), [tenants]);

  const [tenantId, setTenantId] = useState(currentTenants[0]?.id ?? "");
  const [body, setBody] = useState("");
  const [err, setErr] = useState<string | null>(null);

  const tenantName = useMemo(
    () => new Map(tenants.map((t) => [t.id, `${t.first_name} ${t.last_name}`.trim()])),
    [tenants],
  );
  const selected = currentTenants.find((t) => t.id === tenantId) ?? null;

  const { data: messages = [] } = useQuery({
    queryKey: ["messages", propertyId],
    queryFn: () => api.listMessages(propertyId),
  });

  const send = useMutation({
    mutationFn: () => api.sendMessage(tenantId, { body }),
    onSuccess: async () => {
      setBody("");
      setErr(null);
      await queryClient.invalidateQueries({ queryKey: ["messages", propertyId] });
    },
    onError: (e) => setErr((e as Error).message),
  });

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!tenantId || !body.trim()) return;
    send.mutate();
  }

  return (
    <div>
      <p className="mb-4 rounded-lg border border-[var(--border)] bg-[var(--background)] px-4 py-2.5 text-xs text-[var(--muted)]">
        Outstanding-balance and lease-expiry reminders are sent automatically each day.
        Use the form below for one-off messages.
      </p>

      <form onSubmit={submit} className="mb-6 card p-5">
        {err && <p className="mb-3 text-sm text-red-700">{err}</p>}
        {currentTenants.length === 0 ? (
          <p className="text-sm text-[var(--muted)]">
            Add a current tenant to send messages.
          </p>
        ) : (
          <>
            <div className="flex flex-wrap items-end gap-3">
              <Field label="To">
                <select
                  className="input"
                  value={tenantId}
                  onChange={(e) => setTenantId(e.target.value)}
                >
                  {currentTenants.map((t) => (
                    <option key={t.id} value={t.id}>
                      {`${t.first_name} ${t.last_name}`.trim()}
                      {t.phone ? ` · ${formatPhone(t.phone)}` : " · no phone"}
                    </option>
                  ))}
                </select>
              </Field>
            </div>
            <div className="mt-3">
              <Field label="Message">
                <textarea
                  className="input min-h-24 w-full"
                  value={body}
                  onChange={(e) => setBody(e.target.value)}
                  placeholder="Type a message…"
                  required
                />
              </Field>
            </div>
            {selected && !selected.phone && (
              <p className="mt-2 text-xs text-amber-700">
                {`${selected.first_name} ${selected.last_name}`.trim()} has no phone number — sending will be recorded as failed.
              </p>
            )}
            <div className="mt-3 flex justify-end">
              <button type="submit" disabled={send.isPending || !body.trim()} className="btn-primary">
                {send.isPending ? "Sending…" : "Send message"}
              </button>
            </div>
          </>
        )}
      </form>

      {messages.length === 0 ? (
        <p className="card px-4 py-8 text-center text-sm text-[var(--muted)]">No messages yet.</p>
      ) : (
        <div className="table-card">
          <div className="table-scroll">
          <table className="data-table">
            <thead>
              <tr>
                <th className="th">Sent</th>
                <th className="th">To</th>
                <th className="th hidden md:table-cell">Type</th>
                <th className="th">Message</th>
                <th className="th">Status</th>
              </tr>
            </thead>
            <tbody>
              {messages.map((m) => (
                <tr key={m.id} className="align-top">
                  <td className="td whitespace-nowrap text-[var(--muted)]">
                    {m.created_at.slice(0, 10)}
                  </td>
                  <td className="td">
                    <span className="font-medium">{tenantName.get(m.tenant_id) ?? "Tenant"}</span>
                    {m.to_phone && (
                      <span className="block text-xs text-[var(--muted)]">{formatPhone(m.to_phone)}</span>
                    )}
                  </td>
                  <td className="td hidden capitalize text-[var(--muted)] md:table-cell">{m.kind.replace(/_/g, " ")}</td>
                  <td className="td max-w-md">{m.body}</td>
                  <td className="td">
                    <span className={`badge ${STATUS_BADGE[m.status]}`}>{m.status}</span>
                    {m.error && <span className="block text-xs text-red-700">{m.error}</span>}
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
