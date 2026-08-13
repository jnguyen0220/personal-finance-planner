"use client";

import { useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, formatPhone, type BroadcastResult } from "@/lib/api";
import { renderTemplate } from "@/lib/templates";

export function Broadcast() {
  const queryClient = useQueryClient();
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [body, setBody] = useState("");
  const [result, setResult] = useState<BroadcastResult | null>(null);

  const { data: recipients = [] } = useQuery({
    queryKey: ["broadcast-recipients"],
    queryFn: api.broadcastRecipients,
  });
  const { data: settings } = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });

  const sig = settings?.signature?.trim();
  // Mirror the backend: {signature} renders to the plain sign-off, blank when unset.
  const preview = renderTemplate(body, { signature: sig ?? "" });

  function insertSignature() {
    const el = textareaRef.current;
    const token = "{signature}";
    if (!el) {
      setBody((b) => b + token);
      return;
    }
    const start = el.selectionStart ?? body.length;
    const end = el.selectionEnd ?? body.length;
    setBody(body.slice(0, start) + token + body.slice(end));
    setResult(null);
    requestAnimationFrame(() => {
      el.focus();
      const caret = start + token.length;
      el.setSelectionRange(caret, caret);
    });
  }

  const send = useMutation({
    mutationFn: () => api.broadcast(body),
    onSuccess: async (res) => {
      setResult(res);
      setBody("");
      // Refresh any open per-property message logs.
      await queryClient.invalidateQueries({ queryKey: ["messages"] });
    },
  });

  function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!body.trim() || send.isPending) return;
    const ok = window.confirm(
      `Send this message to ${recipients.length} current tenant${
        recipients.length === 1 ? "" : "s"
      } across all properties?`,
    );
    if (ok) send.mutate();
  }

  return (
    <section className="mb-6 card p-5">
      <h2 className="font-semibold tracking-tight">Broadcast</h2>
      <p className="mt-0.5 text-sm text-[var(--muted)]">
        Send a one-off message to every current tenant across all properties. Each
        send is recorded in the property&apos;s message log. Add{" "}
        <span className="font-mono text-xs">{"{signature}"}</span> where you want your
        sign-off to appear.
      </p>

      <div className="mt-4">
        <p className="mb-1.5 text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">
          Recipients ({recipients.length})
        </p>
        {recipients.length === 0 ? (
          <p className="rounded-lg border border-[var(--border)] bg-[var(--surface-2)] px-3 py-2 text-sm text-[var(--muted)]">
            No current tenants with a phone number.
          </p>
        ) : (
          <ul className="flex flex-wrap gap-1.5">
            {recipients.map((r) => (
              <li
                key={r.id}
                className="rounded-md border border-[var(--border)] bg-[var(--surface-2)] px-2 py-1 text-xs"
                title={`${r.property_name} · ${formatPhone(r.phone)}`}
              >
                <span className="font-medium">{r.name}</span>
                <span className="text-[var(--muted)]"> · {formatPhone(r.phone)}</span>
              </li>
            ))}
          </ul>
        )}
      </div>

      <form onSubmit={submit} className="mt-4">
        <div className="mb-1.5 flex flex-wrap items-center gap-1.5">
          <span className="text-xs font-medium uppercase tracking-wide text-[var(--muted)]">
            Insert:
          </span>
          <button
            type="button"
            title="Your configured sign-off"
            onClick={insertSignature}
            className="rounded-md border border-[var(--border)] bg-[var(--surface-2)] px-2 py-0.5 font-mono text-xs text-indigo-700 transition hover:border-indigo-300 hover:bg-indigo-50"
          >
            {"{signature}"}
          </button>
        </div>
        <textarea
          ref={textareaRef}
          className="input min-h-28 w-full"
          value={body}
          onChange={(e) => {
            setBody(e.target.value);
            setResult(null);
          }}
          placeholder="Type a message to all current tenants…"
        />

        <div className="mt-3">
          <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">
            Preview
          </p>
          <p className="whitespace-pre-wrap rounded-lg border border-[var(--border)] bg-[var(--surface-2)] p-3 text-sm">
            {preview.trim() ? (
              preview
            ) : (
              <span className="text-[var(--muted)]">
                Your message preview will appear here…
              </span>
            )}
          </p>
          {!sig && body.includes("{signature}") && (
            <p className="mt-1 text-xs text-[var(--muted)]">
              No signature set — add one under Automated messaging above.
            </p>
          )}
        </div>

        {send.error && (
          <p className="mt-2 text-sm text-red-700">{(send.error as Error).message}</p>
        )}

        {result && (
          <p className="mt-2 text-sm text-emerald-700">
            Sent to {result.sent} of {result.total} tenant
            {result.total === 1 ? "" : "s"}
            {result.failed > 0 ? ` · ${result.failed} failed` : ""}.
          </p>
        )}

        <div className="mt-3 flex justify-end">
          <button
            type="submit"
            disabled={send.isPending || !body.trim() || recipients.length === 0}
            className="btn-primary"
          >
            {send.isPending ? "Sending…" : "Send broadcast"}
          </button>
        </div>
      </form>
    </section>
  );
}
