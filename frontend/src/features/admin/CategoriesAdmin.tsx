"use client";

import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type AdminCategory, type TxField } from "@/lib/api";
import { Field } from "@/components/ui/Field";
import { Modal } from "@/components/ui/Modal";
import { Switch } from "@/components/ui/Switch";
import { EditButton } from "@/components/ui/EditButton";
import { DeleteButton } from "@/components/ui/DeleteButton";
import { FIELD_LABELS } from "@/features/transactions/categories";

const ALL_FIELDS = Object.keys(FIELD_LABELS) as TxField[];

const EMPTY: AdminCategory = {
  id: "",
  label: "",
  parent_id: null,
  kind: "expense",
  fields: ["date", "amount", "description", "receipt"],
  deductible: false,
  applies_rental: true,
  applies_personal: true,
  selectable: true,
  counts_as_rent: false,
};

/// Manage the transaction category tree that drives the income/expense forms
/// and the tax report. Categories are stored in the database and edited here.
export function CategoriesAdmin() {
  const queryClient = useQueryClient();
  const { data: categories = [] } = useQuery({
    queryKey: ["admin-categories"],
    queryFn: api.listAdminCategories,
  });

  const [editing, setEditing] = useState<AdminCategory | null>(null);
  const [isNew, setIsNew] = useState(false);

  // Render top-level categories, each followed by its children.
  const ordered = useMemo(() => {
    const roots = categories.filter((c) => c.parent_id == null);
    return roots.flatMap((r) => [r, ...categories.filter((c) => c.parent_id === r.id)]);
  }, [categories]);

  async function invalidate() {
    await queryClient.invalidateQueries({ queryKey: ["admin-categories"] });
    await queryClient.invalidateQueries({ queryKey: ["categories"] });
  }

  async function remove(id: string) {
    try {
      await api.deleteCategory(id);
      await invalidate();
    } catch (e) {
      alert((e as Error).message);
    }
  }

  return (
    <section className="card p-5">
      <div className="mb-4 flex items-center justify-between gap-4">
        <div>
          <h2 className="font-semibold tracking-tight">Transaction categories</h2>
          <p className="mt-0.5 text-sm text-[var(--muted)]">
            The tree of income and expense types. A group (non-selectable) organizes sub-categories,
            e.g. Service → Late fee, Repair.
          </p>
        </div>
        <button
          type="button"
          className="btn-primary shrink-0"
          onClick={() => {
            setEditing({ ...EMPTY });
            setIsNew(true);
          }}
        >
          Add category
        </button>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-xs uppercase tracking-wide text-[var(--muted)]">
              <th className="py-2 pr-3 font-medium">Category</th>
              <th className="py-2 pr-3 font-medium">Kind</th>
              <th className="py-2 pr-3 font-medium">Applies to</th>
              <th className="py-2 pr-3 font-medium">Deductible</th>
              <th className="py-2" />
            </tr>
          </thead>
          <tbody>
            {ordered.map((c) => (
              <tr key={c.id} className="border-t border-[var(--border)]">
                <td className="py-2 pr-3 font-medium">
                  <span className={c.parent_id ? "pl-5 text-[var(--muted)]" : ""}>
                    {c.parent_id ? "↳ " : ""}
                    {c.label}
                  </span>
                  {!c.selectable && (
                    <span className="ml-2 badge bg-slate-100 text-xs font-normal text-slate-600">group</span>
                  )}
                </td>
                <td className="py-2 pr-3 capitalize text-[var(--muted)]">
                  {c.selectable ? c.kind : "—"}
                </td>
                <td className="py-2 pr-3 text-[var(--muted)]">{appliesLabel(c)}</td>
                <td className="py-2 pr-3 text-[var(--muted)]">{c.deductible ? "Yes" : "—"}</td>
                <td className="py-2">
                  <div className="flex justify-end gap-1">
                    <EditButton
                      onEdit={() => {
                        setEditing({ ...c, fields: [...c.fields] });
                        setIsNew(false);
                      }}
                    />
                    <DeleteButton
                      onDelete={() => remove(c.id)}
                      confirmMessage={`Delete the "${c.label}" category?`}
                    />
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {editing && (
        <CategoryEditor
          value={editing}
          isNew={isNew}
          categories={categories}
          onClose={() => setEditing(null)}
          onSaved={async () => {
            setEditing(null);
            await invalidate();
          }}
        />
      )}
    </section>
  );
}

function appliesLabel(c: AdminCategory): string {
  if (!c.selectable) return "—";
  const parts = [c.applies_rental && "Rentals", c.applies_personal && "Personal"].filter(Boolean);
  return parts.length ? parts.join(" & ") : "None";
}

function CategoryEditor({
  value,
  isNew,
  categories,
  onClose,
  onSaved,
}: {
  value: AdminCategory;
  isNew: boolean;
  categories: AdminCategory[];
  onClose: () => void;
  onSaved: () => Promise<void>;
}) {
  const [draft, setDraft] = useState<AdminCategory>(value);
  const [err, setErr] = useState<string | null>(null);

  // Only groups (non-selectable) make sense as parents; never self.
  const parentOptions = categories.filter((c) => !c.selectable && c.id !== draft.id);

  const save = useMutation({
    mutationFn: () => (isNew ? api.createCategory(draft) : api.updateCategory(value.id, draft)),
    onSuccess: onSaved,
    onError: (e) => setErr((e as Error).message),
  });

  function toggleField(field: TxField, on: boolean) {
    setDraft((d) => ({
      ...d,
      fields: on ? [...d.fields, field] : d.fields.filter((f) => f !== field),
    }));
  }

  return (
    <Modal
      title={isNew ? "Add category" : `Edit ${value.label}`}
      onClose={onClose}
      onSubmit={(e) => {
        e.preventDefault();
        setErr(null);
        save.mutate();
      }}
      error={err}
      saving={save.isPending}
      submitLabel={isNew ? "Add category" : "Save changes"}
    >
      <div className="grid grid-cols-2 gap-3">
        <Field label="Id (slug)">
          <input
            className="input"
            value={draft.id}
            onChange={(e) => setDraft({ ...draft, id: e.target.value })}
            disabled={!isNew}
            required
          />
        </Field>
        <Field label="Label">
          <input
            className="input"
            value={draft.label}
            onChange={(e) => setDraft({ ...draft, label: e.target.value })}
            required
          />
        </Field>
        <Field label="Parent group">
          <select
            className="input"
            value={draft.parent_id ?? ""}
            onChange={(e) => setDraft({ ...draft, parent_id: e.target.value || null })}
          >
            <option value="">None (top-level)</option>
            {parentOptions.map((p) => (
              <option key={p.id} value={p.id}>
                {p.label}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Kind">
          <select
            className="input"
            value={draft.kind}
            onChange={(e) => setDraft({ ...draft, kind: e.target.value as AdminCategory["kind"] })}
          >
            <option value="income">Income</option>
            <option value="expense">Expense</option>
          </select>
        </Field>
      </div>

      <div className="mt-4">
        <span className="label">Form fields</span>
        <div className="mt-1 flex flex-wrap gap-x-4 gap-y-2">
          {ALL_FIELDS.map((f) => (
            <label key={f} className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                className="h-4 w-4"
                checked={draft.fields.includes(f)}
                onChange={(e) => toggleField(f, e.target.checked)}
              />
              {FIELD_LABELS[f]}
            </label>
          ))}
        </div>
      </div>

      <div className="mt-4 space-y-3 border-t border-[var(--border)] pt-4">
        <ToggleRow
          label="Selectable"
          hint="Off makes this a grouping parent that can't be recorded against directly."
          checked={draft.selectable}
          onChange={(v) => setDraft({ ...draft, selectable: v })}
        />
        <ToggleRow
          label="Rental properties"
          hint="Available when recording for a rental."
          checked={draft.applies_rental}
          onChange={(v) => setDraft({ ...draft, applies_rental: v })}
        />
        <ToggleRow
          label="Personal properties"
          hint="Available when recording for a personal property."
          checked={draft.applies_personal}
          onChange={(v) => setDraft({ ...draft, applies_personal: v })}
        />
        <ToggleRow
          label="Tenant-deductible"
          hint="A tenant who pays this can deduct it from rent owed."
          checked={draft.deductible}
          onChange={(v) => setDraft({ ...draft, deductible: v })}
        />
        <ToggleRow
          label="Counts as rent"
          hint="Income of this type counts toward rent paid (used for outstanding balances)."
          checked={draft.counts_as_rent}
          onChange={(v) => setDraft({ ...draft, counts_as_rent: v })}
        />
      </div>
    </Modal>
  );
}

function ToggleRow({
  label,
  hint,
  checked,
  onChange,
}: {
  label: string;
  hint: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex cursor-pointer items-center justify-between gap-4">
      <span>
        <span className="text-sm font-medium">{label}</span>
        <span className="block text-xs text-[var(--muted)]">{hint}</span>
      </span>
      <Switch checked={checked} onChange={onChange} size="sm" />
    </label>
  );
}
