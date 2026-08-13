"use client";

import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, optionListQueryOptions, type OptionListName } from "@/lib/api";
import { DeleteButton } from "@/components/ui/DeleteButton";

/// Manage the simple dropdown lists used across the app (provider kinds).
export function OptionListsAdmin() {
  return (
    <div className="space-y-6">
      <OptionListEditor
        list="provider_kinds"
        title="Utility provider kinds"
        description="Categories shown when adding a property's utility providers."
        placeholder="e.g. sewer"
      />
    </div>
  );
}

function OptionListEditor({
  list,
  title,
  description,
  placeholder,
}: {
  list: OptionListName;
  title: string;
  description: string;
  placeholder: string;
}) {
  const queryClient = useQueryClient();
  const { data: values } = useQuery(optionListQueryOptions(list));

  const [items, setItems] = useState<string[]>([]);
  const [added, setAdded] = useState("");

  // Sync the local draft with the server list once it loads or changes.
  useEffect(() => {
    if (values) setItems(values);
  }, [values]);

  const save = useMutation({
    mutationFn: () => api.updateOptionList(list, items),
    onSuccess: (saved) => {
      queryClient.setQueryData(optionListQueryOptions(list).queryKey, saved);
      setItems(saved);
    },
  });

  const dirty = values ? JSON.stringify(items) !== JSON.stringify(values) : false;

  function addItem() {
    const value = added.trim();
    if (!value || items.some((v) => v.toLowerCase() === value.toLowerCase())) {
      setAdded("");
      return;
    }
    setItems([...items, value]);
    setAdded("");
  }

  return (
    <section className="card p-5">
      <div className="mb-3">
        <h2 className="font-semibold tracking-tight">{title}</h2>
        <p className="mt-0.5 text-sm text-[var(--muted)]">{description}</p>
      </div>

      <ul className="mb-3 space-y-2">
        {items.map((value, i) => (
          <li key={`${value}-${i}`} className="flex items-center gap-2">
            <input
              className="input flex-1 capitalize"
              value={value}
              onChange={(e) =>
                setItems(items.map((v, j) => (j === i ? e.target.value : v)))
              }
            />
            <DeleteButton
              onDelete={async () => setItems(items.filter((_, j) => j !== i))}
              confirmMessage={`Remove "${value}"?`}
            />
          </li>
        ))}
        {items.length === 0 && (
          <li className="text-sm text-[var(--muted)]">No values yet.</li>
        )}
      </ul>

      <div className="flex items-center gap-2">
        <input
          className="input flex-1"
          placeholder={placeholder}
          value={added}
          onChange={(e) => setAdded(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              addItem();
            }
          }}
        />
        <button type="button" className="btn-secondary" onClick={addItem}>
          Add
        </button>
        <button
          type="button"
          className="btn-primary"
          onClick={() => save.mutate()}
          disabled={!dirty || save.isPending}
        >
          {save.isPending ? "Saving…" : "Save"}
        </button>
      </div>
    </section>
  );
}
