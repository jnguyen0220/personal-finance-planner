"use client";

import Link from "next/link";
import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type Settings } from "@/lib/api";
import { Switch } from "@/components/ui/Switch";
import { MessageTemplates } from "@/features/messages/MessageTemplates";
import { Broadcast } from "@/features/messages/Broadcast";

function GeneralSettings() {
  const queryClient = useQueryClient();
  const { data: settings } = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const enabled = settings?.messaging_enabled ?? true;

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
    <section className="mb-6 card p-5">
      <div className="flex items-center justify-between gap-4">
        <div>
          <h2 className="font-semibold tracking-tight">Automated messaging</h2>
          <p className="mt-0.5 text-sm text-[var(--muted)]">
            When off, no reminder texts are sent to any tenant.
          </p>
        </div>
        <label className="flex cursor-pointer items-center gap-2">
          <span className="text-sm font-medium">{enabled ? "On" : "Off"}</span>
          <Switch checked={enabled} onChange={(v) => save.mutate({ messaging_enabled: v })} />
        </label>
      </div>

      <div className="mt-5 border-t border-[var(--border)] pt-4">
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
      </div>
    </section>
  );
}

export default function SettingsPage() {
  return (
    <main className="mx-auto max-w-3xl px-6 py-8">
      <div className="mb-6">
        <div className="flex items-center gap-2 text-sm text-[var(--muted)]">
          <Link href="/" className="transition hover:text-[var(--foreground)]">
            Portfolio
          </Link>
          <span>/</span>
          <span>Settings</span>
        </div>
        <h1 className="mt-1 text-2xl font-bold tracking-tight">Messaging settings</h1>
        <p className="mt-1 text-sm text-[var(--muted)]">
          Control automated tenant messaging and customize the wording of each reminder.
          Placeholders in <span className="font-mono text-xs">{"{braces}"}</span> are filled in
          with each tenant&apos;s details when a message is sent.
        </p>
      </div>

      <GeneralSettings />

      <Broadcast />

      <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-[var(--muted)]">
        Message templates
      </h2>
      <MessageTemplates />
    </main>
  );
}
