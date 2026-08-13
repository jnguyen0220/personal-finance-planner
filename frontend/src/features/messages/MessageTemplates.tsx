"use client";

import { useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type MessageTemplate } from "@/lib/api";
import { renderTemplate } from "@/lib/templates";

// Stand-in values so the operator can see how a template reads once filled in.
const SAMPLE: Record<string, string> = {
  tenant_name: "Jordan Lee",
  address: "12 Oak Street",
  city: "Springfield",
  state: "TX",
  zip: "62704",
  balance: "$1,200.00",
  year: String(new Date().getFullYear()),
  end_date: `${new Date().getFullYear()}-12-31`,
  providers: "- Electricity: City Power — 555-0100\n- Water: Metro Water — 555-0111",
  hoa: "\nHOA: Oakwood HOA — 555-0199",
};

export function MessageTemplates() {
  const { data: templates = [], isLoading, error } = useQuery({
    queryKey: ["templates"],
    queryFn: api.listTemplates,
  });
  const { data: settings } = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });

  // The real sign-off makes the preview accurate; blank when none is set.
  const sig = settings?.signature?.trim();
  const samples: Record<string, string> = {
    ...SAMPLE,
    signature: sig ?? "",
  };

  if (isLoading) {
    return <div className="card h-40 animate-pulse" />;
  }
  if (error) {
    return (
      <p className="rounded-lg border border-red-300 bg-red-50 px-4 py-2.5 text-sm text-red-700">
        {(error as Error).message}
      </p>
    );
  }

  return (
    <div className="space-y-5">
      {templates.map((t) => (
        <TemplateCard key={t.kind} template={t} samples={samples} />
      ))}
    </div>
  );
}

function TemplateCard({
  template,
  samples,
}: {
  template: MessageTemplate;
  samples: Record<string, string>;
}) {
  const queryClient = useQueryClient();
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [body, setBody] = useState(template.body);
  const [saved, setSaved] = useState(false);

  const dirty = body !== template.body;

  const save = useMutation({
    mutationFn: (next: string) => api.updateTemplate(template.kind, next),
    onSuccess: (updated) => {
      setBody(updated.body);
      setSaved(true);
      queryClient.setQueryData<MessageTemplate[]>(["templates"], (prev) =>
        prev?.map((t) => (t.kind === updated.kind ? updated : t)),
      );
      window.setTimeout(() => setSaved(false), 2000);
    },
  });

  function insertToken(token: string) {
    const el = textareaRef.current;
    const insert = `{${token}}`;
    if (!el) {
      setBody((b) => b + insert);
      return;
    }
    const start = el.selectionStart ?? body.length;
    const end = el.selectionEnd ?? body.length;
    const next = body.slice(0, start) + insert + body.slice(end);
    setBody(next);
    // Restore focus with the caret positioned after the inserted token.
    requestAnimationFrame(() => {
      el.focus();
      const caret = start + insert.length;
      el.setSelectionRange(caret, caret);
    });
  }

  return (
    <section className="card p-5">
      <div className="flex items-start justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <h3 className="font-semibold tracking-tight">{template.label}</h3>
            <span
              className={`badge ${
                template.is_custom
                  ? "bg-indigo-100 text-indigo-700"
                  : "bg-slate-100 text-slate-600"
              }`}
            >
              {template.is_custom ? "Customized" : "Default"}
            </span>
          </div>
          <p className="mt-0.5 text-sm text-[var(--muted)]">{template.description}</p>
        </div>
        {saved && <span className="badge bg-emerald-100 text-emerald-700">Saved</span>}
      </div>

      <div className="mt-4">
        <div className="mb-1.5 flex flex-wrap items-center gap-1.5">
          <span className="text-xs font-medium uppercase tracking-wide text-[var(--muted)]">
            Insert:
          </span>
          {template.placeholders.map((p) => (
            <button
              key={p.token}
              type="button"
              title={p.description}
              onClick={() => insertToken(p.token)}
              className="rounded-md border border-[var(--border)] bg-[var(--surface-2)] px-2 py-0.5 font-mono text-xs text-indigo-700 transition hover:border-indigo-300 hover:bg-indigo-50"
            >
              {`{${p.token}}`}
            </button>
          ))}
        </div>
        <textarea
          ref={textareaRef}
          className="input min-h-28 w-full font-mono text-sm leading-relaxed"
          value={body}
          onChange={(e) => setBody(e.target.value)}
        />
      </div>

      <div className="mt-3">
        <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">
          Preview
        </p>
        <p className="whitespace-pre-wrap rounded-lg border border-[var(--border)] bg-[var(--surface-2)] p-3 text-sm">
          {renderTemplate(body, samples)}
        </p>
      </div>

      {save.error && (
        <p className="mt-3 text-sm text-red-700">{(save.error as Error).message}</p>
      )}

      <div className="mt-4 flex items-center justify-end gap-3">
        <button
          type="button"
          onClick={() => save.mutate("")}
          disabled={save.isPending || !template.is_custom}
          className="link-muted disabled:cursor-not-allowed disabled:opacity-40"
        >
          Reset to default
        </button>
        <button
          type="button"
          onClick={() => save.mutate(body)}
          disabled={save.isPending || !dirty}
          className="btn-primary"
        >
          {save.isPending ? "Saving…" : "Save"}
        </button>
      </div>
    </section>
  );
}
