"use client";

export function EditButton({ onEdit, label = "Edit" }: { onEdit: () => void; label?: string }) {
  return (
    <button
      type="button"
      onClick={onEdit}
      aria-label={label}
      title={label}
      className="rounded-lg p-2 text-[var(--muted)] transition hover:bg-[var(--background)] hover:text-[var(--foreground)]"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        <path d="M12 20h9" />
        <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4Z" />
      </svg>
    </button>
  );
}
