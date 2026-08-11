"use client";

import { useState } from "react";

export function DeleteButton({
  onDelete,
  label = "Delete",
  confirmMessage = "Delete this item?",
}: {
  onDelete: () => Promise<void>;
  label?: string;
  confirmMessage?: string;
}) {
  const [busy, setBusy] = useState(false);
  return (
    <button
      type="button"
      onClick={async () => {
        if (!confirm(confirmMessage)) return;
        setBusy(true);
        try {
          await onDelete();
        } finally {
          setBusy(false);
        }
      }}
      disabled={busy}
      aria-label={label}
      title={label}
      className="rounded-lg p-2 text-[var(--muted)] transition hover:bg-red-50 hover:text-red-600 disabled:opacity-50"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        <path d="M3 6h18" />
        <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
        <path d="m19 6-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
        <path d="M10 11v6" />
        <path d="M14 11v6" />
      </svg>
    </button>
  );
}
