import type { CategoryInfo, TxField } from "@/lib/api";

export type { TxField } from "@/lib/api";

// UI labels for the field identifiers the backend returns.
export const FIELD_LABELS: Record<TxField, string> = {
  date: "Date",
  amount: "Amount",
  tenant: "Tenant",
  description: "Description",
  receipt: "Receipt",
};

// Safe empty config for an id not in the loaded set (e.g. before the backend
// list has loaded). All real field/kind/deductibility data comes from the backend.
const MISSING_CATEGORY = {
  kind: "expense" as const,
  fields: [] as TxField[],
  deductible: false,
  label: "",
};

/// Income/expense kind, form fields, rent-deductibility and label for a category id.
export function configFor(categories: CategoryInfo[], id: string) {
  const found = categories.find((c) => c.id === id);
  return found
    ? { kind: found.kind, fields: found.fields, deductible: found.deductible, label: found.label }
    : MISSING_CATEGORY;
}

/// Human label for a category id, falling back to the id itself.
export function labelFor(categories: CategoryInfo[], id: string): string {
  return categories.find((c) => c.id === id)?.label ?? id;
}

/// Children of a parent category, in position order.
export function childrenOf(categories: CategoryInfo[], parentId: string): CategoryInfo[] {
  return categories
    .filter((c) => c.parent_id === parentId)
    .sort((a, b) => a.position - b.position);
}

/// Whether a node can be recorded for the current property: a leaf must apply;
/// a grouping parent must have at least one applicable, selectable child.
export function nodeApplies(categories: CategoryInfo[], node: CategoryInfo): boolean {
  if (node.selectable) return node.applies;
  return childrenOf(categories, node.id).some((c) => c.applies && c.selectable);
}

/// Top-level nodes to show as tabs (applicable roots), in position order.
export function topLevel(categories: CategoryInfo[]): CategoryInfo[] {
  return categories
    .filter((c) => c.parent_id == null && nodeApplies(categories, c))
    .sort((a, b) => a.position - b.position);
}

/// The selectable leaf categories usable for `node`: itself if a leaf,
/// otherwise its applicable children.
export function leavesFor(categories: CategoryInfo[], node: CategoryInfo): CategoryInfo[] {
  if (node.selectable) return [node];
  return childrenOf(categories, node.id).filter((c) => c.applies && c.selectable);
}

/// The set of leaf ids in a node's subtree, for filtering a tab's transactions.
export function subtreeIds(categories: CategoryInfo[], node: CategoryInfo): Set<string> {
  if (node.selectable) return new Set([node.id]);
  return new Set(childrenOf(categories, node.id).map((c) => c.id));
}
