"use client";

import { useMemo, useState, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  api,
  categoriesQueryOptions,
  formatCurrency,
  formatDateTime,
  type InboxItem,
  type Property,
} from "@/lib/api";
import { Field } from "@/components/ui/Field";
import { MoneyInput } from "@/components/ui/MoneyInput";

const today = () => new Date().toISOString().slice(0, 10);

export default function InboxReview() {
  const qc = useQueryClient();
  const {
    data: items = [],
    isLoading,
    error,
  } = useQuery({ queryKey: ["inbox"], queryFn: api.listInbox });
  const { data: overview } = useQuery({
    queryKey: ["overview", "all"],
    queryFn: () => api.overview("all"),
  });
  const properties = useMemo(
    () => (overview?.rows ?? []).map((r) => r.property),
    [overview],
  );

  const { data: pollStatus } = useQuery({
    queryKey: ["inbox", "status"],
    queryFn: api.inboxStatus,
  });

  const refresh = () => qc.invalidateQueries({ queryKey: ["inbox"] });

  const poll = useMutation({
    mutationFn: api.pollInbox,
    onSuccess: () => refresh(),
  });

  let content: ReactNode;
  if (isLoading) {
    content = <p className="text-sm text-[var(--muted)]">Loading…</p>;
  } else if (error) {
    content = <p className="text-sm text-red-600">{(error as Error).message}</p>;
  } else if (items.length === 0) {
    content = (
      <p className="card px-4 py-10 text-center text-sm text-[var(--muted)]">
        No invoices awaiting review. New attachments from Gmail appear here.
      </p>
    );
  } else {
    content = items.map((item) => (
      <InboxCard key={item.id} item={item} properties={properties} onDone={refresh} />
    ));
  }

  const lastPoll = poll.data?.last_poll ?? pollStatus?.last_poll ?? null;
  const lastChecked = lastPoll
    ? `Last checked ${formatDateTime(lastPoll)}`
    : "Not checked yet";

  const result = poll.error
    ? (poll.error as Error).message
    : poll.data
      ? poll.data.ingested > 0
        ? `Queued ${poll.data.ingested} new attachment${poll.data.ingested === 1 ? "" : "s"}.`
        : "No new invoices found."
      : "";

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-[var(--muted)]">
          {lastChecked}
          {result && (
            <span className={poll.error ? "text-red-600" : undefined}>
              {" · "}
              {result}
            </span>
          )}
        </p>
        <button
          type="button"
          onClick={() => poll.mutate()}
          disabled={poll.isPending}
          className="rounded-lg bg-indigo-600 px-3 py-1.5 text-sm font-medium text-white transition hover:bg-indigo-700 disabled:opacity-50"
        >
          {poll.isPending ? "Checking…" : "Check for new invoices"}
        </button>
      </div>
      {content}
    </div>
  );
}

/// A traffic-light indicator of how OCR fared on an invoice attachment: green
/// when a total was detected, orange when it ran without finding one, and red
/// when extraction failed. Items ingested before OCR status was tracked show a
/// neutral light, inferring success only when an amount is present.
function OcrStatusLight({
  status: rawStatus,
  amount,
}: {
  status: string | null;
  amount: number | null;
}) {
  const status = rawStatus ?? (amount != null ? "success" : "unknown");

  const variants: Record<string, { color: string; label: ReactNode }> = {
    success: {
      color: "bg-green-500",
      label:
        amount != null ? (
          <>
            OCR read a total of{" "}
            <span className="font-medium text-[var(--foreground)]">
              {formatCurrency(amount)}
            </span>{" "}
            — confirm before assigning.
          </>
        ) : (
          "OCR extracted text — confirm the amount before assigning."
        ),
    },
    no_detection: {
      color: "bg-amber-500",
      label: "OCR ran but found no amount — enter it manually.",
    },
    failed: {
      color: "bg-red-500",
      label: "OCR didn't complete — enter the amount manually.",
    },
    unknown: {
      color: "bg-gray-400",
      label: "OCR status unavailable — enter the amount manually.",
    },
  };

  const variant = variants[status] ?? variants.unknown;

  return (
    <p className="flex items-center gap-2 text-xs text-[var(--muted)]">
      <span
        className={`inline-block h-2.5 w-2.5 shrink-0 rounded-full ${variant.color}`}
        aria-hidden
      />
      <span>{variant.label}</span>
    </p>
  );
}

function InboxCard({
  item,
  properties,
  onDone,
}: {
  item: InboxItem;
  properties: Property[];
  onDone: () => void;
}) {
  const [propertyId, setPropertyId] = useState("");
  const property = properties.find((p) => p.id === propertyId);

  const { data: categories = [] } = useQuery({
    ...categoriesQueryOptions(property?.kind ?? "rental"),
    enabled: !!property,
  });
  const expenseCategories = useMemo(
    () => categories.filter((c) => c.kind === "expense" && c.selectable && c.applies),
    [categories],
  );

  const [categoryId, setCategoryId] = useState("");
  const [amount, setAmount] = useState(
    item.ocr_amount != null ? String(item.ocr_amount) : "",
  );
  const [ocrStatus, setOcrStatus] = useState(item.ocr_status);
  const [ocrAmount, setOcrAmount] = useState(item.ocr_amount);
  const [date, setDate] = useState(today());
  const [description, setDescription] = useState(item.subject);
  const [err, setErr] = useState<string | null>(null);

  const assign = useMutation({
    mutationFn: () =>
      api.assignInbox(item.id, {
        property_id: propertyId,
        category_id: categoryId,
        amount: parseFloat(amount || "0"),
        date,
        description,
      }),
    onSuccess: onDone,
    onError: (e) => setErr((e as Error).message),
  });

  const rerunOcr = useMutation({
    mutationFn: () => api.rerunOcr(item.id),
    onSuccess: (updated) => {
      setOcrStatus(updated.ocr_status);
      setOcrAmount(updated.ocr_amount);
      if (updated.ocr_amount != null) {
        setAmount(String(updated.ocr_amount));
      }
      setErr(null);
    },
    onError: (e) => setErr((e as Error).message),
  });

  const dismiss = useMutation({
    mutationFn: () => api.dismissInbox(item.id),
    onSuccess: onDone,
    onError: (e) => setErr((e as Error).message),
  });

  const busy = assign.isPending || dismiss.isPending;
  const canAssign = Boolean(propertyId && categoryId && amount && date) && !busy;

  return (
    <div className="card overflow-hidden">
      <div className="grid gap-0 md:grid-cols-2">
        <Preview item={item} />

        <div className="space-y-3 p-4">
          <div>
            <p className="truncate text-sm font-semibold" title={item.subject}>
              {item.subject || "(no subject)"}
            </p>
            <p className="truncate text-xs text-[var(--muted)]" title={item.from_addr}>
              {item.from_addr}
            </p>
          </div>

          <div className="flex items-center justify-between gap-3">
            <OcrStatusLight status={ocrStatus} amount={ocrAmount} />
            <button
              type="button"
              onClick={() => rerunOcr.mutate()}
              disabled={rerunOcr.isPending || busy || !item.attachment_id}
              className="shrink-0 rounded-lg border border-[var(--border)] px-2.5 py-1 text-xs font-medium transition hover:bg-[var(--surface-2)] disabled:opacity-50"
            >
              {rerunOcr.isPending ? "Running OCR…" : "Run OCR"}
            </button>
          </div>



          <div className="grid grid-cols-2 gap-3">
            <Field label="Property">
              <select
                className="input"
                value={propertyId}
                onChange={(e) => {
                  setPropertyId(e.target.value);
                  setCategoryId("");
                }}
              >
                <option value="">Select…</option>
                {properties.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
            </Field>

            <Field label="Category">
              <select
                className="input"
                value={categoryId}
                onChange={(e) => setCategoryId(e.target.value)}
                disabled={!property}
              >
                <option value="">Select…</option>
                {expenseCategories.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.label}
                  </option>
                ))}
              </select>
            </Field>

            <Field label="Amount">
              <MoneyInput
                className="input"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
              />
            </Field>

            <Field label="Date">
              <input
                type="date"
                className="input"
                value={date}
                onChange={(e) => setDate(e.target.value)}
              />
            </Field>
          </div>

          <Field label="Description">
            <input
              className="input"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
          </Field>

          {err && <p className="text-sm text-red-600">{err}</p>}

          <div className="flex items-center justify-end gap-2 pt-1">
            <button
              type="button"
              onClick={() => dismiss.mutate()}
              disabled={busy}
              className="rounded-lg px-3 py-1.5 text-sm font-medium text-[var(--muted)] transition hover:bg-[var(--surface-2)] disabled:opacity-50"
            >
              Dismiss
            </button>
            <button
              type="button"
              onClick={() => assign.mutate()}
              disabled={!canAssign}
              className="rounded-lg bg-indigo-600 px-3 py-1.5 text-sm font-medium text-white transition hover:bg-indigo-700 disabled:opacity-50"
            >
              {assign.isPending ? "Filing…" : "Assign & file"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function Preview({ item }: { item: InboxItem }) {
  if (!item.attachment_id) {
    return (
      <div className="flex items-center justify-center bg-[var(--surface-2)] p-6 text-sm text-[var(--muted)]">
        Attachment unavailable
      </div>
    );
  }
  const url = api.attachmentUrl(item.attachment_id);
  const type = item.attachment_type ?? "";

  if (type.startsWith("image/")) {
    return (
      <a href={url} target="_blank" rel="noreferrer" className="block bg-[var(--surface-2)]">
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img
          src={url}
          alt={item.attachment_name ?? "invoice"}
          className="max-h-96 w-full object-contain"
        />
      </a>
    );
  }
  if (type === "application/pdf") {
    return (
      <object data={url} type="application/pdf" className="h-96 w-full bg-[var(--surface-2)]">
        <a href={url} target="_blank" rel="noreferrer" className="block p-6 text-sm text-indigo-600">
          Open {item.attachment_name ?? "PDF"}
        </a>
      </object>
    );
  }
  return (
    <div className="flex items-center justify-center bg-[var(--surface-2)] p-6">
      <a href={url} target="_blank" rel="noreferrer" className="text-sm text-indigo-600">
        Open {item.attachment_name ?? "attachment"}
      </a>
    </div>
  );
}
