"use client";

// Shared modal scaffold: overlay, card, title, error, and Cancel/Save footer.
export function Modal({
  title,
  onClose,
  onSubmit,
  error,
  saving,
  submitLabel = "Save changes",
  children,
}: {
  title: React.ReactNode;
  onClose: () => void;
  onSubmit: (e: React.FormEvent) => void;
  error?: string | null;
  saving?: boolean;
  submitLabel?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="modal-overlay" onClick={onClose}>
      <form
        onClick={(e) => e.stopPropagation()}
        onSubmit={onSubmit}
        className="modal-panel w-full max-w-lg p-6"
      >
        <h2 className="mb-4 text-lg font-bold tracking-tight">{title}</h2>
        {error && <p className="mb-3 text-sm text-red-700">{error}</p>}
        {children}
        <div className="mt-5 flex justify-end gap-3">
          <button type="button" onClick={onClose} className="btn-secondary">
            Cancel
          </button>
          <button type="submit" disabled={saving} className="btn-primary">
            {saving ? "Saving…" : submitLabel}
          </button>
        </div>
      </form>
    </div>
  );
}
