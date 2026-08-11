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

// Fallback for legacy categories the backend no longer defines.
const EXPENSE_FALLBACK = {
  kind: "expense" as const,
  fields: ["date", "amount", "description", "receipt"] as TxField[],
  deductible: false,
};

/// Income/expense kind, form fields, and rent-deductibility for a category.
export function configFor(categories: CategoryInfo[], name: string) {
  const found = categories.find((c) => c.name === name);
  return found
    ? { kind: found.kind, fields: found.fields, deductible: found.deductible }
    : EXPENSE_FALLBACK;
}

/// Category names that can be recorded for the current property kind.
export function categoriesFor(categories: CategoryInfo[]): string[] {
  return categories.filter((c) => c.applies).map((c) => c.name);
}
